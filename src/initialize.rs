// Copyright (C) 2018 Christopher R. Field.
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

use chrono::{Datelike, Utc};
use Error;
use mustache::{self, MapBuilder};
use regex::Regex;
use Result;
use std::env;
use std::fmt;
use std::fs::{self, File};
use std::io::Read;
use std::path::{Path, PathBuf};
use std::str::FromStr;
use Template;
use toml::Value;
use uuid::Uuid;
use wix::{CARGO_MANIFEST_FILE, WIX_SOURCE_FILE_NAME, WIX_SOURCE_FILE_EXTENSION, WIX, RTF_FILE_EXTENSION};

/// A builder for running the `cargo wix init` subcommand.
#[derive(Debug, Clone)]
pub struct Builder<'a> {
    binary_name: Option<&'a str>,
    copyright_year: Option<&'a str>,
    copyright_holder: Option<&'a str>,
    description: Option<&'a str>,
    eula: Option<&'a str>,
    force: bool,
    help_url: Option<&'a str>,
    input: Option<&'a str>,
    license: Option<&'a str>,
    manufacturer: Option<&'a str>,
    output: Option<&'a str>,
    product_name: Option<&'a str>,
}

impl<'a> Builder<'a> {
    /// Creates a new `Wix` instance.
    pub fn new() -> Self {
        Builder {
            binary_name: None,
            copyright_year: None,
            copyright_holder: None,
            description: None,
            eula: None,
            force: false,
            help_url: None,
            input: None,
            license: None,
            manufacturer: None,
            output: None,
            product_name: None,
        }
    }

    /// Sets the binary name.
    ///
    /// The default is to use the `name` field under the `bin` section of the package's manifest
    /// (Cargo.toml) or the `name` field under the `package` section if the `bin` section does
    /// _not_ exist. This overrides either of these defaults.
    ///
    /// Generally, the binary name should _not_ have spaces or special characters. The
    /// `product_name` can contain spaces or special characters.
    pub fn binary_name(&mut self, b: &'a str) -> &mut Self {
        self.binary_name = Some(b);
        self
    }

    /// Sets the copyright holder in the license dialog of the Windows installer (msi).
    pub fn copyright_holder(&mut self, h: &'a str) -> &mut Self {
        self.copyright_holder = Some(h);
        self
    }

    /// Sets the copyright year in the license dialog of the Windows installer (msi).
    pub fn copyright_year(&mut self, y: &'a str) -> &mut Self {
        self.copyright_year = Some(y);
        self
    }

    /// Sets the description.
    ///
    /// This overrides the description determined from the `description` field in the package's
    /// manifest (Cargo.toml).
    pub fn description(&mut self, d: &'a str) -> &mut Self {
        self.description = Some(d);
        self
    }

    /// Sets the path to a custom EULA.
    ///
    /// The default is to generate an EULA from an embedded template as a RTF file based on the
    /// name of the license specified in the `license` field of the package's manifest
    /// (Cargo.toml). If the `license` field is not specified or a template for the license does
    /// not exist but the `license-file` field does specify a path to a file with the RTF
    /// extension, then that RTF file is used as the EULA for the license agreement dialog in the
    /// installer. Finally, if the `license-file` does not exist or it specifies a file that does
    /// not have the `.rtf` extension, then the license agreement dialog is skipped and there is no
    /// EULA for the installer. This would override the default behavior and ensure the license
    /// agreement dialog is used.
    pub fn eula(&mut self, e: &'a str) -> &mut Self {
        self.eula = Some(e);
        self
    }

    /// Forces the generation of new output even if the various outputs already exists at the
    /// destination.
    pub fn force(&mut self, f: bool) -> &mut Self {
        self.force = f;
        self
    }

