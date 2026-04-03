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

use log::{debug, error, trace, warn};

use super::ext::{PackageCache, WxsDependency};
use super::project::{open_wxs_source, WxsSchema};
use super::Toolset;
use std::ffi::OsStr;
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
    /// True if this source defines a package
    pub(super) is_package: bool,
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

    /// Returns true if this source uses the bootstrapper applications (bal) namespace,
    /// indicating it produces a bundle (.exe) rather than an MSI package.
    ///
    /// The `bal` (WixBalExtension) namespace is required for bootstrapper bundles,
    /// so its presence reliably indicates a bundle.
    pub fn is_bundle(&self) -> bool {
        self.exts.iter().any(|e| e.namespace_prefix() == "bal")
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

        let content = std::fs::read_to_string(&converted_path)?;
        let has_bom = content.starts_with('\u{FEFF}');

        let output = convert.output()?;

        if !output.status.success() {
            // This is expected, `wix convert`` decided to always return a non-zero exit code because
            // it will output a log of the changes it made
            if log::log_enabled!(log::Level::Debug) && !output.stderr.is_empty() {
                let std_err = String::from_utf8(output.stderr.clone())?;
                for line in std_err.lines() {
                    debug!("{line}");
                }
            }
        }

        // The converted_path must be a valid file name
        let converted_path =
            if let Some((work_dir, file_name)) = work_dir.zip(converted_path.file_name()) {
                let dest = work_dir.join(file_name);
                std::fs::copy(converted_path, &dest)?;
                dest
            } else {
                converted_path
            };
        if !has_bom {
            // Strip the BOM if the previous file did not have a BOM
            let content = std::fs::read_to_string(&converted_path)?;
            let has_bom = content.starts_with('\u{FEFF}');
            if has_bom {
                debug!(
                    "Detected BOM, previous file did not have a BOM, removing to preserve tooling"
                );
                std::fs::write(&converted_path, content.trim_start_matches('\u{FEFF}'))?;
            }
        }
        open_wxs_source(converted_path)
    }

    pub fn try_move_to_installer_destination(
        &self,
        name: &str,
        version: &str,
        cfg: &rustc_cfg::Cfg,
        debug_name: bool,
        target_directory: &std::path::Path,
        output: Option<&String>,
    ) -> crate::Result<()> {
        if !self.is_package {
            return Ok(());
        }

        // Won't know the extension until we scan for the file
        // Capture what we need to do to create the filename
        // TODO: This would have been easier to just use PathBuf::add_extension, but it requires 1.91
        let file_name_with_ext = |ext: &str| -> PathBuf {
            let name = if self.is_main() {
                name
            } else {
                self.path
                    .file_stem()
                    .and_then(|f| f.to_str())
                    .unwrap_or(name)
            };
            let filename = if debug_name {
                format!("{}-{}-{}-debug.{}", name, version, cfg.target_arch, ext)
            } else {
                format!("{}-{}-{}.{}", name, version, cfg.target_arch, ext)
            };
            if let Some(path_str) = output {
                trace!("Using the explicitly specified output path for the MSI destination");
                let path = std::path::Path::new(path_str);
                if path_str.ends_with('/') || path_str.ends_with('\\') || path.is_dir() {
                    path.join(filename)
                } else {
                    path.to_owned()
                }
            } else {
                trace!(
                    "Using the package's manifest (Cargo.toml) file path to specify the MSI destination"
                );
                target_directory.join(crate::WIX).join(filename)
            }
        };

        let mut path = self.path.clone();
        for output_type in ["msi", "exe"] {
            path.set_extension(output_type);
            if path.exists() {
                let filename = file_name_with_ext(output_type);
                std::fs::rename(&path, &filename)?;
                debug!("Moving {path:?} to {filename:?}");
                return Ok(());
            }
        }

        error!("Expected {:?} to have output files", self.path);
        Err("Could not find package output for source".into())
    }

    fn is_main(&self) -> bool {
        self.path.file_stem() == Some(OsStr::new("main"))
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

        assert!(
            !WixSource {
                is_package: false,
                wxs_schema: WxsSchema::Legacy,
                path: PathBuf::new(),
                exts: vec![],
                toolset: crate::toolset::Toolset::Legacy
            }
            .can_upgrade(),
            "should not be able to upgrade legacy w/ legacy toolset"
        );

        assert!(
            WixSource {
                is_package: false,
                wxs_schema: WxsSchema::Legacy,
                path: PathBuf::new(),
                exts: vec![],
                toolset: crate::toolset::Toolset::Modern
            }
            .can_upgrade(),
            "should be able to upgrade legacy w/ modern toolset"
        );

        assert!(
            !WixSource {
                is_package: false,
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
            is_package: true,
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

    #[test]
    fn test_source_is_bundle() {
        use super::WixSource;
        use crate::toolset::ext::WellKnownExtentions;
        use std::path::PathBuf;

        let source_with_bal = WixSource {
            is_package: true,
            wxs_schema: crate::toolset::project::WxsSchema::V4,
            path: PathBuf::new(),
            exts: vec![Box::new(WellKnownExtentions::BootstrapperApplications)],
            toolset: crate::toolset::Toolset::Modern,
        };
        assert!(
            source_with_bal.is_bundle(),
            "source with bal namespace should be a bundle"
        );

        let source_without_bal = WixSource {
            is_package: true,
            wxs_schema: crate::toolset::project::WxsSchema::V4,
            path: PathBuf::new(),
            exts: vec![Box::new(WellKnownExtentions::UI)],
            toolset: crate::toolset::Toolset::Modern,
        };
        assert!(
            !source_without_bal.is_bundle(),
            "source without bal namespace should not be a bundle"
        );

        let source_no_exts = WixSource {
            is_package: true,
            wxs_schema: crate::toolset::project::WxsSchema::V4,
            path: PathBuf::new(),
            exts: vec![],
            toolset: crate::toolset::Toolset::Modern,
        };
        assert!(
            !source_no_exts.is_bundle(),
            "source with no exts should not be a bundle"
        );
    }
}
