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

//! The implementation for the `upgrade` is focused on upgrading a wix3 to wix4 project
//! with minimal effort on the user's part.
//!
//! # Process for upgrading
//! - wix4 must be installed, this can be validated by inspecting `wix --version` -- (This version will be required later)
//! - main.wxs and any other .wxs files need to have the encoding set to `utf-8`
//! - `wix convert` needs to be executed on all dependent *.wxs files
//! - After conversion, the root `<Wix/>` element will have been updated to reflect that v4 is in use and the wix extensions that are in use
//! - These extensions must be installed matching the current wix version, i.e. if wix 4.0.1 is installed then the 4.0.1 version of the extension must be installed, by default the latest extension is always installed

use std::{collections::BTreeMap, path::PathBuf, process::Command};

use sxd_document::parser;
use sxd_xpath::{evaluate_xpath, Value};
use wix_exts::{PackageCache, WxsExtDependency};

/// Wix3 XML Namespace URI
const WIX3_NAMESPACE_URI: &str = "http://schemas.microsoft.com/wix/2006/wi";

/// Wix4 XML Namespace
const WIX4_NAMESPACE_URI: &str = "http://wixtoolset.org/schemas/v4/wxs";

const WIX_ROOT_ELEMENT_XPATH: &str = "/*[local-name()='Wix']";

/// Enumerations of wix versions
#[derive(Debug)]
enum WixVersion {
    /// Wix3 is not compatible with Wix4 and must always be upgraded
    V3,
    /// Wix4, Wix5 these versions are backwards compatible and share the same namespace
    ///
    /// If the V4 namespace is detected, then upgrade is not required
    Modern,
    /// Unsupported wix version
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
                    .map(WxsExtDependency::from)
                    .collect();