    /// Sets the help URL.
    ///
    /// The default is to obtain a URL from one of the following fields in the package's manifest
    /// (Cargo.toml): `documentation`, `homepage`, or `respository`. If none of these are
    /// specified, then the default is to exclude a help URL from the installer. This will override
    /// the default behavior and provide a help URL for the installer if none of the fields exist.
    pub fn help_url(&mut self, h: &'a str) -> &mut Self {
        self.help_url = Some(h);
        self
    }

    /// Sets the path to a package's manifest (Cargo.toml) to be used to generate a WiX Source
    /// (wxs) file from the embedded template.
    ///
    /// A `wix` and `wix\main.wxs` file will be created in the same directory as the package's
    /// manifest. The default is to use the package's manifest in the current working directory.
    pub fn input(&mut self, i: &'a str) -> &mut Self {
        self.input = Some(i);
        self
    }
    
    /// Sets the path to a file to be used as the
    /// [sidecar](https://en.wikipedia.org/wiki/Sidecar_file) license file.
    ///
    /// This will override the `license-file` field in the package's manifest (Cargo.toml). If the
    /// file has the `.rtf` extension, then it will also be used for the EULA in the license
    /// agreement dialog for the installer.
    ///
    /// The default is to use the value specified in the `license-file` field of the package's
    /// manifest or generate a license file and RTFed EULA from an embedded template based on the
    /// license name used in the `license` field of the package's manifest. If none of these fields
    /// are specified or overriden, then a license file is _not_ included in the installation
    /// directory and the license agreement dialog is skipped in the installer.
    pub fn license(&mut self, l: &'a str) -> &mut Self {
        self.license = Some(l);
        self
    }

    /// Sets the manufacturer.
    ///
    /// Default is to use the first author in the `authors` field of the package's manifest
    /// (Cargo.toml). This would override the default value.
    pub fn manufacturer(&mut self, m: &'a str) -> &mut Self {
        self.manufacturer = Some(m);
        self
    }

    /// Sets the destination for creating all of the output from initialization. 
    ///
    /// The default is to create all initialization output in the current working directory.
    pub fn output(&mut self, o: &'a str) -> &mut Self {
        self.output = Some(o);
        self
    }

    /// Sets the product name.
    ///
    /// The default is to use the `name` field under the `package` section of the package's
    /// manifest (Cargo.toml). This overrides that value. An error occurs if the `name` field is
    /// not found in the manifest.
    pub fn product_name(&mut self, p: &'a str) -> &mut Self {
        self.product_name = Some(p);
        self
    }
   
    /// Builds a read-only Initialize action.
    pub fn build(&mut self) -> Initialize {
        Initialize {
            binary_name: self.binary_name.map(String::from),
            copyright_year: self.copyright_year.map(String::from),
            copyright_holder: self.copyright_holder.map(String::from),
            description: self.description.map(String::from),
            eula: self.eula.map(PathBuf::from),
            force: self.force,
            help_url: self.help_url.map(String::from),
            input: self.input.map(PathBuf::from),
            license: self.license.map(PathBuf::from),
            manufacturer: self.manufacturer.map(String::from),
            output: self.output.map(PathBuf::from),
            product_name: self.product_name.map(String::from),
        }
    }
}

#[derive(Debug)]
pub struct Initialize {
    binary_name: Option<String>,
    copyright_year: Option<String>,
    copyright_holder: Option<String>,
    description: Option<String>,
    eula: Option<PathBuf>,
    force: bool,
    help_url: Option<String>,
    input: Option<PathBuf>,
    license: Option<PathBuf>,
    manufacturer: Option<String>,
    output: Option<PathBuf>,
    product_name: Option<String>,
}

