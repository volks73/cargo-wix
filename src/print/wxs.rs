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
use EXE_FILE_EXTENSION;
use LICENSE_FILE_NAME;
use manifest;
use mustache::{self, MapBuilder};
use product_name;
use Result;
use RTF_FILE_EXTENSION;
use std::ffi::OsStr;
use std::path::{Path, PathBuf};
use Template;
use toml::Value;
use uuid::Uuid;

/// A builder for creating an execution context to print a WiX Toolset source file (wxs).
#[derive(Debug, Clone)]
pub struct Builder<'a> {
    banner: Option<&'a str>,
    binary: Option<&'a str>,
    description: Option<&'a str>,
    dialog: Option<&'a str>,
    eula: Option<&'a str>,
    help_url: Option<&'a str>,
    input: Option<&'a str>,
    license: Option<&'a str>,
    manufacturer: Option<&'a str>,
    output: Option<&'a str>,
    product_icon: Option<&'a str>,
    product_name: Option<&'a str>,
}

impl<'a> Builder<'a> {
    /// Creates a new `Builder` instance.
    pub fn new() -> Self {
        Builder {
            banner: None,
            binary: None,
            description: None,
            dialog: None,
            eula: None,
            help_url: None,
            input: None,
            license: None,
            manufacturer: None,
            output: None,
            product_icon: None,
            product_name: None,
        }
    }

    /// Sets the path to a bitmap (BMP) file to be used as a banner image across
    /// the top of each dialog in the installer.
    ///
    /// The banner image must be 493 x 58 pixels. See the [Wix Toolset
    /// documentation] for details about [customization].
    ///
    /// [Wix Toolset documentation]: http://wixtoolset.org/documentation/
    /// [customization]: http://wixtoolset.org/documentation/manual/v3/wixui/wixui_customizations.html
    pub fn banner(&mut self, b: Option<&'a str>) -> &mut Self {
        self.banner = b;
        self
    }

    /// Sets the binary.
    ///
    /// The default is to use the `name` field under the `bin` section of the
    /// package's manifest (Cargo.toml) or the `name` field under the `package`
    /// section if the `bin` section does _not_ exist. This overrides either of
    /// these defaults.
    ///
    /// Generally, the binary name should _not_ have spaces, special characters,
    /// or a file extension. The binary name is the name of the executable. This
    /// will _not_ appear in the Add/Remove Programs control panel. Use the
    /// `product_name` method to change the name that appears in the Add/Remove
    /// Programs control panel.
    pub fn binary(&mut self, b: Option<&'a str>) -> &mut Self {
        self.binary = b;
        self
    }

    /// Sets the description.
    ///
    /// This overrides the description determined from the `description` field
    /// in the package'
    pub fn description(&mut self, d: Option<&'a str>) -> &mut Self {
        self.description = d;
        self
    }

    /// Sets the path to a bitmap (`.bmp`) file that will be displayed on the
    /// first dialog to the left.
    ///
    /// The image must be 493 x 312 pixels. See the [Wix Toolset
    /// documentation] for details about [customization].
    ///
    /// [Wix Toolset documentation]: http://wixtoolset.org/documentation/
    /// [customization]: http://wixtoolset.org/documentation/manual/v3/wixui/wixui_customizations.html
    pub fn dialog(&mut self, d: Option<&'a str>) -> &mut Self {
        self.dialog = d;
        self
    }

    /// Sets the path to a custom EULA.
    ///
    /// The default is to generate an EULA from an embedded template as a RTF
    /// file based on the name of the license specified in the `license` field
    /// of the package's manifest (Cargo.toml). If the `license` field is not
    /// specified or a template for the license does not exist but the
    /// `license-file` field does specify a path to a file with the RTF
    /// extension, then that RTF file is used as the EULA for the license
    /// agreement dialog in the installer. Finally, if the `license-file` does
    /// not exist or it specifies a file that does not have the `.rtf`
    /// extension, then the license agreement dialog is skipped and there is no
    /// EULA for the installer. This would override the default behavior and
    /// ensure the license agreement dialog is used.
    pub fn eula(&mut self, e: Option<&'a str>) -> &mut Self {
        self.eula = e;
        self
    }

