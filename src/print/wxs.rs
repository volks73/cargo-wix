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

use description;
use Error;
use eula::Eula;
use manifest;
use mustache::{self, MapBuilder};
use product_name;
use Result;
use std::path::{Path, PathBuf};
use Template;
use toml::Value;
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct Builder<'a> {
    binary_name: Option<&'a str>,
    description: Option<&'a str>,
    eula: Option<&'a str>,
    help_url: Option<&'a str>,
    input: Option<&'a str>,
    license: Option<&'a str>,
    manufacturer: Option<&'a str>,
    output: Option<&'a str>,
    product_name: Option<&'a str>,
}

impl<'a> Builder<'a> {
    /// Creates a new `Builder` instance.
    pub fn new() -> Self {
        Builder {
            binary_name: None,
            description: None,
            eula: None,
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
    /// Generally, the binary name should _not_ have spaces or special characters. The binary name
    /// is the name of the executable. This will _not_ appear in the Add/Remove Programs control
    /// panel. Use the `product_name` method to change the name that appears in the Add/Remove
    /// Programs control panel.
    pub fn binary_name(&mut self, b: Option<&'a str>) -> &mut Self {
        self.binary_name = b;
        self
    }

    /// Sets the description.
    ///
    /// This overrides the description determined from the `description` field in the package's
    /// manifest (Cargo.toml).
    pub fn description(&mut self, d: Option<&'a str>) -> &mut Self {
        self.description = d;
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
    pub fn eula(&mut self, e: Option<&'a str>) -> &mut Self {
        self.eula = e;
        self
    }

    /// Sets the help URL.
    ///
    /// The default is to obtain a URL from one of the following fields in the package's manifest
    /// (Cargo.toml): `documentation`, `homepage`, or `respository`. If none of these are
    /// specified, then the default is to exclude a help URL from the installer. This will override
    /// the default behavior and provide a help URL for the installer if none of the fields exist.
    pub fn help_url(&mut self, h: Option<&'a str>) -> &mut Self {
        self.help_url = h;
        self
    }

    /// Sets the path to a package's manifest (Cargo.toml) to be used to generate a WiX Source
    /// (wxs) file from the embedded template.
    ///
    /// A `wix` and `wix\main.wxs` file will be created in the same directory as the package's
    /// manifest. The default is to use the package's manifest in the current working directory.
    pub fn input(&mut self, i: Option<&'a str>) -> &mut Self {
        self.input = i;
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
    pub fn license(&mut self, l: Option<&'a str>) -> &mut Self {
        self.license = l;
        self
    }

    /// Sets the manufacturer.
    ///
    /// Default is to use the first author in the `authors` field of the package's manifest
    /// (Cargo.toml). This would override the default value.
    pub fn manufacturer(&mut self, m: Option<&'a str>) -> &mut Self {
        self.manufacturer = m;
        self
    }

    /// Sets the destination for creating all of the output from initialization. 
    ///
    /// The default is to create all initialization output in the current working directory.
    pub fn output(&mut self, o: Option<&'a str>) -> &mut Self {
        self.output = o;
        self
    }

    /// Sets the product name.
    ///
    /// The default is to use the `name` field under the `package` section of the package's
    /// manifest (Cargo.toml). This overrides that value. An error occurs if the `name` field is
    /// not found in the manifest.
    ///
    /// This is different from the binary name in that it is the name that appears in the
    /// Add/Remove Programs control panel, _not_ the name of the executable. The `binary_name`
    /// method can be used to change the executable name. This value can have spaces and special
    /// characters, where the binary (executable) name should avoid spaces and special characters.
    pub fn product_name(&mut self, p: Option<&'a str>) -> &mut Self {
        self.product_name = p;
        self
    }

    pub fn build(&self) -> Execution {
        Execution {
            binary_name: self.binary_name.map(String::from),
            description: self.description.map(String::from),
            eula: self.eula.map(PathBuf::from),
            help_url: self.help_url.map(String::from),
            input: self.input.map(PathBuf::from),
            license: self.license.map(PathBuf::from),
            manufacturer: self.manufacturer.map(String::from),
            output: self.output.map(PathBuf::from),
            product_name: self.product_name.map(String::from),
        }
    }
}

impl<'a> Default for Builder<'a> {
    fn default() -> Self {
        Builder::new()
    }
}

#[derive(Debug)]
pub struct Execution {
    binary_name: Option<String>,
    description: Option<String>,
    eula: Option<PathBuf>,
    help_url: Option<String>,
    input: Option<PathBuf>,
    license: Option<PathBuf>,
    manufacturer: Option<String>,
    output: Option<PathBuf>,
    product_name: Option<String>,
}

impl Execution {
    pub fn run(self) -> Result<()> {
        debug!("binary_name = {:?}", self.binary_name);
        debug!("description = {:?}", self.description);
        debug!("eula = {:?}", self.eula);
        debug!("help_url = {:?}", self.help_url);
        debug!("input = {:?}", self.input);
        debug!("license = {:?}", self.license);
        debug!("manufacturer = {:?}", self.manufacturer);
        debug!("output = {:?}", self.output);
        debug!("product_name = {:?}", self.product_name);
        let manifest = manifest(self.input.as_ref())?;
        let mut destination = super::destination(self.output.as_ref())?;
        let template = mustache::compile_str(Template::Wxs.to_str())?;
        let mut map = MapBuilder::new()
            .insert_str("binary-name", self.binary_name(&manifest)?)
            .insert_str("product-name", product_name(self.product_name.as_ref(), &manifest)?)
            .insert_str("manufacturer", self.manufacturer(&manifest)?)
            .insert_str("upgrade-code-guid", Uuid::new_v4().to_hyphenated().to_string().to_uppercase())
            .insert_str("path-component-guid", Uuid::new_v4().to_hyphenated().to_string().to_uppercase());
        if let Some(description) = description(self.description.clone(), &manifest) {
            map = map.insert_str("description", description);
        } else {
            warn!("A description was not specified at the command line or in the package's manifest \
                  (Cargo.toml). The description can be added manually to the generated WiX \
                  Source (wxs) file using a text editor.");
        }
        match Eula::new(self.eula.as_ref(), &manifest)? {
            Eula::Disabled => {
                warn!("An EULA was not specified at the command line, a RTF license file was \
                      not specified in the package's manifest (Cargo.toml), or the license ID \
                      from the package's manifest is not recognized. The license agreement \
                      dialog will be excluded from the installer. An EULA can be added manually \
                      to the generated WiX Source (wxs) file using a text editor.");
            },
            e => map = map.insert_str("eula", e.to_string()),
        }
        if let Some(url) = self.help_url.as_ref().or(Execution::help_url(&manifest).as_ref()) {
            map = map.insert_str("help-url", url.to_owned());
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
        template.render_data(&mut destination, &data).map_err(Error::from)
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
                .map_or(product_name(self.product_name.as_ref(), manifest), |s| Ok(s))
        }
    }

    fn help_url(manifest: &Value) -> Option<String> {
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
            super::first_author(&manifest)
        }
    }
}

impl Default for Execution {
    fn default() -> Self {
        Builder::new().build()
    }
}