impl Initialize {
    pub fn run(self) -> Result<()> {
        let manifest = self.manifest()?;
        let mut destination = self.destination()?;
        debug!("destination = {:?}", destination);
        if !destination.exists() {
            info!("Creating the '{}' directory", destination.display());
            fs::create_dir(&destination)?;
        }
        destination.push(WIX_SOURCE_FILE_NAME);
        destination.set_extension(WIX_SOURCE_FILE_EXTENSION);
        let eula = self.eula(&manifest)?; 
        if destination.exists() && !self.force {
            return Err(Error::Generic(format!(
                "The '{}' file already exists. Use the '--force' flag to overwite the contents.",
                destination.display()
            )));
        } else {
            info!("Creating the '{}\\{}.{}' file", WIX, WIX_SOURCE_FILE_NAME, WIX_SOURCE_FILE_EXTENSION);
            let mut main_wxs = File::create(&destination)?;
            let template = mustache::compile_str(Template::Wxs.to_str())?;
            let mut map = MapBuilder::new()
                .insert_str("binary-name", self.binary_name(&manifest)?)
                .insert_str("product-name", self.product_name(&manifest)?)
                .insert_str("manufacturer", self.manufacturer(&manifest)?)
                .insert_str("upgrade-code-guid", Uuid::new_v4().hyphenated().to_string().to_uppercase())
                .insert_str("path-component-guid", Uuid::new_v4().hyphenated().to_string().to_uppercase());
            if let Some(description) = self.description(&manifest) {
                map = map.insert_str("description", description);
            } else {
                warn!("A description was not specified at the command line or in the package's manifest \
                      (Cargo.toml). The description can be added manually to the generated WiX \
                      Source (wxs) file using a text editor.");
            }
            match eula {
                Eula::Disabled => {
                    warn!("An EULA was not specified at the command line, a RTF license file was \
                          not specified in the package's manifest (Cargo.toml), or the license ID \
                          from the package's manifest is not recognized. The license agreement \
                          dialog will be excluded from the installer. An EULA can be added manually \
                          to the generated WiX Source (wxs) file using a text editor.");
                },
                _ => map = map.insert_str("eula", eula.to_string()),
            }
            if let Some(url) = self.help_url(&manifest) {
                map = map.insert_str("help-url", url);
            } else {
                warn!("A help URL could not be found and it will be excluded from the installer. \
                      A help URL can be added manually to the generated WiX Source (wxs) file \
                      using a text editor.");
            }
            if let Some(name) = self.license_name(&manifest) {
                map = map.insert_str("license-name", name);
            }
            if let Some(source) = self.license_source(&manifest)? {
                map = map.insert_str("license-source", source);
            } else {
                warn!("A license file could not be found and it will be excluded from the \
                      installer. A license file can be added manually to the generated WiX Source \
                      (wxs) file using a text editor.");
            }
            let data = map.build();
            template.render_data(&mut main_wxs, &data)?;
        }
        destination.pop(); // Remove main.wxs
        if let Eula::Generate(template) = eula {
            destination.push("License");
            destination.set_extension(RTF_FILE_EXTENSION);
            if destination.exists() && !self.force {
                return Err(Error::Generic(format!(
                    "The '{}' file already exists. Use the '--force' flag to overwrite the contents.",
                    destination.display()
                )));
            } else {
                let mut rtf = File::create(destination)?;
                let mustache_template = mustache::compile_str(template.to_str())?;
                let data = MapBuilder::new()
                    .insert_str("copyright-year", self.copyright_year())
                    .insert_str("copyright-holder", self.copyright_holder(&manifest)?)
                    .build();
                mustache_template.render_data(&mut rtf, &data)?;
            }
        }
        Ok(())
    }

    fn binary_name(&self, manifest: &Value) -> Result<String> {
        if let Some(ref b) = self.binary_name {
            Ok(b.to_owned())
        } else {
            manifest.get("bin")
                .and_then(|b| b.as_table())
                .and_then(|t| t.get("name"))
                .and_then(|n| n.as_str())
                .map(String::from)
                .map_or(self.product_name(manifest), |s| Ok(s))
        }
    }

    fn copyright_holder(&self, manifest: &Value) -> Result<String> {
        if let Some(ref h) = self.copyright_holder {
            Ok(h.to_owned())
        } else {
            self.manufacturer(manifest)
        }
    }