    /// Sets the help URL.
    ///
    /// The default is to obtain a URL from one of the following fields in the
    /// package's manifest (Cargo.toml): `documentation`, `homepage`, or
    /// `respository`. If none of these are specified, then the default is to
    /// exclude a help URL from the installer. This will override the default
    /// behavior and provide a help URL for the installer if none of the fields
    /// exist.
    pub fn help_url(&mut self, h: Option<&'a str>) -> &mut Self {
        self.help_url = h;
        self
    }

    /// Sets the path to a package's manifest (Cargo.toml) to be used to
    /// generate a WiX Source (wxs) file from the embedded template.
    ///
    /// A `wix` and `wix\main.wxs` file will be created in the same directory as
    /// the package's manifest. The default is to use the package's manifest in
    /// the current working directory.
    pub fn input(&mut self, i: Option<&'a str>) -> &mut Self {
        self.input = i;
        self
    }

    /// Sets the path to a file to be used as the license [sidecar] file.
    ///
    /// The default is to use the value specified in the `license-file` field of
    /// the package's manifest (Cargo.toml) or generate a Rich Text Format (RTF)
    /// license file from an embedded template based on the license ID used in
    /// the `license` field of the package's manifest. If none of these fields
    /// are specified or overriden, then a license sidecar file is _not_
    /// included in the installation directory.
    ///
    /// This will override the default behavior and skip using either the
    /// `license` or `license-file` fields in the package's manifest.
    ///
    /// [sidecar]: https://en.wikipedia.org/wiki/Sidecar_file
    pub fn license(&mut self, l: Option<&'a str>) -> &mut Self {
        self.license = l;
        self
    }

    /// Sets the manufacturer.
    ///
    /// Default is to use the first author in the `authors` field of the
    /// package's manifest (Cargo.toml). This would override the default value.
    pub fn manufacturer(&mut self, m: Option<&'a str>) -> &mut Self {
        self.manufacturer = m;
        self
    }

    /// Sets the destination for creating all of the output from initialization.
    ///
    /// The default is to create all initialization output in the current
    /// working directory.
    pub fn output(&mut self, o: Option<&'a str>) -> &mut Self {
        self.output = o;
        self
    }

    /// Sets the path to an image file to be used for product icon.
    ///
    /// The product icon is the icon that appears for an installed application
    /// in the Add/Remove Programs (ARP) control panel. If a product icon is
    /// _not_ defined for an application within the installer, then the Windows
    /// OS assigns a generic one.
    pub fn product_icon(&mut self, p: Option<&'a str>) -> &mut Self {
        self.product_icon = p;
        self
    }

    /// Sets the product name.
    ///
    /// The default is to use the `name` field under the `package` section of
    /// the package's manifest (Cargo.toml). This overrides that value. An error
    /// occurs if the `name` field is not found in the manifest.
    ///
    /// This is different from the binary name in that it is the name that
    /// appears in the Add/Remove Programs (ARP) control panel, _not_ the name
    /// of the executable. The [`binary_name`] method can be used to change the
    /// executable name. This value can have spaces and special characters,
    /// where the binary name should avoid spaces and special characters.
    ///
    /// [`binary_name`]: #binary_name
    pub fn product_name(&mut self, p: Option<&'a str>) -> &mut Self {
        self.product_name = p;
        self
    }

    /// Builds an execution context for printing a template.
    pub fn build(&self) -> Execution {
        Execution {
            banner: self.banner.map(PathBuf::from),
            binary: self.binary.map(PathBuf::from),
            description: self.description.map(String::from),
            dialog: self.dialog.map(PathBuf::from),
            eula: self.eula.map(PathBuf::from),
            help_url: self.help_url.map(String::from),
            input: self.input.map(PathBuf::from),
            license: self.license.map(PathBuf::from),
            manufacturer: self.manufacturer.map(String::from),
            output: self.output.map(PathBuf::from),
            product_icon: self.product_icon.map(PathBuf::from),
            product_name: self.product_name.map(String::from),
        }
    }
}

impl<'a> Default for Builder<'a> {
    fn default() -> Self {
        Builder::new()
    }
}

/// A context for printing a WiX Toolset source file (wxs).
#[derive(Debug)]
pub struct Execution {
    banner: Option<PathBuf>,
    binary: Option<PathBuf>,
    description: Option<String>,
    dialog: Option<PathBuf>,
    eula: Option<PathBuf>,
    help_url: Option<String>,
    input: Option<PathBuf>,
    license: Option<PathBuf>,
    manufacturer: Option<String>,
    output: Option<PathBuf>,
    product_icon: Option<PathBuf>,
    product_name: Option<String>,
}

