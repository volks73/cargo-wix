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

use std::path::PathBuf;
use log::warn;

use super::project::{open_wxs_source, WxsSchema};
use super::ext::{PackageCache, WxsDependency};
use super::Toolset;


/// Struct containing information about a wxs source file
pub struct WixSource {
    /// WiX toolset version
    pub(super) wix_version: WxsSchema,
    /// Path to this *.wxs file
    pub(super) path: PathBuf,
    /// Extensions this wix source is dependent on
    pub(super) exts: Vec<WxsDependency>,
    /// Toolset this source is using
    pub(super) toolset: Toolset,
}

impl WixSource {
    /// Returns true if the format of this *.wxs source can be upgraded
    pub fn can_upgrade(&self) -> bool {
        match self.wix_version {
            WxsSchema::Legacy => true,
            WxsSchema::V4 => false,
            WxsSchema::Unsupported => false,
        }
    }

    /// Returns true if this source is in the modern format
    /// 
    /// This is relevant because in the modern formats, extensions are namespaced. Knowing
    /// if the wxs format is "modern" indicates that extensions can be derived programatically.
    pub fn is_modern(&self) -> bool {
        matches!(self.wix_version, WxsSchema::V4)
    }

    /// Checks that the dependencies required by this *.wxs file exist in the package cache
    pub fn check_deps(&self, package_cache: &mut PackageCache) {
        for ext in self
            .exts
            .iter()
            .filter(|e| !package_cache.installed(*e))
            .collect::<Vec<_>>()
        {
            package_cache.add_missing(ext.package_name());
        }
    }

    /// Upgrades the current wix source file using `wix convert` if applicable
    ///
    /// Returns an updated WixSource object if the conversion and dependent ext install is successful
    pub fn upgrade(&self, work_dir: Option<&PathBuf>) -> crate::Result<Self> {
        let mut convert = self.toolset.wix("convert")?;
        let converted_path = if work_dir.is_some() {
            // If a work dir is specified, do not modify the input file directly
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
            // The converted_path must be a valid file name
            let converted_path = if let Some((work_dir, file_name)) = work_dir.zip(converted_path.file_name()) {
                // FIXNOW: Update the shim so that the program args are passed in to simplify this logic
                let dest = work_dir.join(file_name);
                if !dest.exists() {
                    std::fs::copy(converted_path, &dest)?;
                } else {
                    warn!("An existing file exists at destination `{dest:?}`, skip copying intermediate file");
                }
                dest
            } else {
                converted_path
            };
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