    fn copyright_year(&self) -> String {
        self.copyright_year.clone()
            .map(String::from)
            .unwrap_or(Utc::now().year().to_string())
    }

    fn description(&self, manifest: &Value) -> Option<String> {
        self.description.clone().or(manifest.get("package")
            .and_then(|p| p.as_table())
            .and_then(|t| t.get("description"))
            .and_then(|d| d.as_str())
            .map(String::from))
    }

    fn destination(&self) -> Result<PathBuf> {
        if let Some(ref input) = self.input {
            trace!("An input path has been explicitly specified");
            if input.exists() && input.is_file() {
                trace!("The input path exists and it is a file");
                Ok(input.parent().map(|p| p.to_path_buf()).and_then(|mut p| {
                    p.push(WIX);
                    Some(p)
                }).unwrap())
            } else {
                Err(Error::Generic(format!("The '{}' path does not exist or it is not a file.", input.display())))
            }
        } else {
            trace!("An input path has NOT been explicitly specified, implicitly using the current working directory");
            let mut cwd = env::current_dir()?;
            cwd.push(WIX);
            Ok(cwd)
        }
    }

    fn eula(&self, manifest: &Value) -> Result<Eula> {
        if let Some(ref path) = self.eula {
            if path.exists() {
                trace!("The '{}' path from the command line for the EULA exists", path.display());
                Ok(Eula::CommandLine(path.into()))
            } else {
                Err(Error::Generic(format!(
                    "The '{}' path from the command line for the EULA does not exist", 
                    path.display()
                )))
            }
        } else {
            if let Some(license_file_path) = manifest.get("package")
                .and_then(|p| p.as_table())
                .and_then(|t| t.get("license-file"))
                .and_then(|l| l.as_str())
                .map(PathBuf::from) {
                trace!("The 'license-file' field is specified in the package's manifest (Cargo.toml).");
                if license_file_path.extension().and_then(|s| s.to_str()) == Some(RTF_FILE_EXTENSION) {
                    trace!("The '{}' path from the 'license-file' field in the package's \
                           manifest (Cargo.toml) has a RTF file extension.",
                           license_file_path.display()); 
                    if license_file_path.exists() {
                        trace!("The '{}' path from the 'license-file' field of the package's \
                               manifest (Cargo.toml) exists and has a RTF file extension.",
                               license_file_path.exists());
                        Ok(Eula::Manifest(license_file_path.into()))
                    } else {
                        Err(Error::Generic(format!(
                            "The '{}' file to be used for the EULA specified in the package's \
                            manifest (Cargo.toml) using the 'license-file' field does not exist.", 
                            license_file_path.display()
                        )))
                    }
                } else {
                    trace!("The '{}' path from the 'license-file' field in the package's \
                           manifest (Cargo.toml) exists but it does not have a RTF file \
                           extension.",
                           license_file_path.display());
                    Ok(Eula::Disabled)
                }
            } else {
                if let Some(license_name) = manifest.get("package")
                    .and_then(|p| p.as_table())
                    .and_then(|t| t.get("license"))
                    .and_then(|n| n.as_str()) {
                    trace!("The 'license' field is specified in the package's manifest (Cargo.toml).");
                    if let Ok(template) = Template::from_str(license_name) {
                        trace!("An embedded template for the '{}' license from the package's \
                               manifest (Cargo.toml) exists.", license_name);
                        Ok(Eula::Generate(template))
                    } else {
                        trace!("The '{}' license from the package's manifest (Cargo.toml) is \
                               unknown or an embedded template does not exist for it.", license_name);
                        Ok(Eula::Disabled)
                    }
                } else {
                    Ok(Eula::Disabled)
                }
            }
        }
    }

    fn help_url(&self, manifest: &Value) -> Option<String> {
        manifest.get("package")
            .and_then(|p| p.as_table())
            .and_then(|t| t.get("documentation").or(t.get("homepage")).or(t.get("repository")))
            .and_then(|h| h.as_str())
            .map(|s| {
                trace!("Using '{}' for the help URL", s);
                String::from(s)
            })
    }

