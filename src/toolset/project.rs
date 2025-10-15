// Copyright (C) 2017 Christopher R. Field.
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
// http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

//! # Project Module
//!
//! This module orchestrates project state hydration by analyzing *.wxs files and
//! determining the current wix toolset installation state.
//!
//! The `Project` type is the entrypoint for all major utilities provided by the toolset module.

use super::source::WixSource;
use super::{ext::PackageCache, Toolset};
use clap::ValueEnum;
use log::debug;
use std::{collections::BTreeMap, path::PathBuf};
use sxd_xpath::{evaluate_xpath, Context, Factory, Value};

/// Wix3 XML Namespace URI
const LEGACY_NAMESPACE_URI: &str = "http://schemas.microsoft.com/wix/2006/wi";

/// Wix4+ XML Namespace URI
pub const V4_NAMESPACE_URI: &str = "http://wixtoolset.org/schemas/v4/wxs";

/// XPATH query for the root `<Wix/>` element
const WIX_ROOT_ELEMENT_XPATH: &str = "/*[local-name()='Wix']";

const WIX_PACKAGE_ROOT_ELEMENT_XPATH: &str = "//w:Package";

/// Enumerations of wix wxs schemas
#[derive(Debug, Clone, Copy, ValueEnum, PartialEq)]
pub enum WxsSchema {
    /// Wix3 is not compatible with Wix4 and must always be upgraded
    #[clap(name = "2006")]
    Legacy,
    /// Wix4, Wix5 these versions are backwards compatible and share the same namespace
    ///
    /// If the V4 namespace is detected, then a wxs format upgrade is not required
    #[clap(name = "v4")]
    V4,
    /// Unsupported wxs schema
    #[clap(skip)]
    Unsupported,
}

/// Opens a .wxs source from path and identifies the version
pub fn open_wxs_source(path: PathBuf) -> crate::Result<WixSource> {
    let source = std::fs::read_to_string(&path)?;

    // If this isn't set correctly, wix convert will **SILENTLY** fail. Throw an error so that the user can go and fix these files manually
    if source.contains("<?xml version='1.0' encoding='windows-1252'?>") {
        return Err(format!("Source file {path:?} has an xml header with encoding `windows-1252`. This must be changed to `utf-8` otherwise subsequent tooling will silently fail.").as_str().into());
    }

    debug!("Trying to parse xml document");
    let package = sxd_document::parser::parse(source.trim_start_matches('\u{FEFF}'))?;
    let document = package.as_document();

    let root = evaluate_xpath(&document, WIX_ROOT_ELEMENT_XPATH)?;
    match root {
        Value::Nodeset(ns) => {
            if let Some((default, exts)) = ns
                .document_order_first()
                .and_then(|e| e.element())
                .map(|e| (e.default_namespace_uri(), e.namespaces_in_scope()))
            {
                let exts = exts
                    .iter()
                    .filter(|e| e.prefix() != "xml")
                    .map(super::ext::WxsDependency::from)
                    .collect();

                let wix_version = match default {
                    Some(LEGACY_NAMESPACE_URI) => WxsSchema::Legacy,
                    Some(V4_NAMESPACE_URI) => WxsSchema::V4,
                    _ => WxsSchema::Unsupported,
                };

                let is_package = match default {
                    Some(ns) => {
                        let mut context = Context::new();
                        context.set_namespace("w", ns);
                        let factory = Factory::new();
                        if let Ok(Some(xpath)) = factory.build(WIX_PACKAGE_ROOT_ELEMENT_XPATH) {
                            let value = xpath.evaluate(&context, document.root())?;
                            matches!(value, Value::Nodeset(ns) if ns.size() > 0)
                        } else {
                            false
                        }
                    }
                    _ => false,
                };
                Ok(WixSource {
                    is_package,
                    wxs_schema: wix_version,
                    path,
                    exts,
                    toolset: if matches!(wix_version, WxsSchema::Legacy) {
                        Toolset::Legacy
                    } else {
                        Toolset::Modern
                    },
                })
            } else {
                Err("Corrupted .wxs file".into())
            }
        }
        _ => Err("Invalid .wxs file".into()),
    }
}

