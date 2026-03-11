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

use crate::create::InstallerKind;
use crate::toolset::ToolsetCommand;

use super::source::WixSource;
use super::{ext::PackageCache, Toolset};
use clap::ValueEnum;
use log::debug;
use std::collections::BTreeSet;
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

                // Check if this .wxs file is a "package"
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
    #[inline]
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

    /// Configures a toolset command from project state
    #[inline]
    pub fn configure_toolset_extensions(&self, toolset: &mut ToolsetCommand) -> crate::Result<()> {
        // Find all Wix extensions being used
        let ext_flags = self
            .sources()
            .flat_map(|s| s.exts.iter().map(|e| e.package_name()))
            .fold(BTreeSet::new(), |mut acc, p| {
                acc.insert(p);
                acc
            });

        // Apply all required `-ext` flags to toolset command
        for ext in ext_flags.iter() {
            debug!("Adding WiX extension flag: -ext {}", ext);
            toolset.args(["-ext", ext]);
        }

        Ok(())
    }

    /// Adds a *.wxs source to the upgrade context
    ///
    /// Analyzes the *.wxs file to determine if the source requires conversion
    ///
    /// Returns an error if the *.wxs file is not valid XML
    #[inline]
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
    /// If a target directory is provided, none of the original source files will be updated.
    /// Returns an error if multiple source files share the same file name under different
    /// directories, since the work_dir copy would cause collisions.
    #[inline]
    pub fn upgrade(&mut self, work_dir: Option<&PathBuf>) -> crate::Result<()> {
        if work_dir.is_some() {
            // Check for duplicate file names across different directories
            let mut seen: BTreeMap<std::ffi::OsString, &std::path::Path> = BTreeMap::new();
            for path in self.wxs_sources.keys() {
                if let Some(name) = path.file_name() {
                    if let Some(prev) = seen.get(name) {
                        return Err(crate::Error::Generic(format!(
                            "Multiple WiX source files share the file name '{}' \
                             (found in '{}' and '{}'). The migrate command does not support \
                             duplicate file names across different directories. Please rename \
                             one of the files before running migrate.",
                            name.to_string_lossy(),
                            prev.display(),
                            path.display(),
                        )));
                    }
                    seen.insert(name.to_os_string(), path.as_path());
                }
            }
        }

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
            .install_missing(use_global, self.wix_version.clone(), work_dir)?;
        Ok(())
    }

    /// Returns an iterator over wix sources
    #[inline]
    pub fn sources(&self) -> impl Iterator<Item = &WixSource> {
        self.wxs_sources.values()
    }

    /// Returns the number of package sources (wxs files that define a `<Package>`)
    #[inline]
    pub fn package_count(&self) -> usize {
        self.wxs_sources.values().filter(|s| s.is_package).count()
    }

    /// Determines whether the project produces a bundle (.exe) or a package (.msi)
    ///
    /// If any source uses the bootstrapper applications (bal) namespace, this is a bundle.
    /// Otherwise it defaults to an MSI package.
    #[inline]
    pub fn is_bundle(&self) -> bool {
        self.wxs_sources.values().any(|s| s.is_bundle())
    }

    /// Returns the installer kind for this project (Exe for bundles, Msi otherwise)
    #[inline]
    pub fn installer_kind(&self) -> InstallerKind {
        if self.is_bundle() {
            InstallerKind::Exe
        } else {
            InstallerKind::Msi
        }
    }

    /// Load installed ext cache
    #[inline]
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
    use assert_fs::TempDir;
    use cargo_metadata::Package;

    use super::Project;
    use crate::{
        tests::setup_project,
        toolset::{
            ext::{WellKnownExtentions, WixExtension},
            project::{open_wxs_source, WxsSchema},
            source::WixSource,
            test::{self, ok_stdout},
            Includes, ProjectProvider, Toolset, ToolsetAction,
        },
    };
    use serial_test::serial;
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
        let test_toolset = TestProject::new(stringify!(test_project_upgrade), "post_v4");
        let mut project = test_toolset.create_project(&test_toolset.package).unwrap();
        project
            .upgrade(test_toolset.work_dir().as_ref())
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
        let test_toolset = TestProject::new(
            stringify!(test_project_upgrade_extension_detection),
            "well_known_exts",
        );
        let mut project = test_toolset.create_project(&test_toolset.package).unwrap();
        project
            .upgrade(test_toolset.work_dir().as_ref())
            .expect("should be able to convert");

        let test_wxs = test_toolset
            .work_dir()
            .unwrap()
            .join("main.test_project_upgrade_extension_detection.wxs");
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

    pub fn validate_wxs_ext(source: &WixSource, ext: impl WixExtension) {
        assert!(source
            .exts
            .iter()
            .find(|e| e.package_name() == ext.package_name()
                && e.namespace_prefix() == ext.namespace_prefix()
                && e.namespace_uri() == ext.namespace_uri())
            .is_some());
    }

    pub struct TestProject {
        test_dir: TempDir,
        expected_wxs_name: &'static str,
        includes: Vec<PathBuf>,
        pub(crate) package: Package,
    }
    const MIN_MANIFEST: &str = r#"[package]
            name = "Example"
            version = "0.1.0"
            authors = ["First Last <first.last@example.com>"]
        "#;

    impl TestProject {
        /// Creates a new test toolset
        pub fn new(test_name: &str, expected_wxs_name: &'static str) -> Self {
            let project = setup_project(MIN_MANIFEST);
            let manifest = crate::manifest(Some(&project.path().join("Cargo.toml"))).unwrap();
            let package = crate::package(&manifest, None).unwrap();
            let test_dir = project.path().to_path_buf();
            let test_src_file_name = format!("main.{test_name}.wxs");
            let test_src = test_dir.join(test_src_file_name);
            std::fs::copy(
                PathBuf::from("tests")
                    .join("common")
                    .join("pre_v4")
                    .join("main.wxs"),
                &test_src,
            )
            .unwrap();
            Self {
                test_dir: project,
                expected_wxs_name,
                includes: vec![test_src],
                package,
            }
        }
    }

    impl Includes for TestProject {
        fn includes(&self) -> Option<&Vec<PathBuf>> {
            Some(&self.includes)
        }
    }

    impl ProjectProvider for TestProject {
        fn work_dir(&self) -> Option<PathBuf> {
            Some(self.test_dir.path().to_path_buf().clone())
        }

        fn toolset(&self) -> Toolset {
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

            let expected_wxs_name = self.expected_wxs_name;
            // Define test shim to do the "conversion" which is copying over a pre-baked converted file
            test::toolset(
                move |a: &ToolsetAction, cmd: &std::process::Command| match a {
                    ToolsetAction::Convert => {
                        let args = cmd.get_args();
                        let dest = args.last().expect("should be the dest");

                        std::fs::copy(
                            PathBuf::from("tests")
                                .join("common")
                                .join(expected_wxs_name)
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
            )
        }
    }

    #[test]
    fn test_project_package_count() {
        let test_toolset = TestProject::new(stringify!(test_project_package_count), "post_v4");
        let project = test_toolset.create_project(&test_toolset.package).unwrap();
        // The includes list only has the pre_v4 source (no <Package> element)
        assert_eq!(0, project.package_count());
    }

    #[test]
    fn test_project_installer_kind_msi() {
        let test_toolset = TestProject::new(stringify!(test_project_installer_ext_msi), "post_v4");
        let project = test_toolset.create_project(&test_toolset.package).unwrap();
        // post_v4 has no bal namespace, so should produce msi
        assert_eq!(crate::create::InstallerKind::Msi, project.installer_kind());
        assert!(!project.is_bundle());
    }

    #[test]
    fn test_project_installer_extension_from_well_known_exts() {
        // Open well_known_exts directly (which has bal namespace) and verify is_bundle
        let source = open_wxs_source(PathBuf::from("./tests/common/well_known_exts/main.wxs"))
            .expect("must be able to open wxs file");
        assert!(
            source.is_bundle(),
            "well_known_exts has bal namespace and should be a bundle"
        );
    }

    #[test]
    #[serial]
    fn test_upgrade_duplicate_filenames_errors() {
        let project = setup_project(MIN_MANIFEST);
        let manifest = crate::manifest(Some(&project.path().join("Cargo.toml"))).unwrap();
        let package = crate::package(&manifest, None).unwrap();
        let test_dir = project.path().to_path_buf();

        // Create two wxs files with the same name under different subdirs
        let sub_a = test_dir.join("a");
        let sub_b = test_dir.join("b");
        std::fs::create_dir_all(&sub_a).unwrap();
        std::fs::create_dir_all(&sub_b).unwrap();

        let src_a = sub_a.join("main.wxs");
        let src_b = sub_b.join("main.wxs");
        std::fs::copy(
            PathBuf::from("tests")
                .join("common")
                .join("pre_v4")
                .join("main.wxs"),
            &src_a,
        )
        .unwrap();
        std::fs::copy(
            PathBuf::from("tests")
                .join("common")
                .join("pre_v4")
                .join("main.wxs"),
            &src_b,
        )
        .unwrap();

        struct DupProvider {
            includes: Vec<PathBuf>,
            work_dir: PathBuf,
        }
        impl Includes for DupProvider {
            fn includes(&self) -> Option<&Vec<PathBuf>> {
                Some(&self.includes)
            }
        }
        impl ProjectProvider for DupProvider {
            fn work_dir(&self) -> Option<PathBuf> {
                Some(self.work_dir.clone())
            }
            fn toolset(&self) -> Toolset {
                test::toolset(|a: &ToolsetAction, _: &std::process::Command| match a {
                    ToolsetAction::ListExtension => ok_stdout(""),
                    ToolsetAction::ListGlobalExtension => ok_stdout(""),
                    ToolsetAction::Version => ok_stdout("0.0.0"),
                    _ => ok_stdout(""),
                })
            }
        }

        let provider = DupProvider {
            includes: vec![src_a, src_b],
            work_dir: test_dir.clone(),
        };
        let mut proj = provider.create_project(&package).unwrap();
        let result = proj.upgrade(Some(&test_dir));
        assert!(
            result.is_err(),
            "Should error on duplicate file names with work_dir"
        );
        let err = format!("{}", result.unwrap_err());
        assert!(
            err.contains("duplicate file names"),
            "Error message should mention duplicate file names, got: {err}"
        );
    }

    #[test]
    fn test_project_package_count_multiple() {
        // Create a project with two wxs files that both define <Package>
        let test_toolset = TestProject::new(stringify!(test_project_pkg_count_multi), "post_v4");
        let mut project = test_toolset.create_project(&test_toolset.package).unwrap();

        // Add two sources that have <Package>
        let first_pkg = open_wxs_source(PathBuf::from("./tests/common/post_v4/main.wxs"))
            .expect("must be able to open wxs file");
        assert!(first_pkg.is_package);
        let second_pkg = open_wxs_source(PathBuf::from("./tests/common/post_v4/main.wxs"))
            .expect("must be able to open wxs file");
        assert!(second_pkg.is_package);

        project
            .wxs_sources
            .insert(PathBuf::from("first_main.wxs"), first_pkg);
        project
            .wxs_sources
            .insert(PathBuf::from("second_main.wxs"), second_pkg);
        assert_eq!(2, project.package_count());
    }

    #[test]
    fn test_project_fragment_only_has_zero_packages() {
        // A project with only fragment sources should have package_count == 0
        let test_toolset = TestProject::new(stringify!(test_project_fragment_only), "post_v4");
        let mut project = test_toolset.create_project(&test_toolset.package).unwrap();

        // Replace all sources with just a fragment (no <Package>)
        project.wxs_sources.clear();
        let fragment = open_wxs_source(PathBuf::from("./tests/common/post_v4/fragment.wxs"))
            .expect("must be able to open wxs file");
        assert!(!fragment.is_package, "fragment.wxs should not be a package");
        project
            .wxs_sources
            .insert(PathBuf::from("fragment.wxs"), fragment);

        assert_eq!(0, project.package_count());
        assert!(!project.is_bundle());
        // This is the condition that triggers the "no Package or Bundle" error in create.rs
        assert_eq!(crate::create::InstallerKind::Msi, project.installer_kind());
    }

    #[test]
    fn test_project_bundle_installer_kind() {
        // A project with a bal-namespace source should return InstallerKind::Exe
        let test_toolset = TestProject::new(stringify!(test_project_bundle_kind), "post_v4");
        let mut project = test_toolset.create_project(&test_toolset.package).unwrap();

        // Replace sources with well_known_exts which has bal namespace
        project.wxs_sources.clear();
        let bundle = open_wxs_source(PathBuf::from("./tests/common/well_known_exts/main.wxs"))
            .expect("must be able to open wxs file");
        project
            .wxs_sources
            .insert(PathBuf::from("bundle.wxs"), bundle);

        assert!(project.is_bundle());
        assert_eq!(crate::create::InstallerKind::Exe, project.installer_kind());
    }
}