    fn license_name(&self, manifest: &Value) -> Option<String> {
        if let Some(ref l) = self.license.clone().map(PathBuf::from) {
            l.file_name().and_then(|f| f.to_str()).map(String::from)
        } else {
            manifest.get("package")
                .and_then(|p| p.as_table())
                .and_then(|t| t.get("license-file"))
                .and_then(|l| l.as_str())
                .and_then(|l| {
                    Path::new(l).file_name()
                        .and_then(|f| f.to_str())
                        .map(String::from)
                })
        }
    }

    fn license_source(&self, manifest: &Value) -> Result<Option<String>> {
        // Order of precedence:
        //
        // 1. CLI (-l,--license)
        // 2. Manifest `license-file`
        // 3. LICENSE file in root
        if let Some(ref path) = self.license.clone().map(PathBuf::from) {
            if path.exists() {
                trace!("The '{}' path from the command line for the license exists",
                       path.display());
                Ok(path.to_str().map(String::from))
            } else {
                Err(Error::Generic(format!(
                    "The '{}' path from the command line for the license file does not exist", 
                    path.display()
                )))
            }
        } else {
            Ok(manifest.get("package")
                .and_then(|p| p.as_table())
                .and_then(|t| t.get("license-file"))
                .and_then(|l| l.as_str())
                .and_then(|s| {
                    let p = PathBuf::from(s);
                    if p.exists() {
                        trace!("The '{}' path from the 'license-file' field in the package's \
                               manifest (Cargo.toml) exists.", p.display());
                        Some(p.into_os_string().into_string().unwrap())
                    } else {
                        None
                    }
                }))
        }
    }

    fn manufacturer(&self, manifest: &Value) -> Result<String> {
        if let Some(ref m) = self.manufacturer {
            Ok(m.to_owned())
        } else {
            manifest.get("package")
                .and_then(|p| p.as_table())
                .and_then(|t| t.get("authors"))
                .and_then(|a| a.as_array())
                .and_then(|a| a.get(0)) 
                .and_then(|f| f.as_str())
                .and_then(|s| {
                    // Strip email if it exists.
                    let re = Regex::new(r"<(.*?)>").unwrap();
                    Some(re.replace_all(s, ""))
                })
                .map(|s| String::from(s.trim()))
                .ok_or(Error::Manifest("authors"))
        }
    }

    fn product_name(&self, manifest: &Value) -> Result<String> {
        if let Some(ref p) = self.product_name {
            Ok(p.to_owned())
        } else {
            manifest.get("package")
                .and_then(|p| p.as_table())
                .and_then(|t| t.get("name"))
                .and_then(|n| n.as_str())
                .map(String::from)
                .ok_or(Error::Manifest("name"))
        }
    }

    fn manifest(&self) -> Result<Value> {
        let default_manifest = {
            let mut cwd = env::current_dir()?;
            cwd.push(CARGO_MANIFEST_FILE);
            cwd
        };
        let cargo_file_path = self.input.as_ref().unwrap_or(&default_manifest);
        debug!("cargo_file_path = {:?}", cargo_file_path);
        let mut cargo_file = File::open(cargo_file_path)?;
        let mut cargo_file_content = String::new();
        cargo_file.read_to_string(&mut cargo_file_content)?;
        let manifest = cargo_file_content.parse::<Value>()?;
        Ok(manifest)
    }
}

enum Eula {
    CommandLine(PathBuf),
    Manifest(PathBuf),
    Generate(Template),
    Disabled,
}

impl fmt::Display for Eula {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            Eula::CommandLine(ref path) => path.display().fmt(f),
            Eula::Manifest(ref path) => path.display().fmt(f),
            Eula::Generate(..) => write!(f, "License.{}", RTF_FILE_EXTENSION),
            Eula::Disabled => write!(f, "Disabled"),
        }
    }
}