/// Context wix project related information such as the current toolset version, *.wxs files, and installed extension packages in scope
#[derive(Debug)]
pub struct Project {
    /// Current version of `wix` command
    wix_version: semver::Version,
    /// Paths to all wxs sources
    wxs_sources: BTreeMap<PathBuf, WixSource>,
    /// Extension package cache
    package_cache: PackageCache,
    /// Toolset
    toolset: Toolset,
}

impl Project {
    /// Tries to create a new WiX project context
    ///
    /// Returns an error if the modern wix toolset is not installed
    pub fn try_new(toolset: Toolset) -> crate::Result<Self> {
        let wix_version_output = toolset.wix("--version")?.output()?;

        if wix_version_output.status.success() && !wix_version_output.stdout.is_empty() {
            let output = String::from_utf8(wix_version_output.stdout)?;

            let version = semver::Version::parse(output.trim())?;

            let mut upgrade = Self {
                wix_version: version,
                wxs_sources: BTreeMap::new(),
                package_cache: PackageCache::from(toolset.clone()),
                toolset,
            };

            upgrade.load_ext_cache()?;
            Ok(upgrade)
        } else {
            Err("wix.exe could not be found from PATH. Ensure that WiX4+ is installed.".into())
        }
    }

    /// Returns the Side-by-Side (sxs) folder name
    pub fn sxs_folder_name(&self) -> String {
        format!("wix{}", self.wix_version.major)
    }

    /// Adds a *.wxs source to the upgrade context
    ///
    /// Analyzes the *.wxs file to determine if the source requires conversion
    ///
    /// Returns an error if the *.wxs file is not valid XML
    pub fn add_wxs(&mut self, source: PathBuf) -> crate::Result<()> {
        if let std::collections::btree_map::Entry::Vacant(e) =
            self.wxs_sources.entry(source.clone())
        {
            debug!("Opening and parsing wxs source file to insert into project");
            let mut wix_source = open_wxs_source(source)?;
            wix_source.toolset = self.toolset.clone();
            debug!("Inserting wix_source={wix_source:?}");
            e.insert(wix_source);
        }
        Ok(())
    }

    /// Converts all of the source files that are part of the upgrade
    ///
    /// If a target directory is provided, none of the original source files will be updated
    pub fn upgrade(&mut self, work_dir: Option<&PathBuf>) -> crate::Result<()> {
        let mut converted = BTreeMap::new();
        for (path, src) in self.wxs_sources.iter().collect::<Vec<_>>() {
            if src.can_upgrade() {
                log::debug!("Upgrading {path:?}");
                // Upgrade will not modify the original file if a work_dir is provided
                let converted_src = src.upgrade(work_dir)?;

                // Finds missing dependencies in the package cache
                converted_src.check_deps(&mut self.package_cache);

                // If target_dir is enabled, conversion will not modify the original files and will instead
                // convert and copy the files to the target_dir
                converted.insert(converted_src.path.clone(), converted_src);
            } else {
                log::debug!("Skipping upgrade for {path:?}");
                if src.is_modern() {
                    // Finds missing dependencies in the package cache
                    src.check_deps(&mut self.package_cache);
                }
            }
        }

        // Update the state of the current set of sources
        self.wxs_sources = converted;
        Ok(())
    }

    /// Restores any missing extensions
    #[inline]
    pub fn restore(&mut self, use_global: bool, work_dir: Option<&PathBuf>) -> crate::Result<()> {
        self.package_cache
            .install_missing(use_global, self.wix_version.clone(), work_dir)
    }

    /// Returns an iterator over wix sources
    #[inline]
    pub fn sources(&self) -> impl Iterator<Item = &WixSource> {
        self.wxs_sources.values()
    }

