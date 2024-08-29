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

use std::{
    collections::{BTreeMap, BTreeSet},
    path::PathBuf,
    process::Command,
};
use log::debug;
use semver::Version;

/// Type-alias for .wxs extension dependency
pub type WxsDependency = Box<dyn WixExtension>;

/// Trait to provide wix extension identifiers w/ use the `wix extension` command
pub trait WixExtension {
    /// Returns the .wixext package name used to identify the extension
    fn package_name(&self) -> &str;
    /// Returns the xmlns prefix
    fn namespace_prefix(&self) -> &str;
    /// Returns the xmlns uri
    fn namespace_uri(&self) -> &str;
}

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

    /// Returns an iterator over missing extensions
    pub fn iter_missing(&self) -> impl Iterator<Item = &String> {
        self.missing.iter()
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
        work_dir: Option<&PathBuf>,
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
            if log::log_enabled!(log::Level::Debug) && !output.stderr.is_empty() {
                let std_err = String::from_utf8(output.stderr)?;
                for line in std_err.lines() {
                    debug!("{line}");
                }
            }
            Ok(())
        } else {
            Err("Could not install missing dependencies".into())
        }
    }
}

impl WixExtension for WxsDependency {
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

impl<'a> From<&sxd_document::dom::Namespace<'a>> for WxsDependency {
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