                let wix_version = match default {
                    Some(WIX3_NAMESPACE_URI) => WixVersion::V3,
                    Some(WIX4_NAMESPACE_URI) => WixVersion::Modern,
                    _ => WixVersion::Unsupported,
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

/// Struct containing wix upgrade state and context
#[derive(Debug)]
pub struct WixUpgrade {
    /// Current version of `wix` command
    wix_version: semver::Version,
    /// Paths to all wxs sources
    wxs_sources: BTreeMap<PathBuf, WixSource>,
    /// Extension package cache
    package_cache: PackageCache,
}

impl WixUpgrade {
    /// Try to start a WiX toolset upgrade
    pub fn try_start_upgrade() -> crate::Result<Self> {
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
            Err("`wix` command from WiX toolset is not installed".into())
        }
    }

    /// Returns the SxS folder name
    pub fn sxs_folder_name(&self) -> String {
        format!("wix{}", self.wix_version.major)
    }

    /// Adds a *.wxs source to the upgrade context
    ///
    /// Analyzes the *.wxs file to determine if the source requires conversion
    ///
    /// Returns an error if the *.wxs file is not valid XML
    pub fn add_wxs_source(&mut self, source: PathBuf) -> crate::Result<()> {
        if let std::collections::btree_map::Entry::Vacant(e) =
            self.wxs_sources.entry(source.clone())
        {
            let wix_source = open_wxs_source(source)?;
            e.insert(wix_source);
        }
        Ok(())
    }

    /// Converts all of the source files that are part of the upgrade
    ///
    /// If a target directory is provided, none of the original source files will be updated
    pub fn convert(&mut self, target_dir: Option<PathBuf>) -> crate::Result<()> {
        for (path, src) in self.wxs_sources.iter() {
            if src.can_upgrade() {
                log::debug!("Upgrading {path:?}");
                src.upgrade(target_dir.is_some(), &mut self.package_cache)?;
            } else {
                log::debug!("Skipping upgrade for {path:?}");
            }
        }

        Ok(())
    }

    /// Installs all missing extensions
    pub fn install_extensions(
        &mut self,
        use_global: bool,
        work_dir: Option<PathBuf>,
    ) -> crate::Result<()> {
        self.package_cache
            .install_missing(use_global, self.wix_version.clone(), work_dir)
    }

    /// Load installed ext cache
    fn load_ext_cache(&mut self) -> crate::Result<()> {
        fn build_package_cache(
            list_output: std::process::Output,
            cache: &mut wix_exts::PackageCache,
        ) -> crate::Result<()> {
            if list_output.status.success() {
                let output = String::from_utf8(list_output.stdout)?;

                for (package_name, version) in output
                    .lines()
                    // If the current wix version doesn't match the extension that is installed, it will append "(damaged)"
                    .filter(|l| !l.trim().ends_with("(damaged)"))
                    .filter_map(|l| l.split_once(' '))
                {
                    if let Ok(version) = semver::Version::parse(version) {
                        cache.add(package_name, version);
                    }
                }
            }
            Ok(())
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

/// Struct containing information about a wxs source file
pub struct WixSource {
    /// WiX toolset version
    wix_version: WixVersion,
    /// Path to this *.wxs file
    path: PathBuf,
    /// Extensions this wix source is dependent on
    exts: Vec<WxsExtDependency>,
}

impl WixSource {
    /// Returns true if this *.wxs source can be upgraded
    pub fn can_upgrade(&self) -> bool {
        match self.wix_version {
            WixVersion::V3 => true,
            WixVersion::Modern => false,
            WixVersion::Unsupported => false,
        }
    }

    /// Upgrades the current wix source file using `wix convert` if applicable
    ///
    /// Returns an updated WixSource object if the conversion and dependent ext install is successful
    pub fn upgrade(&self, modify: bool, package_cache: &mut PackageCache) -> crate::Result<Self> {
        for ext in self
            .exts
            .iter()
            .filter(|e| package_cache.installed(*e))
            .collect::<Vec<_>>()
        {
            package_cache.add_missing(ext.package_name());
        }

        let mut convert = Command::new("wix");
        let convert = convert.arg("convert");
        let converted_path = if !modify {
            let temp = std::env::temp_dir().join(
                self.path
                    .file_name()
                    .expect("should have a file name because requires opening to create type"),
            );
            std::fs::copy(&self.path, &temp)?;
            convert.arg(&temp);
            temp
        } else {
            convert.arg(&self.path);
            self.path.clone()
        };

        let output = convert.output()?;
        if output.status.success() {
            open_wxs_source(converted_path)
        } else {
            Err("Could not convert wix source".into())
        }
    }
}

impl std::fmt::Debug for WixSource {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("WixSource")
            .field("wix_version", &self.wix_version)
            .field("path", &self.path)
            .finish()
    }
}

pub(crate) mod wix_exts {
    use std::{
        collections::{BTreeMap, BTreeSet},
        path::PathBuf,
        process::Command,
    };

    use semver::Version;

    /// Contains a map of locally/globally installed packages
    #[derive(Default, Debug)]
    pub struct PackageCache {
        /// Installed packages
        installed: BTreeMap<String, Version>,
        /// Packages that are indicated as missing from the package cache
        missing: BTreeSet<String>,
    }

    impl PackageCache {
        /// Add an installed package to the cache
        pub fn add(&mut self, name: impl Into<String>, version: Version) {
            self.installed.insert(name.into(), version);
        }

        /// Returns true if an ext is installed matching the
        pub fn installed(&self, ext: &impl WixExtension) -> bool {
            self.installed.contains_key(ext.package_name())
        }

        /// Add's a missing package to the package cache for later installation
        pub fn add_missing(&mut self, name: impl Into<String>) {
            self.missing.insert(name.into());
        }

        /// Installs all missing packages
        pub fn install_missing(
            &mut self,
            global_cache: bool,
            version: Version,
            work_dir: Option<PathBuf>,
        ) -> crate::Result<()> {
            let mut wix = Command::new("wix");
            {
                wix.arg("extension").arg("add");
            }

            if let Some(work_dir) = work_dir {
                wix.current_dir(work_dir);
            }

            if global_cache {
                wix.arg("--global");
            }

            for m in self.missing.iter() {
                let ext_ref = format!("{m}/{}.{}.{}", version.major, version.minor, version.patch);
                wix.arg(ext_ref);
            }

            let output = wix.output()?;
            if output.status.success() {
                Ok(())
            } else {
                Err("Could not install missing dependencies".into())
            }
        }
    }

    impl WixExtension for WxsExtDependency {
        fn package_name(&self) -> &str {
            self.as_ref().package_name()
        }

        fn namespace_prefix(&self) -> &str {
            self.as_ref().namespace_prefix()
        }

        fn namespace_uri(&self) -> &str {
            self.as_ref().namespace_uri()
        }
    }

    impl<'a> From<&sxd_document::dom::Namespace<'a>> for WxsExtDependency {
        fn from(value: &sxd_document::dom::Namespace<'a>) -> Self {
            match (value.prefix(), value.uri()) {
                (BAL_NS_PREFIX, BAL_NS_URI) => {
                    Box::new(WellKnownExtentions::BootstrapperApplications)
                }
                (COMPLUS_NS_PREFIX, COMPLUS_NS_URI) => Box::new(WellKnownExtentions::ComPlus),
                (DEPENDENCY_NS_PREFIX, DEPENDENCY_NS_URI) => {
                    Box::new(WellKnownExtentions::Dependency)
                }
                (DIRECTX_NS_PREFIX, DIRECTX_NS_URI) => Box::new(WellKnownExtentions::DirectX),
                (FIREWALL_NS_PREFIX, FIREWALL_NS_URI) => Box::new(WellKnownExtentions::Firewall),
                (HTTP_NS_PREFIX, HTTP_NS_URI) => Box::new(WellKnownExtentions::Http),
                (IIS_NS_PREFIX, IIS_NS_URI) => Box::new(WellKnownExtentions::Iis),
                (MSMQ_NS_PREFIX, MSMQ_NS_URI) => Box::new(WellKnownExtentions::Msmq),
                (NETFX_NS_PREFIX, NETFX_NS_URI) => Box::new(WellKnownExtentions::Netfx),
                (POWERSHELL_NS_PREFIX, POWERSHELL_NS_URI) => {
                    Box::new(WellKnownExtentions::Powershell)
                }
                (SQL_NS_PREFIX, SQL_NS_URI) => Box::new(WellKnownExtentions::Sql),
                (UI_NS_PREFIX, UI_NS_URI) => Box::new(WellKnownExtentions::UI),
                (UTIL_NS_PREFIX, UTIL_NS_URI) => Box::new(WellKnownExtentions::Util),
                (VS_NS_PREFIX, VS_NS_URI) => Box::new(WellKnownExtentions::VS),
                (prefix, uri) => Box::new(UnknownExtNamespace {
                    prefix: prefix.to_string(),
                    uri: uri.to_string(),
                }),
            }
        }
    }

    /// Type-alias for .wxs extension dependency
    pub type WxsExtDependency = Box<dyn WixExtension>;

    /// Trait to provide wix extension identifiers w/ use the `wix extension` command
    pub trait WixExtension {
        /// Returns the .wixext package name used to identify the extension
        fn package_name(&self) -> &str;
        /// Returns the xmlns prefix
        fn namespace_prefix(&self) -> &str;
        /// Returns the xmlns uri
        fn namespace_uri(&self) -> &str;
    }

    /// Struct containing information on an unknown extension found in the `<Wix/>` element
    pub struct UnknownExtNamespace {
        prefix: String,
        uri: String,
    }

    impl WixExtension for UnknownExtNamespace {
        fn package_name(&self) -> &str {
            ""
        }

        fn namespace_prefix(&self) -> &str {
            &self.prefix
        }

        fn namespace_uri(&self) -> &str {
            &self.uri
        }
    }

    /// Enumeration of Well-known extensions documented by the wix toolset org
    ///
    /// # Background
    /// Because XML namespaces are intended to be known ahead of time, this is an explicit enuemration of all well known extensions installable by the Wix Toolset.
    /// This enables `cargo-wix` to identify which extensions are required to be installed in order for `wix build` to succeed after a V3 project has been upgraded
    /// to a V4 project.
    ///
    /// Source: https://wixtoolset.org/docs/tools/wixext/
    #[derive(Clone, Copy)]
    pub enum WellKnownExtentions {
        /// WiX Toolset Bootstrapper Applications Extension
        /// Docs: https://wixtoolset.org/docs/schema/bal/
        BootstrapperApplications,
        /// WiX Toolset COM+ Extension
        /// Docs: https://wixtoolset.org/docs/schema/complus/
        ComPlus,
        /// WiX Toolset Dependency Extension
        /// Docs: https://wixtoolset.org/docs/schema/dependency/
        Dependency,
        /// WiX Toolset DirectX Extension
        /// Docs: https://wixtoolset.org/docs/schema/directx/
        DirectX,
        /// WiX Toolset Firewall Extension
        /// Docs: https://wixtoolset.org/docs/schema/firewall/
        Firewall,
        /// Windows Installer XML Toolset Http Extension
        /// Docs: https://wixtoolset.org/docs/schema/http/
        Http,
        /// WiX Toolset Internet Information Services Extension
        /// Docs: https://wixtoolset.org/docs/schema/iis/
        Iis,
        /// WiX Toolset MSMQ Extension
        /// Docs: https://wixtoolset.org/docs/schema/msmq/
        Msmq,
        /// WiX Toolset .NET Framework Extension
        /// Docs: https://wixtoolset.org/docs/schema/netfx/
        Netfx,
        /// WiX Toolset PowerShell Extension
        /// Docs: https://wixtoolset.org/docs/schema/powershell/
        Powershell,
        /// WiX Toolset SQL Server Extension
        /// Docs: https://wixtoolset.org/docs/schema/sql/
        Sql,
        /// WiX Toolset UI Extension
        /// Docs: https://wixtoolset.org/docs/schema/ui/
        UI,
        /// WiX Toolset Utility Extension
        /// Docs: https://wixtoolset.org/docs/schema/util/
        Util,
        /// WiX Toolset Visual Studio Extension
        /// Docs: https://wixtoolset.org/docs/schema/vs/
        VS,
    }

    impl WixExtension for WellKnownExtentions {
        fn package_name(&self) -> &str {
            match self {
                WellKnownExtentions::BootstrapperApplications => BAL_EXT,
                WellKnownExtentions::ComPlus => COMPLUS_EXT,
                WellKnownExtentions::Dependency => DEPENDENCY_EXT,
                WellKnownExtentions::DirectX => DIRECTX_EXT,
                WellKnownExtentions::Firewall => FIREWALL_EXT,
                WellKnownExtentions::Http => HTTP_EXT,
                WellKnownExtentions::Iis => IIS_EXT,
                WellKnownExtentions::Msmq => MSMQ_EXT,
                WellKnownExtentions::Netfx => NETFX_EXT,
                WellKnownExtentions::Powershell => POWERSHELL_EXT,
                WellKnownExtentions::Sql => SQL_EXT,
                WellKnownExtentions::UI => UI_EXT,
                WellKnownExtentions::Util => UTIL_EXT,
                WellKnownExtentions::VS => VS_EXT,
            }
        }

        /// Returns the xmlns prefix
        fn namespace_prefix(&self) -> &str {
            match self {
                WellKnownExtentions::BootstrapperApplications => BAL_NS_PREFIX,
                WellKnownExtentions::ComPlus => COMPLUS_NS_PREFIX,
                WellKnownExtentions::Dependency => DEPENDENCY_NS_PREFIX,
                WellKnownExtentions::DirectX => DIRECTX_NS_PREFIX,
                WellKnownExtentions::Firewall => FIREWALL_NS_PREFIX,
                WellKnownExtentions::Http => HTTP_NS_PREFIX,
                WellKnownExtentions::Iis => IIS_NS_PREFIX,
                WellKnownExtentions::Msmq => MSMQ_NS_PREFIX,
                WellKnownExtentions::Netfx => NETFX_NS_PREFIX,
                WellKnownExtentions::Powershell => POWERSHELL_NS_PREFIX,
                WellKnownExtentions::Sql => SQL_NS_PREFIX,
                WellKnownExtentions::UI => UI_NS_PREFIX,
                WellKnownExtentions::Util => UTIL_NS_PREFIX,
                WellKnownExtentions::VS => VS_NS_PREFIX,
            }
        }

        fn namespace_uri(&self) -> &str {
            match self {
                WellKnownExtentions::BootstrapperApplications => BAL_NS_URI,
                WellKnownExtentions::ComPlus => COMPLUS_NS_URI,
                WellKnownExtentions::Dependency => DEPENDENCY_NS_URI,
                WellKnownExtentions::DirectX => DIRECTX_NS_URI,
                WellKnownExtentions::Firewall => FIREWALL_NS_URI,
                WellKnownExtentions::Http => HTTP_NS_URI,
                WellKnownExtentions::Iis => IIS_NS_URI,
                WellKnownExtentions::Msmq => MSMQ_NS_URI,
                WellKnownExtentions::Netfx => NETFX_NS_URI,
                WellKnownExtentions::Powershell => POWERSHELL_NS_URI,
                WellKnownExtentions::Sql => SQL_NS_URI,
                WellKnownExtentions::UI => UI_NS_URI,
                WellKnownExtentions::Util => UTIL_NS_URI,
                WellKnownExtentions::VS => VS_NS_URI,
            }
        }
    }

    const BAL_EXT: &str = "WixToolset.BootstrapperApplications.wixext";
    const BAL_NS_PREFIX: &str = "bal";
    const BAL_NS_URI: &str = "http://wixtoolset.org/schemas/v4/wxs/bal";

    const COMPLUS_EXT: &str = "WixToolset.ComPlus.wixext";
    const COMPLUS_NS_PREFIX: &str = "complus";
    const COMPLUS_NS_URI: &str = "http://wixtoolset.org/schemas/v4/wxs/complus";

    const DEPENDENCY_EXT: &str = "WixToolset.Dependency.wixext";
    const DEPENDENCY_NS_PREFIX: &str = "dependency";
    const DEPENDENCY_NS_URI: &str = "http://wixtoolset.org/schemas/v4/wxs/dependency";

    const DIRECTX_EXT: &str = "WixToolset.DirectX.wixext";
    const DIRECTX_NS_PREFIX: &str = "directx";
    const DIRECTX_NS_URI: &str = "http://wixtoolset.org/schemas/v4/wxs/directx";

    const FIREWALL_EXT: &str = "WixToolset.Firewall.wixext";
    const FIREWALL_NS_PREFIX: &str = "firewall";
    const FIREWALL_NS_URI: &str = "http://wixtoolset.org/schemas/v4/wxs/firewall";

    const HTTP_EXT: &str = "WixToolset.Http.wixext";
    const HTTP_NS_PREFIX: &str = "http";
    const HTTP_NS_URI: &str = "http://wixtoolset.org/schemas/v4/wxs/http";

    const IIS_EXT: &str = "WixToolset.Iis.wixext";
    const IIS_NS_PREFIX: &str = "iis";
    const IIS_NS_URI: &str = "http://wixtoolset.org/schemas/v4/wxs/iis";

    const MSMQ_EXT: &str = "WixToolset.Msmq.wixext";
    const MSMQ_NS_PREFIX: &str = "msmq";
    const MSMQ_NS_URI: &str = "http://wixtoolset.org/schemas/v4/wxs/msmq";

    const NETFX_EXT: &str = "WixToolset.Netfx.wixext";
    const NETFX_NS_PREFIX: &str = "netfx";
    const NETFX_NS_URI: &str = "http://wixtoolset.org/schemas/v4/wxs/netfx";

    const POWERSHELL_EXT: &str = "WixToolset.PowerShell.wixext";
    const POWERSHELL_NS_PREFIX: &str = "powershell";
    const POWERSHELL_NS_URI: &str = "http://wixtoolset.org/schemas/v4/wxs/powershell";

    const SQL_EXT: &str = "WixToolset.Sql.wixext";
    const SQL_NS_PREFIX: &str = "sql";
    const SQL_NS_URI: &str = "http://wixtoolset.org/schemas/v4/wxs/sql";

    const UI_EXT: &str = "WixToolset.UI.wixext";
    const UI_NS_PREFIX: &str = "ui";
    const UI_NS_URI: &str = "http://wixtoolset.org/schemas/v4/wxs/ui";

    const UTIL_EXT: &str = "WixToolset.Util.wixext";
    const UTIL_NS_PREFIX: &str = "util";
    const UTIL_NS_URI: &str = "http://wixtoolset.org/schemas/v4/wxs/util";

    const VS_EXT: &str = "WixToolset.VisualStudio.wixext";
    const VS_NS_PREFIX: &str = "vs";
    const VS_NS_URI: &str = "http://wixtoolset.org/schemas/v4/wxs/vs";
}