    /// Load installed ext cache
    fn load_ext_cache(&mut self) -> crate::Result<()> {
        fn build_package_cache(
            output: std::process::Output,
            cache: &mut PackageCache,
        ) -> crate::Result<()> {
            if output.status.success() {
                let std_out = String::from_utf8(output.stdout)?;

                for (package_name, version) in std_out
                    .lines()
                    // If the current wix version doesn't match the extension that is installed, it will append "(damaged)"
                    .filter(|l| !l.trim().ends_with("(damaged)"))
                    .filter_map(|l| l.split_once(' '))
                {
                    if let Ok(version) = semver::Version::parse(version) {
                        cache.add(package_name, version);
                    }
                }

                Ok(())
            } else {
                Err("Could not load installed WiX extensions".into())
            }
        }

        match self.toolset.wix("extension list")?.output() {
            Ok(wix_ext_list) => {
                build_package_cache(wix_ext_list, &mut self.package_cache)?;
            }
            Err(err) => {
                // If this returns an error check to see if a local extension cache exists
                // Docs: https://docs.firegiant.com/wix/development/wips/6184-command-line-extension-acquisition-and-cache/
                // The path is always .wix\extensions in the current directory
                if std::env::current_dir()?
                    .join(".wix")
                    .join("extensions")
                    .exists()
                {
                    log::error!("Could not list extensions {err}");
                    return Err("Listing local extensions failed".into());
                }
            }
        }

        let wix_ext_list_global = self.toolset.wix("extension list --global")?.output()?;
        build_package_cache(wix_ext_list_global, &mut self.package_cache)?;

        Ok(())
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::Project;
    use crate::toolset::{
        ext::{WellKnownExtentions, WixExtension},
        project::{open_wxs_source, WxsSchema},
        source::WixSource,
        test::{self, ok_stdout},
        ToolsetAction,
    };
    use std::{collections::BTreeSet, path::PathBuf};

    #[test]
    fn test_open_wxs() {
        let main = open_wxs_source(PathBuf::from("./tests/common/post_v4/main.wxs"))
            .expect("must be able to open wxs file");
        assert!(main.is_package);

        let fragment = open_wxs_source(PathBuf::from("./tests/common/post_v4/fragment.wxs"))
            .expect("must be able to open wxs file");
        assert!(!fragment.is_package);
    }

    #[test]
    fn test_project_create() {
        let shim = test::toolset(|a: &ToolsetAction, _: &std::process::Command| match a {
            ToolsetAction::ListExtension => ok_stdout("WixToolset.PowerShell.wixext 0.0.0"),
            ToolsetAction::ListGlobalExtension => ok_stdout("WixToolset.VisualStudio.wixext 0.0.0"),
            ToolsetAction::Version => ok_stdout("0.0.0"),
            _ => {
                unreachable!("Should only be executing version and list actions")
            }
        });

        let project = Project::try_new(shim).unwrap();
        assert!(project
            .package_cache
            .installed(&WellKnownExtentions::Powershell));
        assert!(project.package_cache.installed(&WellKnownExtentions::VS));
    }

    #[test]
    fn test_project_upgrade() {
        let (test_dir, mut project) =
            create_test_project(stringify!(test_project_upgrade), "post_v4");

        project
            .upgrade(Some(&test_dir))
            .expect("should be able to convert");

        let missing = project
            .package_cache
            .iter_missing()
            .next()
            .expect("should have a missing package");
        assert_eq!("WixToolset.UI.wixext", missing);
    }

    #[test]
    fn test_project_upgrade_extension_detection() {
        let (test_dir, mut project) = create_test_project(
            stringify!(test_project_upgrade_extension_detection),
            "well_known_exts",
        );
        project
            .upgrade(Some(&test_dir))
            .expect("should be able to convert");

        let test_wxs = test_dir.join("main.test_project_upgrade_extension_detection.wxs");
        let wxs_source = project
            .wxs_sources
            .get(&test_wxs)
            .expect("should have been added to the project");

        assert_eq!(WxsSchema::V4, wxs_source.wxs_schema);
        assert_eq!(test_wxs, wxs_source.path);
        assert!(wxs_source.toolset.is_modern());
        validate_wxs_ext(wxs_source, WellKnownExtentions::BootstrapperApplications);
        validate_wxs_ext(wxs_source, WellKnownExtentions::ComPlus);
        validate_wxs_ext(wxs_source, WellKnownExtentions::Dependency);
        validate_wxs_ext(wxs_source, WellKnownExtentions::DirectX);
        validate_wxs_ext(wxs_source, WellKnownExtentions::Firewall);
        validate_wxs_ext(wxs_source, WellKnownExtentions::Http);
        validate_wxs_ext(wxs_source, WellKnownExtentions::Iis);
        validate_wxs_ext(wxs_source, WellKnownExtentions::Msmq);
        validate_wxs_ext(wxs_source, WellKnownExtentions::Netfx);
        validate_wxs_ext(wxs_source, WellKnownExtentions::Powershell);
        validate_wxs_ext(wxs_source, WellKnownExtentions::Sql);
        validate_wxs_ext(wxs_source, WellKnownExtentions::UI);
        validate_wxs_ext(wxs_source, WellKnownExtentions::Util);
        validate_wxs_ext(wxs_source, WellKnownExtentions::VS);
    }

    pub fn create_test_project(
        test_name: &str,
        expected_wxs_name: &'static str,
    ) -> (PathBuf, Project) {
        const PACKAGES: &[&str] = &[
            "WixToolset.BootstrapperApplications.wixext/0.0.0",
            "WixToolset.ComPlus.wixext/0.0.0",
            "WixToolset.Dependency.wixext/0.0.0",
            "WixToolset.DirectX.wixext/0.0.0",
            "WixToolset.Firewall.wixext/0.0.0",
            "WixToolset.Http.wixext/0.0.0",
            "WixToolset.Iis.wixext/0.0.0",
            "WixToolset.Msmq.wixext/0.0.0",
            "WixToolset.Netfx.wixext/0.0.0",
            "WixToolset.PowerShell.wixext/0.0.0",
            "WixToolset.Sql.wixext/0.0.0",
            "WixToolset.UI.wixext/0.0.0",
            "WixToolset.Util.wixext/0.0.0",
            "WixToolset.VisualStudio.wixext/0.0.0",
        ];

        let test_dir = PathBuf::from(".test").join(test_name);
        let test_src_file_name = format!("main.{test_name}.wxs");
        let test_src = test_dir.join(test_src_file_name);

        // Define test shim to do the "conversion" which is copying over a pre-baked converted file
        let shim = test::toolset(
            move |a: &ToolsetAction, cmd: &std::process::Command| match a {
                ToolsetAction::Convert => {
                    let args = cmd.get_args();
                    let dest = args.last().expect("should be the dest");

                    std::fs::copy(
                        PathBuf::from("tests")
                            .join("common")
                            .join(&expected_wxs_name)
                            .join("main.wxs"),
                        PathBuf::from(dest),
                    )
                    .unwrap();
                    ok_stdout("")
                }
                ToolsetAction::AddGlobalExtension => {
                    let args = cmd.get_args().map(|a| a.to_string_lossy().to_string());
                    let args = BTreeSet::from_iter(args);
                    assert!(PACKAGES.iter().all(|p| args.contains(*p)));
                    ok_stdout("")
                }
                ToolsetAction::AddExtension => {
                    let args = cmd.get_args().map(|a| a.to_string_lossy().to_string());
                    let args = BTreeSet::from_iter(args);
                    assert!(PACKAGES.iter().all(|p| args.contains(*p)));
                    ok_stdout("")
                }
                ToolsetAction::ListExtension => ok_stdout(""),
                ToolsetAction::ListGlobalExtension => ok_stdout(""),
                ToolsetAction::Version => ok_stdout("0.0.0"),
                a => {
                    unreachable!("Unexpected action, tried to execute {a:?}")
                }
            },
        );

        // Prepare test directory
        if test_dir.exists() {
            std::fs::remove_dir_all(&test_dir).unwrap();
        }
        std::fs::create_dir_all(&test_dir).unwrap();
        std::fs::copy(
            PathBuf::from("tests")
                .join("common")
                .join("pre_v4")
                .join("main.wxs"),
            &test_src,
        )
        .unwrap();

        let mut project = Project::try_new(shim).unwrap();
        project
            .add_wxs(test_src)
            .expect("Must be able to add src to project");
        (test_dir, project)
    }

    pub fn validate_wxs_ext(source: &WixSource, ext: impl WixExtension) {
        assert!(source
            .exts
            .iter()
            .find(|e| e.package_name() == ext.package_name()
                && e.namespace_prefix() == ext.namespace_prefix()
                && e.namespace_uri() == ext.namespace_uri())
            .is_some());
    }
}
