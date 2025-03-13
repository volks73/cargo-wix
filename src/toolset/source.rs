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

use log::{debug, warn};

use super::ext::{PackageCache, WxsDependency};
use super::project::{open_wxs_source, WxsSchema};
use super::Toolset;
use std::path::PathBuf;

/// Struct containing information about a wxs source file
pub struct WixSource {
    /// WiX toolset version
    pub(super) wxs_schema: WxsSchema,
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
        match (self.wxs_schema, &self.toolset) {
            (WxsSchema::Legacy, Toolset::Modern) => true,
            #[cfg(test)]
            (
                WxsSchema::Legacy,
                Toolset::Test {
                    is_legacy: false, ..
                },
            ) => true,
            _ => false,
        }
    }

    /// Returns true if this source is in the modern format
    ///
    /// This is relevant because in the modern formats, extensions are namespaced. Knowing
    /// if the wxs format is "modern" indicates that extensions can be derived programatically.
    pub fn is_modern(&self) -> bool {
        matches!(self.wxs_schema, WxsSchema::V4)
    }

    /// Checks that the dependencies required by this *.wxs file exist in the package cache
    pub fn check_deps(&self, package_cache: &mut PackageCache) {
        for ext in self
            .exts
            .iter()
            .filter(|e| !package_cache.installed(*e))
            .collect::<Vec<_>>()
        {
            // Package names are known ahead of time because they map to a well known extension uri
            // If a package name returns as empty, it means that tooling is not aware of it
            if !ext.package_name().is_empty() {
                debug!(
                    "Missing extension, xmlns:{}='{}'",
                    ext.namespace_prefix(),
                    ext.namespace_uri()
                );
                package_cache.add_missing(ext.package_name());
            } else {
                warn!(
                    "Unknown extension, xmlns:{}='{}'",
                    ext.namespace_prefix(),
                    ext.namespace_uri()
                );
            }
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
            let converted_path =
                if let Some((work_dir, file_name)) = work_dir.zip(converted_path.file_name()) {
                    let dest = work_dir.join(file_name);
                    std::fs::copy(converted_path, &dest)?;
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
            .field("wxs_schema", &self.wxs_schema)
            .field("path", &self.path)
            .finish()
    }
}

mod tests {

    #[test]
    fn test_source_can_upgrade() {
        use crate::toolset::{project::WxsSchema, source::WixSource};
        use std::path::PathBuf;

        assert_eq!(
            false,
            WixSource {
                wxs_schema: WxsSchema::Legacy,
                path: PathBuf::new(),
                exts: vec![],
                toolset: crate::toolset::Toolset::Legacy
            }
            .can_upgrade(),
            "should not be able to upgrade legacy w/ legacy toolset"
        );

        assert_eq!(
            true,
            WixSource {
                wxs_schema: WxsSchema::Legacy,
                path: PathBuf::new(),
                exts: vec![],
                toolset: crate::toolset::Toolset::Modern
            }
            .can_upgrade(),
            "should be able to upgrade legacy w/ modern toolset"
        );

        assert_eq!(
            false,
            WixSource {
                wxs_schema: WxsSchema::V4,
                path: PathBuf::new(),
                exts: vec![],
                toolset: crate::toolset::Toolset::Modern
            }
            .can_upgrade(),
            "should not be able to upgrade from v4"
        );
    }

    #[test]
    fn test_skip_add_unknown_ext_to_package_cache() {
        use super::WixSource;
        use crate::toolset::ext::{PackageCache, UnknownExtNamespace};
        use std::path::PathBuf;

        let source = WixSource {
            wxs_schema: crate::toolset::project::WxsSchema::V4,
            path: PathBuf::new(),
            exts: vec![Box::new(UnknownExtNamespace {
                prefix: String::from("test"),
                uri: String::from("test_uri"),
            })],
            toolset: crate::toolset::Toolset::Modern,
        };

        let mut package_cache = PackageCache::from(crate::toolset::Toolset::Modern);
        source.check_deps(&mut package_cache);

        assert_eq!(0, package_cache.iter_missing().count());
    }
}
