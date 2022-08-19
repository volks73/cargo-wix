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
use crate::Template;
use crate::LICENSE_FILE_NAME;
use crate::RTF_FILE_EXTENSION;

use log::{debug, trace};

use std::fmt;
use std::path::PathBuf;
use std::str::FromStr;

use cargo_metadata::Package;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Eula {
    CommandLine(PathBuf),
    Manifest(PathBuf),
    Generate(Template),
    Disabled,
}

impl Eula {
    pub fn new(p: Option<&PathBuf>, package: &Package) -> Result<Self> {
        if let Some(ref path) = p {
            Ok(Eula::CommandLine(path.into()))
        } else {
            Eula::from_manifest(package)
        }
    }

    pub fn from_manifest(package: &Package) -> Result<Self> {
        if let Some(license_file_path) = package.license_file() {
            trace!("The 'license-file' field is specified in the package's manifest (Cargo.toml)");
            debug!("license_file_path = {:?}", license_file_path);
            if license_file_path.extension().and_then(|s| s.to_str()) == Some(RTF_FILE_EXTENSION) {
                trace!(
                    "The '{}' path from the 'license-file' field in the package's \
                     manifest (Cargo.toml) has a RTF file extension.",
                    license_file_path.display()
                );
                if license_file_path.exists() {
                    trace!(
                        "The '{}' path from the 'license-file' field of the package's \
                         manifest (Cargo.toml) exists and has a RTF file extension.",
                        license_file_path.exists()
                    );
                    Ok(Eula::Manifest(license_file_path))
                } else {
                    Err(Error::Generic(format!(
                        "The '{}' file to be used for the EULA specified in the package's \
                         manifest (Cargo.toml) using the 'license-file' field does not exist.",
                        license_file_path.display()
                    )))
                }
            } else {
                trace!(
                    "The '{}' path from the 'license-file' field in the package's \
                     manifest (Cargo.toml) exists but it does not have a RTF file \
                     extension.",
                    license_file_path.display()
                );
                Ok(Eula::Disabled)
            }
        } else if let Some(license_name) = package.license.as_ref() {
            trace!("The 'license' field is specified in the package's manifest (Cargo.toml)");
            debug!("license_name = {:?}", license_name);
            if let Ok(template) = Template::from_str(license_name) {
                trace!(
                    "An embedded template for the '{}' license from the package's \
                     manifest (Cargo.toml) exists.",
                    license_name
                );
                Ok(Eula::Generate(template))
            } else {
                trace!(
                    "The '{}' license from the package's manifest (Cargo.toml) is \
                     unknown or an embedded template does not exist for it",
                    license_name
                );
                Ok(Eula::Disabled)
            }
        } else {
            trace!("The 'license' field is not specified in the package's manifest (Cargo.toml)");
            Ok(Eula::Disabled)
        }
    }
}

impl fmt::Display for Eula {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            Eula::CommandLine(ref path) => path.display().fmt(f),
            Eula::Manifest(ref path) => path.display().fmt(f),
            Eula::Generate(..) => write!(f, "{}.{}", LICENSE_FILE_NAME, RTF_FILE_EXTENSION),
            Eula::Disabled => write!(f, "Disabled"),
        }
    }
}
