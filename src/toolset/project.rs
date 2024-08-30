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

use super::ext::PackageCache;
use super::source::WixSource;
use std::{collections::BTreeMap, path::PathBuf, process::Command};
use clap::ValueEnum;
use log::debug;
use sxd_document::parser;
use sxd_xpath::{evaluate_xpath, Value};

/// Wix3 XML Namespace URI
const LEGACY_NAMESPACE_URI: &str = "http://schemas.microsoft.com/wix/2006/wi";

/// Wix4+ XML Namespace URI
pub const V4_NAMESPACE_URI: &str = "http://wixtoolset.org/schemas/v4/wxs";

/// XPATH query for the root `<Wix/>` element
const WIX_ROOT_ELEMENT_XPATH: &str = "/*[local-name()='Wix']";

/// Enumerations of wix wxs schemas
#[derive(Debug, Clone, Copy, ValueEnum)]
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

    let package = parser::parse(&source)?;

    let document = package.as_document();
    let root = evaluate_xpath(&document, WIX_ROOT_ELEMENT_XPATH).unwrap();
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

                Ok(WixSource {
                    wix_version,
                    path,
                    exts,
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
}

impl Project {
    /// Tries to create a new WiX project context
    /// 
    /// Returns an error if the modern wix toolset is not installed
    pub fn try_new() -> crate::Result<Self> {
        let wix_version_output = Command::new("wix").arg("--version").output()?;

        if wix_version_output.status.success() && !wix_version_output.stdout.is_empty() {
            let output = String::from_utf8(wix_version_output.stdout)?;

            let version = semver::Version::parse(output.trim())?;

            let mut upgrade = Self {
                wix_version: version,
                wxs_sources: BTreeMap::new(),
                package_cache: PackageCache::default(),
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
            let wix_source = open_wxs_source(source)?;
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
                let modify = work_dir.is_some();
                let converted_src = src.upgrade(modify)?;

                // Finds missing dependencies in the package cache
                converted_src.check_deps(&mut self.package_cache);

                // If target_dir is enabled, conversion will not modify the original files and will instead
                // convert and copy the files to the target_dir
                if let Some(target_dir) = work_dir.as_ref() {
                    let created = converted_src.copy_to(target_dir.to_path_buf())?;
                    converted.insert(created.path.clone(), created);
                } else {
                    converted.insert(converted_src.path.clone(), converted_src);
                }
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
    pub fn restore(&mut self, use_global: bool, work_dir: Option<&PathBuf>) -> crate::Result<()> {
        self.package_cache
            .install_missing(use_global, self.wix_version.clone(), work_dir)
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

                if log::log_enabled!(log::Level::Debug) && !output.stderr.is_empty() {
                    let std_err = String::from_utf8(output.stderr)?;
                    for line in std_err.lines() {
                        debug!("{line}");
                    }
                }
                Ok(())
            } else {
                Err("Could not load installed WiX extensions".into())
            }
        }

        let wix_ext_list = Command::new("wix").args(["extension", "list"]).output()?;
        build_package_cache(wix_ext_list, &mut self.package_cache)?;

        let wix_ext_list_global = Command::new("wix")
            .args(["extension", "list", "--global"])
            .output()?;
        build_package_cache(wix_ext_list_global, &mut self.package_cache)?;

        Ok(())
    }
}