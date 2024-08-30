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

use std::{path::PathBuf, process::Command};
use log::debug;

use super::project::{open_wxs_source, WixNamespace};
use super::ext::{PackageCache, WxsDependency};


/// Struct containing information about a wxs source file
pub struct WixSource {
    /// WiX toolset version
    pub(super) wix_version: WixNamespace,
    /// Path to this *.wxs file
    pub(super) path: PathBuf,
    /// Extensions this wix source is dependent on
    pub(super) exts: Vec<WxsDependency>,
}

impl WixSource {
    /// Returns true if the format of this *.wxs source can be upgraded
    pub fn can_upgrade(&self) -> bool {
        match self.wix_version {
            WixNamespace::V3 => true,
            WixNamespace::Modern => false,
            WixNamespace::Unsupported => false,
        }
    }

    /// Returns true if this source is in the modern format
    pub fn is_modern(&self) -> bool {
        matches!(self.wix_version, WixNamespace::Modern)
    }

    /// Checks that the dependencies required by this *.wxs file exist in the package cache
    pub fn check_deps(&self, package_cache: &mut PackageCache) {
        for ext in self
            .exts
            .iter()
            .filter(|e| package_cache.installed(*e))
            .collect::<Vec<_>>()
        {
            package_cache.add_missing(ext.package_name());
        }
    }

    /// Upgrades the current wix source file using `wix convert` if applicable
    ///
    /// Returns an updated WixSource object if the conversion and dependent ext install is successful
    pub fn upgrade(&self, modify: bool) -> crate::Result<Self> {
        let mut convert = Command::new("wix");
        let convert = convert.arg("convert");
        let converted_path = if modify {
            convert.arg(&self.path);
            self.path.clone()
        } else {
            let temp = std::env::temp_dir().join(
                self.path
                    .file_name()
                    .expect("should have a file name because requires opening to create type"),
            );
            std::fs::copy(&self.path, &temp)?;
            convert.arg(&temp);
            temp
        };

        let output = convert.output()?;
        
        // Regardless of success, if debug is enabled and stderr isn't empty log to debug
        if log::log_enabled!(log::Level::Debug) && !output.stderr.is_empty() {
            let std_err = String::from_utf8(output.stderr)?;
            for line in std_err.lines() {
                debug!("upgrade({:?}): {line}", &converted_path);
            }
        }

        if output.status.success() {
            open_wxs_source(converted_path)
        } else {
            Err("Could not convert wix source".into())
        }
    }

    /// Copies the source file over to a different directory and re-opens the *.wxs file
    pub fn copy_to(&self, dir: PathBuf) -> crate::Result<Self> {
        if !dir.is_dir() {
            return Err(format!("{dir:?} is not a directory").as_str().into());
        }

        let dest = dir.join(
            self.path
                .file_name()
                .expect("should have a file name because requires opening to create type"),
        );

        std::fs::copy(&self.path, &dest)?;
        open_wxs_source(dest)
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