impl Execution {
    /// Prints a WiX Source (wxs) file based on the built context.
    pub fn run(self) -> Result<()> {
        debug!("banner = {:?}", self.banner);
        debug!("binary = {:?}", self.binary);
        debug!("description = {:?}", self.description);
        debug!("dialog = {:?}", self.description);
        debug!("eula = {:?}", self.eula);
        debug!("help_url = {:?}", self.help_url);
        debug!("input = {:?}", self.input);
        debug!("license = {:?}", self.license);
        debug!("manufacturer = {:?}", self.manufacturer);
        debug!("output = {:?}", self.output);
        debug!("product_icon = {:?}", self.product_icon);
        debug!("product_name = {:?}", self.product_name);
        let manifest = manifest(self.input.as_ref())?;
        let mut destination = super::destination(self.output.as_ref())?;
        let template = mustache::compile_str(Template::Wxs.to_str())?;
        let mut map = MapBuilder::new()
            .insert_str("binary-name", self.binary_name(&manifest)?)
            .insert_str("binary-source", self.binary_source(&manifest)?)
            .insert_str("product-name", product_name(self.product_name.as_ref(), &manifest)?)
            .insert_str("manufacturer", self.manufacturer(&manifest)?)
            .insert_str("upgrade-code-guid", Uuid::new_v4().to_hyphenated().to_string().to_uppercase())
            .insert_str("path-component-guid", Uuid::new_v4().to_hyphenated().to_string().to_uppercase());
        if let Some(ref banner) = self.banner {
            map = map.insert_str("banner", banner.display().to_string());
        }
        if let Some(description) = description(self.description.clone(), &manifest) {
            map = map.insert_str("description", description);
        } else {
            warn!("A description was not specified at the command line or in the package's manifest \
                  (Cargo.toml). The description can be added manually to the generated WiX \
                  Source (wxs) file using a text editor.");
        }
        if let Some(ref dialog) = self.dialog {
            map = map.insert_str("dialog", dialog.display().to_string());
        }
        match self.eula(&manifest)? {
            Eula::Disabled => {
                warn!("An EULA was not specified at the command line, a RTF \
                      license file was not specified in the package manifest's \
                      (Cargo.toml) 'license-file' field, or the license ID from the \
                      package manifest's 'license' field is not recognized. The \
                      license agreement dialog will be excluded from the installer. An \
                      EULA can be added manually to the generated WiX Source (wxs) \
                      file using a text editor.");
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
        if let Some(icon) = self.product_icon {
            map = map.insert_str("product-icon", icon.display().to_string());
        }
        let data = map.build();
        template.render_data(&mut destination, &data).map_err(Error::from)
    }

    fn binary_name(&self, manifest: &Value) -> Result<String> {
        if let Some(ref b) = self.binary {
            Ok(b.file_stem().and_then(OsStr::to_str).map(String::from).expect("Binary path to have a file name"))
        } else {
            manifest.get("bin")
                .and_then(|b| b.as_array())
                .and_then(|a| a.get(0))
                .and_then(|e| e.as_table())
                .and_then(|t| t.get("name"))
                .and_then(|n| n.as_str())
                .map_or(
                    product_name(None, manifest),
                    |s| Ok(String::from(s))
                )
        }
    }

    fn binary_source(&self, manifest: &Value) -> Result<String> {
        if let Some(ref path) = self.binary.clone().map(PathBuf::from) {
            Ok(path.to_str().map(String::from).expect("Path to string conversion"))
        } else {
            self.binary_name(manifest).map(|name| {
                let mut path = PathBuf::from("target").join("release").join(name);
                path.set_extension(EXE_FILE_EXTENSION);
                path.to_str().map(String::from).expect("Path to string conversion")
            })
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

    fn eula(&self, manifest: &Value) -> Result<Eula> {
        if let Some(ref path) = self.eula.clone().map(PathBuf::from) {
            Eula::new(Some(path), manifest)
        } else {
            Eula::new(self.license.clone()
                .map(PathBuf::from)
                .filter(|p| p.extension().and_then(|p| p.to_str()) == Some(RTF_FILE_EXTENSION))
                .as_ref(),
                manifest
            )
        }
    }

    fn license_name(&self, manifest: &Value) -> Option<String> {
        if let Some(ref l) = self.license.clone().map(PathBuf::from) {
            l.file_name().and_then(|f| f.to_str()).map(String::from)
        } else {
            manifest.get("package")
                .and_then(|p| p.as_table())
                .and_then(|t| {
                    t.get("license")
                        .filter(|l| {
                            if let Some(s) = l.as_str() {
                                Template::license_ids().contains(&s.to_owned())
                            } else {
                                false
                            }
                        })
                        .map(|_| String::from(LICENSE_FILE_NAME))
                        .or_else(|| {
                            t.get("license-file")
                            .and_then(|l| l.as_str())
                            .and_then(|l| {
                                Path::new(l).file_name()
                                    .and_then(|f| f.to_str())
                                    .map(String::from)
                            })
                        })
                })
        }
    }

    fn license_source(&self, manifest: &Value) -> Result<Option<String>> {
        if let Some(ref path) = self.license.clone().map(PathBuf::from) {
            Ok(path.to_str().map(String::from))
        } else {
            Ok(manifest.get("package")
                .and_then(|p| p.as_table())
               .and_then(|t| {
                   t.get("license")
                       .filter(|l| {
                           if let Some(s) = l.as_str() {
                               Template::license_ids().contains(&s.to_string())
                           } else {
                               false
                           }
                       })
                       .map(|_| LICENSE_FILE_NAME.to_owned() + "." + RTF_FILE_EXTENSION)
                       .or_else(|| {
                            t.get("license-file")
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
                            })
                       })
               })
            )
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

#[cfg(test)]
mod tests {
    use super::*;

    mod builder {
        use super::*;

        #[test]
        fn banner_works() {
            const EXPECTED: &str = "img\\Banner.bmp";
            let mut actual = Builder::new();
            actual.banner(Some(EXPECTED));
            assert_eq!(actual.banner, Some(EXPECTED));
        }

        #[test]
        fn binary_name_works() {
            const EXPECTED: &str = "bin\\Example.exe";
            let mut actual = Builder::new();
            actual.binary(Some(EXPECTED));
            assert_eq!(actual.binary, Some(EXPECTED));
        }

        #[test]
        fn description_works() {
            const EXPECTED: &str = "This is a description.";
            let mut actual = Builder::new();
            actual.description(Some(EXPECTED));
            assert_eq!(actual.description, Some(EXPECTED));
        }

        #[test]
        fn dialog_work() {
            const EXPECTED: &str = "img\\Dialog.bmp";
            let mut actual = Builder::new();
            actual.dialog(Some(EXPECTED));
            assert_eq!(actual.dialog, Some(EXPECTED));
        }

        #[test]
        fn eula_works() {
            const EXPECTED: &str = "Example_Eula.rtf";
            let mut actual = Builder::new();
            actual.eula(Some(EXPECTED));
            assert_eq!(actual.eula, Some(EXPECTED));
        }

        #[test]
        fn help_url_works() {
            const EXPECTED: &str = "http://www.example.com";
            let mut actual = Builder::new();
            actual.help_url(Some(EXPECTED));
            assert_eq!(actual.help_url, Some(EXPECTED));
        }

        #[test]
        fn input_works() {
            const EXPECTED: &str = "C:\\example\\Cargo.toml";
            let mut actual = Builder::new();
            actual.input(Some(EXPECTED));
            assert_eq!(actual.input, Some(EXPECTED));
        }

        #[test]
        fn license_works() {
            const EXPECTED: &str = "C:\\example\\Example License.rtf";
            let mut actual = Builder::new();
            actual.license(Some(EXPECTED));
            assert_eq!(actual.license, Some(EXPECTED));
        }

        #[test]
        fn manufacturer_works() {
            const EXPECTED: &str = "Example";
            let mut actual = Builder::new();
            actual.manufacturer(Some(EXPECTED));
            assert_eq!(actual.manufacturer, Some(EXPECTED));
        }

        #[test]
        fn output_works() {
            const EXPECTED: &str = "C:\\example\\output";
            let mut actual = Builder::new();
            actual.output(Some(EXPECTED));
            assert_eq!(actual.output, Some(EXPECTED));
        }

        #[test]
        fn product_icon_works() {
            const EXPECTED: &str = "img\\Product.ico";
            let mut actual = Builder::new();
            actual.product_icon(Some(EXPECTED));
            assert_eq!(actual.product_icon, Some(EXPECTED));
        }

        #[test]
        fn product_name_works() {
            const EXPECTED: &str = "Example Product Name";
            let mut actual = Builder::new();
            actual.product_name(Some(EXPECTED));
            assert_eq!(actual.product_name, Some(EXPECTED));
        }
    }

    mod execution {
        extern crate tempfile;

        use std::fs::File;
        use super::*;

        const MIN_MANIFEST: &str = r#"[package]
            name = "Example"
            version = "0.1.0"
            authors = ["First Last <first.last@example.com>"]
        "#;

        const MIT_MANIFEST: &str = r#"[package]
            name = "Example"
            version = "0.1.0"
            authors = ["First Last <first.last@example.com>"]
            license = "MIT"
        "#;

        const GPL3_MANIFEST: &str = r#"[package]
            name = "Example"
            version = "0.1.0"
            authors = ["First Last <first.last@example.com>"]
            license = "GPL-3.0"
        "#;

        const APACHE2_MANIFEST: &str = r#"[package]
            name = "Example"
            version = "0.1.0"
            authors = ["First Last <first.last@example.com>"]
            license = "Apache-2.0"
        "#;

        const UNKNOWN_MANIFEST: &str = r#"[package]
            name = "Example"
            version = "0.1.0"
            authors = ["First Last <first.last@example.com>"]
            license = "XYZ"
        "#;

        const MIT_MANIFEST_BIN: &str = r#"[package]
            name = "Example"
            version = "0.1.0"
            authors = ["First Last <first.last@example.com>"]
            license = "MIT"

            [[bin]]
            name = "Different"
        "#;

        const DOCUMENTATION_MANIFEST: &str = r#"[package]
            name = "Example"
            version = "0.1.0"
            authors = ["First Last <first.last@example.com>"]
            license = "MIT"
            documentation = "http://www.example.com"
        "#;

        const HOMEPAGE_MANIFEST: &str = r#"[package]
            name = "Example"
            version = "0.1.0"
            authors = ["First Last <first.last@example.com>"]
            license = "MIT"
            homepage = "http://www.example.com"
        "#;

        const REPOSITORY_MANIFEST: &str = r#"[package]
            name = "Example"
            version = "0.1.0"
            authors = ["First Last <first.last@example.com>"]
            license = "MIT"
            repository = "http://www.example.com"
        "#;

        #[test]
        fn license_name_with_mit_license_field_works() {
            let manifest = MIT_MANIFEST.parse::<Value>().expect("Parsing TOML");
            let actual = Execution::default().license_name(&manifest).expect("License name");
            assert_eq!(actual, String::from(LICENSE_FILE_NAME));
        }

        #[test]
        fn license_name_with_gpl3_license_field_works() {
            let manifest = GPL3_MANIFEST.parse::<Value>().expect("Parsing TOML");
            let actual = Execution::default().license_name(&manifest).expect("License name");
            assert_eq!(actual, String::from(LICENSE_FILE_NAME));
        }

        #[test]
        fn license_name_with_apache2_license_field_works() {
            let manifest = APACHE2_MANIFEST.parse::<Value>().expect("Parsing TOML");
            let actual = Execution::default().license_name(&manifest).expect("License name");
            assert_eq!(actual, String::from(LICENSE_FILE_NAME));
        }

        #[test]
        fn license_name_with_unknown_license_field_works() {
            let manifest = UNKNOWN_MANIFEST.parse::<Value>().expect("Parsing TOML");
            let actual = Execution::default().license_name(&manifest);
            assert!(actual.is_none());
        }

        #[test]
        fn license_source_with_mit_license_field_works() {
            let manifest = MIT_MANIFEST.parse::<Value>().expect("Parsing TOML");
            let actual = Execution::default().license_source(&manifest).expect("License source");
            assert_eq!(actual, Some(LICENSE_FILE_NAME.to_owned() + "." + RTF_FILE_EXTENSION));
        }

        #[test]
        fn license_source_with_gpl3_license_field_works() {
            let manifest = GPL3_MANIFEST.parse::<Value>().expect("Parsing TOML");
            let actual = Execution::default().license_source(&manifest).expect("License source");
            assert_eq!(actual, Some(String::from(LICENSE_FILE_NAME.to_owned() + "." + RTF_FILE_EXTENSION)));
        }

        #[test]
        fn license_source_with_apache2_license_field_works() {
            let manifest = APACHE2_MANIFEST.parse::<Value>().expect("Parsing TOML");
            let actual = Execution::default().license_source(&manifest).expect("License source");
            assert_eq!(actual, Some(String::from(LICENSE_FILE_NAME.to_owned() + "." + RTF_FILE_EXTENSION)));
        }

        #[test]
        fn license_source_with_unknown_license_field_works() {
            let manifest = UNKNOWN_MANIFEST.parse::<Value>().expect("Parsing TOML");
            let actual = Execution::default().license_source(&manifest).unwrap();
            assert!(actual.is_none());
        }

        #[test]
        fn binary_name_with_defaults_works() {
            let manifest = MIT_MANIFEST.parse::<Value>().expect("Parsing TOML");
            let actual = Execution::default().binary_name(&manifest).unwrap();
            assert_eq!(actual, "Example".to_owned());
        }

        #[test]
        fn binary_name_with_bin_name_works() {
            let manifest = MIT_MANIFEST_BIN.parse::<Value>().expect("Parsing TOML");
            let actual = Execution::default().binary_name(&manifest).unwrap();
            assert_eq!(actual, "Different".to_owned());
        }

        #[test]
        fn binary_name_with_override_works() {
            const EXPECTED: &str = "bin\\Example.exe";
            let manifest = MIT_MANIFEST_BIN.parse::<Value>().expect("Parsing TOML");
            let actual = Builder::default()
                .binary(Some(EXPECTED))
                .build()
                .binary_name(&manifest)
                .unwrap();
            assert_eq!(actual, String::from("Example"));
        }

        #[test]
        fn binary_source_with_defaults_works() {
            const EXPECTED: &str = r#"target\release\Example.exe"#;
            let manifest = MIT_MANIFEST.parse::<Value>().expect("Parsing TOML");
            let actual = Execution::default().binary_source(&manifest).unwrap();
            assert_eq!(actual, EXPECTED);
        }

        #[test]
        fn binary_source_with_bin_name_works() {
            const EXPECTED: &str = r#"target\release\Different.exe"#;
            let manifest = MIT_MANIFEST_BIN.parse::<Value>().expect("Parsing TOML");
            let actual = Execution::default().binary_source(&manifest).unwrap();
            assert_eq!(actual, EXPECTED);
        }

        #[test]
        fn binary_source_with_override_works() {
            const EXPECTED: &str = "bin\\Example.exe";
            let manifest = MIT_MANIFEST_BIN.parse::<Value>().expect("Parsing TOML");
            let actual = Builder::default()
                .binary(Some(EXPECTED))
                .build()
                .binary_source(&manifest)
                .unwrap();
            assert_eq!(actual, EXPECTED);
        }

        #[test]
        fn manufacturer_with_defaults_works() {
            const EXPECTED: &str = "First Last";
            let manifest = MIN_MANIFEST.parse::<Value>().expect("Parsing TOML");
            let actual = Execution::default().manufacturer(&manifest).unwrap();
            assert_eq!(actual, String::from(EXPECTED));
        }

        #[test]
        fn manufacturer_with_override_works() {
            const EXPECTED: &str = "Example";
            let manifest = MIN_MANIFEST.parse::<Value>().expect("Parsing TOML");
            let actual = Builder::default()
                .manufacturer(Some(EXPECTED))
                .build()
                .manufacturer(&manifest)
                .unwrap();
            assert_eq!(actual, String::from(EXPECTED));
        }

        #[test]
        fn help_url_with_defaults_works() {
            let manifest = MIN_MANIFEST.parse::<Value>().expect("Parsing TOML");
            let actual = Execution::help_url(&manifest);
            assert!(actual.is_none());
        }

        #[test]
        fn help_url_with_documentation_works() {
            const EXPECTED: &str = "http://www.example.com";
            let manifest = DOCUMENTATION_MANIFEST.parse::<Value>().expect("Parsing TOML");
            let actual = Execution::help_url(&manifest);
            assert_eq!(actual, Some(String::from(EXPECTED)));
        }

        #[test]
        fn help_url_with_homepage_works() {
            const EXPECTED: &str = "http://www.example.com";
            let manifest = HOMEPAGE_MANIFEST.parse::<Value>().expect("Parsing TOML");
            let actual = Execution::help_url(&manifest);
            assert_eq!(actual, Some(String::from(EXPECTED)));
        }

        #[test]
        fn help_url_with_repository_works() {
            const EXPECTED: &str = "http://www.example.com";
            let manifest = REPOSITORY_MANIFEST.parse::<Value>().expect("Parsing TOML");
            let actual = Execution::help_url(&manifest);
            assert_eq!(actual, Some(String::from(EXPECTED)));
        }

        #[test]
        fn eula_with_defaults_works() {
            let manifest = MIN_MANIFEST.parse::<Value>().expect("Parsing TOML");
            let actual = Execution::default().eula(&manifest).unwrap();
            assert_eq!(actual, Eula::Disabled);
        }

        #[test]
        fn eula_with_mit_license_field_works() {
            let manifest = MIT_MANIFEST.parse::<Value>().expect("Parsing TOML");
            let actual = Execution::default().eula(&manifest).unwrap();
            assert_eq!(actual, Eula::Generate(Template::Mit));
        }

        #[test]
        fn eula_with_apache2_license_field_works() {
            let manifest = APACHE2_MANIFEST.parse::<Value>().expect("Parsing TOML");
            let actual = Execution::default().eula(&manifest).unwrap();
            assert_eq!(actual, Eula::Generate(Template::Apache2));
        }

        #[test]
        fn eula_with_gpl3_license_field_works() {
            let manifest = GPL3_MANIFEST.parse::<Value>().expect("Parsing TOML");
            let actual = Execution::default().eula(&manifest).unwrap();
            assert_eq!(actual, Eula::Generate(Template::Gpl3));
        }

        #[test]
        fn eula_with_unknown_license_field_works() {
            let manifest = UNKNOWN_MANIFEST.parse::<Value>().expect("Parsing TOML");
            let actual = Execution::default().eula(&manifest).unwrap();
            assert_eq!(actual, Eula::Disabled);
        }

        #[test]
        fn eula_with_override_works() {
            let temp_dir = tempfile::tempdir().unwrap();
            let license_file_path = temp_dir.path().join("Example.rtf");
            let _license_file_handle = File::create(&license_file_path).expect("Create file");
            let manifest = MIT_MANIFEST.parse::<Value>().expect("Parsing TOML");
            let actual = Builder::default()
                .eula(license_file_path.to_str())
                .build()
                .eula(&manifest)
                .unwrap();
            assert_eq!(actual, Eula::CommandLine(license_file_path));
        }

        #[test]
        fn eula_with_license_file_field_works() {
            let temp_dir = tempfile::tempdir().unwrap();
            let license_file_path = temp_dir.path().join("Example.rtf");
            let _license_file_handle = File::create(&license_file_path).expect("Create file");
            let manifest = format!("[package]
                name = \"Example\"
                version = \"0.1.0\"
                authors = [\"First Last <first.last@example.com>\"]
                license-file = {:?}
                ", license_file_path).parse::<Value>().expect("Parsing TOML");
            let actual = Execution::default().eula(&manifest).unwrap();
            assert_eq!(actual, Eula::Manifest(license_file_path));
        }

        #[test]
        fn eula_with_license_file_extension_works() {
            let temp_dir = tempfile::tempdir().unwrap();
            let license_file_path = temp_dir.path().join("Example.txt");
            let _license_file_handle = File::create(&license_file_path).expect("Create file");
            let manifest = format!("[package]
                name = \"Example\"
                version = \"0.1.0\"
                authors = [\"First Last <first.last@example.com>\"]
                license-file = {:?}
                ", license_file_path).parse::<Value>().expect("Parsing TOML");
            let actual = Execution::default().eula(&manifest).unwrap();
            assert_eq!(actual, Eula::Disabled);
        }

        #[test]
        fn eula_with_wrong_file_extension_override_works() {
            let temp_dir = tempfile::tempdir().unwrap();
            let license_file_path = temp_dir.path().join("Example.txt");
            let _license_file_handle = File::create(&license_file_path).expect("Create file");
            let manifest = MIT_MANIFEST.parse::<Value>().expect("Parsing TOML");
            let actual = Builder::default()
                .eula(license_file_path.to_str())
                .build()
                .eula(&manifest)
                .unwrap();
            assert_eq!(actual, Eula::CommandLine(license_file_path));
        }
    }
}
