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

use crate::Error;
use crate::Result;
use crate::StoredPath;
use crate::StoredPathBuf;
use crate::Template;
use crate::LICENSE_FILE_NAME;
use crate::RTF_FILE_EXTENSION;

use camino::Utf8Path;
use camino::Utf8PathBuf;
use log::trace;

use std::str::FromStr;

use cargo_metadata::Package;

/// License info
#[derive(Clone, Debug)]
pub struct Licenses {
    /// The license for the actual source code
    ///
    /// This likely will become/contain a Vec at some point,
    /// since dual MIT/Apache wants to have two license files!
    pub source_license: Option<License>,
    /// The end-user license (EULA) that must be agreed to when installing
    pub end_user_license: Option<License>,
}

/// A license file/item
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct License {
    /// Path to the file, relative to package's root dir.
    ///
    /// So generated files would be "wix\License.rtf"
    /// and the typical LICENSE file would just be "LICENSE-MIT".
    ///
    /// Conveniently this means we don't need to do any special handling
    /// to rewrite/relativize a path we get out of a Cargo.toml.
    ///
    /// It can also be an absolute path if the user passed this via CLI
    /// and doesn't care about persistence/portability.
    pub stored_path: StoredPathBuf,
    /// File name to use for the license when installed to the user's system
    ///
    /// If None, the source file name is used.
    pub name: Option<String>,

    /// This file needs to be generated, write it to the given path
    /// using the given Template.
    pub generate: Option<(Utf8PathBuf, Template)>,
}

impl License {
    /// Create a License entry with just the StoredPath
    fn from_stored_path(path: &StoredPath) -> Self {
        Self {
            name: None,
            stored_path: path.to_owned(),
            generate: None,
        }
    }
}

impl Licenses {
    pub fn new(
        dest_dir: Option<&Utf8Path>,
        license_path: Option<&StoredPath>,
        eula_path: Option<&StoredPath>,
        package: &Package,
    ) -> Result<Self> {
        let source_license = Self::find_source_license(dest_dir, license_path, package)?;
        let end_user_license =
            Self::find_end_user_license(eula_path, package, source_license.as_ref())?;

        Ok(Self {
            source_license,
            end_user_license,
        })
    }

    fn find_source_license(
        dest_dir: Option<&Utf8Path>,
        path: Option<&StoredPath>,
        package: &Package,
    ) -> Result<Option<License>> {
        trace!("finding source license for {}", package.name);
        // If explicitly passed, use that
        if let Some(path) = path {
            trace!("explicit source-license path passed as argument, using that");
            return Ok(Some(License::from_stored_path(path)));
        }

        // If there's [package.manifest.wix].license, respect that
        if let Some(license) = package
            .metadata
            .get("wix")
            .and_then(|w| w.as_object())
            .and_then(|t| t.get("license"))
        {
            trace!("[package.manifest.wix].license is specified");
            if let Some(license_enabled) = license.as_bool() {
                // If the user sets `eula = false`, disable the eula
                // (= true just falls through to the auto-detection stuff below)
                if !license_enabled {
                    trace!("[package.manifest.wix].license is false, disabling license support");
                    return Ok(None);
                } else {
                    trace!("[package.manifest.wix].license is true, continuing to auto-detect");
                }
            } else if let Some(path) = license.as_str() {
                trace!("[package.manifest.wix].license is a path, using that");
                // If the user sets `license = "path/to/license"`, use that
                return Ok(Some(License::from_stored_path(StoredPath::new(path))));
            } else {
                trace!("[package.manifest.wix].license is an invalid type");
                return Err(Error::Generic(format!(
                    "{}'s [package.metadata.wix].license must be a bool or a path",
                    package.name
                )));
            }
        }

        // First try Cargo's license_file field
        if let Some(path) = &package.license_file {
            // TODO: remap this relativity
            // TODO: reproduce path.exists() logic
            trace!("Cargo.toml license_file is specified, using that");
            return Ok(Some(License::from_stored_path(
                &StoredPathBuf::from_utf8_path(path),
            )));
        }

        // Next try Cargo's license field
        if let Some(name) = package.license.clone() {
            trace!("Cargo.toml license is specified");
            // If there's a template for this license, generate it
            if let (Some(dest_dir), Ok(generate)) = (dest_dir, Template::from_str(&name)) {
                trace!("Found a matching template, generating that");
                let file_name = format!("{LICENSE_FILE_NAME}.{RTF_FILE_EXTENSION}");
                let dest_file = dest_dir.join(file_name);
                let stored_path = StoredPathBuf::from_utf8_path(&dest_file);
                return Ok(Some(License {
                    name: None,
                    stored_path,
                    generate: Some((dest_file, generate)),
                }));
            } else {
                trace!("No matching template, ignoring license");
            }
        }

        trace!("No source-license found");
        Ok(None)
    }

    fn find_end_user_license(
        path: Option<&StoredPath>,
        package: &Package,
        source_license: Option<&License>,
    ) -> Result<Option<License>> {
        trace!("finding end-user-license for {}", package.name);
        // If explicitly passed, use that
        if let Some(path) = path {
            trace!("explicit end-user-license path passed as argument, using that");
            return Ok(Some(License::from_stored_path(path)));
        }

        // If there's [package.manifest.wix].eula, respect that
        if let Some(eula) = package
            .metadata
            .get("wix")
            .and_then(|w| w.as_object())
            .and_then(|t| t.get("eula"))
        {
            trace!("[package.manifest.wix].eula is specified");
            if let Some(eula_enabled) = eula.as_bool() {
                // If the user sets `eula = false`, disable the eula
                // (= true just falls through to the auto-detection stuff below)
                if !eula_enabled {
                    trace!("[package.manifest.wix].eula is false, disabling license support");
                    return Ok(None);
                } else {
                    trace!("[package.manifest.wix].eula is true, continuing to auto-detect");
                }
            } else if let Some(path) = eula.as_str() {
                trace!("[package.manifest.wix].eula is a path, using that");
                // If the user sets `eula = "path/to/license"`, use that
                return Ok(Some(License::from_stored_path(StoredPath::new(path))));
            } else {
                trace!("[package.manifest.wix].eula is an invalid type");
                return Err(Error::Generic(format!(
                    "{}'s [package.metadata.wix].eula must be a bool or a path",
                    package.name
                )));
            }
        }

        // Try to use the license, if it has a path and is RTF,
        if let Some(license) = source_license {
            trace!("source-license is defined");
            let path = &license.stored_path;
            if path.extension() == Some(RTF_FILE_EXTENSION) {
                trace!("using path from license");
                return Ok(Some(License::from_stored_path(path)));
            }
        }

        trace!("No end-user-license found");
        Ok(None)
    }
}
