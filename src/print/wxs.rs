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

//! The implementation for printing a WiX Source (wxs) file.

use crate::description;
use crate::eula::Eula;
use crate::manifest;
use crate::package;
use crate::product_name;
use crate::Error;
use crate::Result;
use crate::Template;
use crate::EXE_FILE_EXTENSION;
use crate::LICENSE_FILE_NAME;
use crate::RTF_FILE_EXTENSION;

use log::{debug, trace, warn};

use mustache::{self, MapBuilder};

use std::path::{Path, PathBuf};
use std::{collections::HashMap, str::FromStr};

use cargo_metadata::Package;

use uuid::Uuid;

/// A builder for creating an execution context to print a WiX Toolset source file (wxs).
#[derive(Debug, Clone)]
pub struct Builder<'a> {
    banner: Option<&'a str>,
    binaries: Option<Vec<&'a str>>,
    description: Option<&'a str>,
    dialog: Option<&'a str>,
    eula: Option<&'a str>,
    help_url: Option<&'a str>,
    input: Option<&'a str>,
    license: Option<&'a str>,
    manufacturer: Option<&'a str>,
    output: Option<&'a str>,
    package: Option<&'a str>,
    path_guid: Option<&'a str>,
    product_icon: Option<&'a str>,
    product_name: Option<&'a str>,
    upgrade_guid: Option<&'a str>,
}

impl<'a> Builder<'a> {
    /// Creates a new `Builder` instance.
    pub fn new() -> Self {
        Builder {
            banner: None,
            binaries: None,
            description: None,
            dialog: None,
            eula: None,
            help_url: None,
            input: None,
            license: None,
            manufacturer: None,
            output: None,
            package: None,
            path_guid: None,
            product_icon: None,
            product_name: None,
            upgrade_guid: None,
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

    /// Sets the path to one or more binaries to include in the installer.
    ///
    /// The default is to first find _all_ binaries defined in the `bin`
    /// sections of the package's manifest (Cargo.toml) and include all of them
    /// in the installer. If there are no `bin` sections, then the `name` field
    /// uder the `package` section is used. This overrides either of these
    /// cases, regardless of the existence and number of `bin` sections.
    ///
    /// A path to each binary should be used for the value. The file stem (file
    /// name without the extension) will be used for each binary's name, while
    /// the path will be used for the source. The binary names will _not_ appear
    /// in the Add/Remove Programs control panel. Use the `product_name` method
    /// to change the name that appears in the Add/Remove Programs control
    /// panel.
    pub fn binaries(&mut self, b: Option<Vec<&'a str>>) -> &mut Self {
        self.binaries = b;
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

    /// Sets the package within a workspace to print a template.
    ///
    /// Each package within a workspace has its own package manifest, i.e.
    /// `Cargo.toml`. This indicates which package manifest within a workspace
    /// should be used to populate a template.
    pub fn package(&mut self, p: Option<&'a str>) -> &mut Self {
        self.package = p;
        self
    }

    /// Sets the GUID for the path component.
    ///
    /// The default automatically generates the GUID needed for the path
    /// component. A GUID is needed so that the path component can be
    /// successfully removed on uninstall.
    ///
    /// Generally, the path component GUID should be generated only once per
    /// project/product and then the same GUID used every time the installer is
    /// created. The GUID is stored in the WiX Source (WXS) file. However,
    /// this allows using an existing GUID, possibly obtained with another tool.
    pub fn path_guid(&mut self, p: Option<&'a str>) -> &mut Self {
        self.path_guid = p;
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

    /// Sets the Upgrade Code GUID.
    ///
    /// The default automatically generates the GUID needed for the `UpgradeCode`
    /// attribute to the `Product` tag. The Upgrade Code uniquely identifies the
    /// installer. It is used to determine if the new installer is the same
    /// product and the current installation should be removed and upgraded to
    /// this version. If the GUIDs of the current product and new product do
    /// _not_ match, then Windows will treat the two installers as separate
    /// products.
    ///
    /// Generally, the upgrade code should be generated only once per
    /// project/product and then the same code used every time the installer is
    /// created. The GUID is stored in the WiX Source (WXS) file. However,
    /// this allows usage of an existing GUID for the upgrade code.
    pub fn upgrade_guid(&mut self, u: Option<&'a str>) -> &mut Self {
        self.upgrade_guid = u;
        self
    }

    /// Builds an execution context for printing a template.
    pub fn build(&self) -> Execution {
        Execution {
            banner: self.banner.map(PathBuf::from),
            binaries: self
                .binaries
                .as_ref()
                .map(|b| b.iter().map(PathBuf::from).collect()),
            description: self.description.map(String::from),
            dialog: self.dialog.map(PathBuf::from),
            eula: self.eula.map(PathBuf::from),
            help_url: self.help_url.map(String::from),
            input: self.input.map(PathBuf::from),
            license: self.license.map(PathBuf::from),
            manufacturer: self.manufacturer.map(String::from),
            output: self.output.map(PathBuf::from),
            package: self.package.map(String::from),
            path_guid: self.path_guid.map(String::from),
            product_icon: self.product_icon.map(PathBuf::from),
            product_name: self.product_name.map(String::from),
            upgrade_guid: self.upgrade_guid.map(String::from),
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
    binaries: Option<Vec<PathBuf>>,
    description: Option<String>,
    dialog: Option<PathBuf>,
    eula: Option<PathBuf>,
    help_url: Option<String>,
    input: Option<PathBuf>,
    license: Option<PathBuf>,
    manufacturer: Option<String>,
    output: Option<PathBuf>,
    package: Option<String>,
    path_guid: Option<String>,
    product_icon: Option<PathBuf>,
    product_name: Option<String>,
    upgrade_guid: Option<String>,
}

impl Execution {
    #[allow(clippy::cognitive_complexity)]
    /// Prints a WiX Source (wxs) file based on the built context.
    pub fn run(self) -> Result<()> {
        debug!("banner = {:?}", self.banner);
        debug!("binaries = {:?}", self.binaries);
        debug!("description = {:?}", self.description);
        debug!("dialog = {:?}", self.description);
        debug!("eula = {:?}", self.eula);
        debug!("help_url = {:?}", self.help_url);
        debug!("input = {:?}", self.input);
        debug!("license = {:?}", self.license);
        debug!("manufacturer = {:?}", self.manufacturer);
        debug!("output = {:?}", self.output);
        debug!("package = {:?}", self.package);
        debug!("path_guid = {:?}", self.path_guid);
        debug!("product_icon = {:?}", self.product_icon);
        debug!("product_name = {:?}", self.product_name);
        debug!("upgrade_guid = {:?}", self.upgrade_guid);
        let manifest = manifest(self.input.as_ref())?;
        let package = package(&manifest, self.package.as_deref())?;
        let mut destination = super::destination(self.output.as_ref())?;
        let template = mustache::compile_str(Template::Wxs.to_str())?;
        let binaries = self.binaries(&package)?;
        let mut map = MapBuilder::new()
            .insert_vec("binaries", |mut builder| {
                for binary in &binaries {
                    builder = builder.push_map(|builder| {
                        builder
                            .insert_str("binary-index", binary.get("binary-index").unwrap())
                            .insert_str("binary-name", binary.get("binary-name").unwrap())
                            .insert_str("binary-source", binary.get("binary-source").unwrap())
                    });
                }
                builder
            })
            .insert_str(
                "product-name",
                product_name(self.product_name.as_ref(), &package)?,
            )
            .insert_str("manufacturer", self.manufacturer(&package)?)
            .insert_str("upgrade-code-guid", self.upgrade_guid(&package)?)
            .insert_str("path-component-guid", self.path_guid(&package)?);
        if let Some(ref banner) = self.banner {
            map = map.insert_str("banner", banner.display().to_string());
        }
        if let Some(description) = description(self.description.clone(), &package) {
            map = map.insert_str("description", description);
        } else {
            warn!(
                "A description was not specified at the command line or in the package's manifest \
                 (Cargo.toml). The description can be added manually to the generated WiX \
                 Source (wxs) file using a text editor."
            );
        }
        if let Some(ref dialog) = self.dialog {
            map = map.insert_str("dialog", dialog.display().to_string());
        }
        match self.eula(&package)? {
            Eula::Disabled => {
                warn!(
                    "An EULA was not specified at the command line, a RTF \
                     license file was not specified in the package manifest's \
                     (Cargo.toml) 'license-file' field, or the license ID from the \
                     package manifest's 'license' field is not recognized. The \
                     license agreement dialog will be excluded from the installer. An \
                     EULA can be added manually to the generated WiX Source (wxs) \
                     file using a text editor."
                );
            }
            e => map = map.insert_str("eula", e.to_string()),
        }
        if let Some(url) = self.help_url(&package) {
            map = map.insert_str("help-url", url);
        } else {
            warn!(
                "A help URL could not be found and it will be excluded from the installer. \
                 A help URL can be added manually to the generated WiX Source (wxs) file \
                 using a text editor."
            );
        }
        if let Some(name) = self.license_name(&package) {
            map = map.insert_str("license-name", name);
        }
        if let Some(source) = self.license_source(&package)? {
            map = map.insert_str("license-source", source);
        } else {
            warn!(
                "A license file could not be found and it will be excluded from the \
                 installer. A license file can be added manually to the generated WiX Source \
                 (wxs) file using a text editor."
            );
        }
        if let Some(icon) = self.product_icon {
            map = map.insert_str("product-icon", icon.display().to_string());
        }
        let data = map.build();
        template
            .render_data(&mut destination, &data)
            .map_err(Error::from)
    }

    fn binaries(&self, package: &Package) -> Result<Vec<HashMap<&'static str, String>>> {
        let mut binaries = Vec::new();
        if let Some(binary_paths) = &self.binaries {
            let mut map = HashMap::with_capacity(3);
            for (index, binary) in binary_paths.iter().enumerate() {
                let binary_file_stem = binary.file_stem().ok_or_else(|| {
                    Error::Generic(format!(
                        "The '{}' binary path does not have a file name",
                        binary.display()
                    ))
                })?;
                map.insert("binary-index", index.to_string());
                map.insert(
                    "binary-name",
                    binary_file_stem.to_string_lossy().into_owned(),
                );
                map.insert("binary-source", binary.to_string_lossy().into_owned());
            }
            binaries.push(map);
        } else {
            let binaries_iter = package
                .targets
                .iter()
                .filter(|v| v.kind.iter().any(|v| v == "bin"));
            for (index, binary) in binaries_iter.enumerate() {
                let mut map = HashMap::with_capacity(3);
                map.insert("binary-index", index.to_string());
                map.insert("binary-name", binary.name.clone());
                map.insert("binary-source", Self::default_binary_path(&binary.name));
                binaries.push(map);
            }
        }
        Ok(binaries)
    }

    fn default_binary_path(name: &str) -> String {
        let mut path = Path::new("$(var.CargoTargetBinDir)").join(name);
        path.set_extension(EXE_FILE_EXTENSION);
        path.to_str()
            .map(String::from)
            .expect("Path to string conversion")
    }

    fn help_url(&self, manifest: &Package) -> Option<String> {
        self.help_url.as_ref()
            .map(String::from)
            .or_else(|| manifest.documentation.clone())
            .or_else(|| manifest.homepage.clone())
            .or_else(|| manifest.repository.clone())
    }

    fn eula(&self, manifest: &Package) -> Result<Eula> {
        if let Some(ref path) = self.eula.clone().map(PathBuf::from) {
            Eula::new(Some(path), manifest)
        } else {
            Eula::new(
                self.license
                    .clone()
                    .map(PathBuf::from)
                    .filter(|p| p.extension().and_then(|p| p.to_str()) == Some(RTF_FILE_EXTENSION))
                    .as_ref(),
                manifest,
            )
        }
    }

    fn license_name(&self, manifest: &Package) -> Option<String> {
        if let Some(ref l) = self.license.clone().map(PathBuf::from) {
            l.file_name().and_then(|f| f.to_str()).map(String::from)
        } else {
            manifest
                .license
                .as_ref()
                .filter(|l| Template::license_ids().contains(&l))
                .map(|_| String::from(LICENSE_FILE_NAME))
                .or_else(|| {
                    manifest
                        .license_file()
                        .and_then(|l| l.file_name().and_then(|f| f.to_str()).map(String::from))
                })
        }
    }

    fn license_source(&self, manifest: &Package) -> Result<Option<String>> {
        if let Some(ref path) = self.license.clone().map(PathBuf::from) {
            Ok(path.to_str().map(String::from))
        } else {
            Ok(manifest
                .license
                .as_ref()
                .filter(|l| Template::license_ids().contains(&l))
                .map(|_| LICENSE_FILE_NAME.to_owned() + "." + RTF_FILE_EXTENSION)
                .or_else(|| {
                    manifest.license_file().and_then(|s| {
                        if s.exists() {
                            trace!(
                                "The '{}' path from the 'license-file' field in the package's \
                                 manifest (Cargo.toml) exists.",
                                s.display()
                            );
                            Some(s.into_os_string().into_string().unwrap())
                        } else {
                            None
                        }
                    })
                }))
        }
    }

    fn manufacturer(&self, manifest: &Package) -> Result<String> {
        if let Some(ref m) = self.manufacturer {
            Ok(m.to_owned())
        } else {
            super::first_author(&manifest)
        }
    }

    fn path_guid(&self, manifest: &Package) -> Result<String> {
        if let Some(ref u) = self.path_guid {
            trace!("An path GUID has been explicitly specified");
            Ok(u.to_owned())
        } else if let Some(pkg_meta_wix_path_guid) = manifest
            .metadata
            .get("wix")
            .and_then(|w| w.as_object())
            .and_then(|t| t.get("path-guid"))
            .and_then(|u| u.as_str())
        {
            Uuid::from_str(pkg_meta_wix_path_guid)
                .map(|u| u.to_hyphenated().to_string().to_uppercase())
                .map_err(Error::from)
        } else {
            Ok(Uuid::new_v4().to_hyphenated().to_string().to_uppercase())
        }
    }

    fn upgrade_guid(&self, manifest: &Package) -> Result<String> {
        if let Some(ref u) = self.upgrade_guid {
            trace!("An upgrade GUID has been explicitly specified");
            Ok(u.to_owned())
        } else if let Some(pkg_meta_wix_upgrade_guid) = manifest
            .metadata
            .get("wix")
            .and_then(|w| w.as_object())
            .and_then(|t| t.get("upgrade-guid"))
            .and_then(|u| u.as_str())
        {
            Uuid::from_str(pkg_meta_wix_upgrade_guid)
                .map(|u| u.to_hyphenated().to_string().to_uppercase())
                .map_err(Error::from)
        } else {
            Ok(Uuid::new_v4().to_hyphenated().to_string().to_uppercase())
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

    use maplit::hashmap;

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
        fn binaries_name_works() {
            const EXPECTED: &str = "bin\\Example.exe";
            let mut actual = Builder::new();
            actual.binaries(Some(vec![EXPECTED]));
            assert_eq!(actual.binaries, Some(vec![EXPECTED]));
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
        fn path_guid_works() {
            let expected = Uuid::new_v4().to_hyphenated().to_string().to_uppercase();
            let mut actual = Builder::new();
            actual.path_guid(Some(&expected));
            assert_eq!(actual.path_guid, Some(expected.as_ref()));
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

        #[test]
        fn upgrade_guid_works() {
            let expected = Uuid::new_v4().to_hyphenated().to_string().to_uppercase();
            let mut actual = Builder::new();
            actual.upgrade_guid(Some(&expected));
            assert_eq!(actual.upgrade_guid, Some(expected.as_ref()));
        }
    }

    mod execution {
        extern crate assert_fs;

        use super::*;
        use crate::tests::setup_project;
        use std::fs::File;

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

        const MULTIPLE_BIN_MANIFEST: &str = r#"[package]
            name = "Example"
            version = "0.1.0"
            authors = ["First Last <first.last@example.com>"]
            license = "MIT"

            [[bin]]
            name = "binary0"
            path = "src/binary0/main.rs"

            [[bin]]
            name = "binary1"
            path = "src/binary1/main.rs"

            [[bin]]
            name = "binary2"
            path = "src/binary2/main.rs"
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

        const LICENSE_FILE_RTF_MANIFEST: &str = r#"[package]
            name = "Example"
            version = "0.1.0"
            authors = ["First Last <first.last@example.com>"]
            license-file = "Example.rtf"
        "#;

        const LICENSE_FILE_TXT_MANIFEST: &str = r#"[package]
            name = "Example"
            version = "0.1.0"
            authors = ["First Last <first.last@example.com>"]
            license-file = "Example.txt"
        "#;

        const PATH_GUID: &str = "C38A18DB-12CC-4BDC-8A05-DFCB981A0F33";
        const UPGRADE_GUID: &str = "71C1A58D-3FD2-493D-BB62-4B27C66FCCF9";

        const PATH_GUID_MANIFEST: &str = r#"[package]
            name = "Example"
            version = "0.1.0"
            authors = ["First Last <first.last@example.com>"]

            [package.metadata.wix]
            path-guid = "C38A18DB-12CC-4BDC-8A05-DFCB981A0F33"
        "#;

        const UPGRADE_GUID_MANIFEST: &str = r#"[package]
            name = "Example"
            version = "0.1.0"
            authors = ["First Last <first.last@example.com>"]

            [package.metadata.wix]
            upgrade-guid = "71C1A58D-3FD2-493D-BB62-4B27C66FCCF9"
        "#;

        #[test]
        fn license_name_with_mit_license_field_works() {
            let project = setup_project(MIT_MANIFEST);
            let manifest = crate::manifest(Some(&project.path().join("Cargo.toml"))).unwrap();
            let package = crate::package(&manifest, None).unwrap();

            let actual = Execution::default()
                .license_name(&package)
                .expect("License name");
            assert_eq!(actual, String::from(LICENSE_FILE_NAME));
        }

        #[test]
        fn license_name_with_gpl3_license_field_works() {
            let project = setup_project(GPL3_MANIFEST);
            let manifest = crate::manifest(Some(&project.path().join("Cargo.toml"))).unwrap();
            let package = crate::package(&manifest, None).unwrap();

            let actual = Execution::default()
                .license_name(&package)
                .expect("License name");
            assert_eq!(actual, String::from(LICENSE_FILE_NAME));
        }

        #[test]
        fn license_name_with_apache2_license_field_works() {
            let project = setup_project(APACHE2_MANIFEST);
            let manifest = crate::manifest(Some(&project.path().join("Cargo.toml"))).unwrap();
            let package = crate::package(&manifest, None).unwrap();

            let actual = Execution::default()
                .license_name(&package)
                .expect("License name");
            assert_eq!(actual, String::from(LICENSE_FILE_NAME));
        }

        #[test]
        fn license_name_with_unknown_license_field_works() {
            let project = setup_project(UNKNOWN_MANIFEST);
            let manifest = crate::manifest(Some(&project.path().join("Cargo.toml"))).unwrap();
            let package = crate::package(&manifest, None).unwrap();

            let actual = Execution::default().license_name(&package);
            assert!(actual.is_none());
        }

        #[test]
        fn license_source_with_mit_license_field_works() {
            let project = setup_project(MIT_MANIFEST);
            let manifest = crate::manifest(Some(&project.path().join("Cargo.toml"))).unwrap();
            let package = crate::package(&manifest, None).unwrap();

            let actual = Execution::default()
                .license_source(&package)
                .expect("License source");
            assert_eq!(
                actual,
                Some(LICENSE_FILE_NAME.to_owned() + "." + RTF_FILE_EXTENSION)
            );
        }

        #[test]
        fn license_source_with_gpl3_license_field_works() {
            let project = setup_project(GPL3_MANIFEST);
            let manifest = crate::manifest(Some(&project.path().join("Cargo.toml"))).unwrap();
            let package = crate::package(&manifest, None).unwrap();

            let actual = Execution::default()
                .license_source(&package)
                .expect("License source");
            assert_eq!(
                actual,
                Some(LICENSE_FILE_NAME.to_owned() + "." + RTF_FILE_EXTENSION)
            );
        }

        #[test]
        fn license_source_with_apache2_license_field_works() {
            let project = setup_project(APACHE2_MANIFEST);
            let manifest = crate::manifest(Some(&project.path().join("Cargo.toml"))).unwrap();
            let package = crate::package(&manifest, None).unwrap();

            let actual = Execution::default()
                .license_source(&package)
                .expect("License source");
            assert_eq!(
                actual,
                Some(LICENSE_FILE_NAME.to_owned() + "." + RTF_FILE_EXTENSION)
            );
        }

        #[test]
        fn license_source_with_unknown_license_field_works() {
            let project = setup_project(UNKNOWN_MANIFEST);
            let manifest = crate::manifest(Some(&project.path().join("Cargo.toml"))).unwrap();
            let package = crate::package(&manifest, None).unwrap();

            let actual = Execution::default().license_source(&package).unwrap();
            assert!(actual.is_none());
        }

        #[test]
        fn binaries_with_no_bin_section_works() {
            let project = setup_project(MIT_MANIFEST);
            let manifest = crate::manifest(Some(&project.path().join("Cargo.toml"))).unwrap();
            let package = crate::package(&manifest, None).unwrap();

            let actual = Execution::default().binaries(&package).unwrap();
            assert_eq!(
                actual,
                vec![hashmap! {
                    "binary-index" => 0.to_string(),
                    "binary-name" => String::from("Example"),
                    "binary-source" => String::from("$(var.CargoTargetBinDir)\\Example.exe")
                }]
            )
        }

        #[test]
        fn binaries_with_single_bin_section_works() {
            let project = setup_project(MIT_MANIFEST_BIN);
            let manifest = crate::manifest(Some(&project.path().join("Cargo.toml"))).unwrap();
            let package = crate::package(&manifest, None).unwrap();

            let actual = Execution::default().binaries(&package).unwrap();
            assert_eq!(
                actual,
                vec![hashmap! {
                    "binary-index" => 0.to_string(),
                    "binary-name" => String::from("Different"),
                    "binary-source" => String::from("$(var.CargoTargetBinDir)\\Different.exe")
                }]
            )
        }

        #[test]
        fn binaries_with_multiple_bin_sections_works() {
            let project = setup_project(MULTIPLE_BIN_MANIFEST);
            let manifest = crate::manifest(Some(&project.path().join("Cargo.toml"))).unwrap();
            let package = crate::package(&manifest, None).unwrap();

            let actual = Execution::default().binaries(&package).unwrap();
            assert_eq!(
                actual,
                vec![
                    hashmap! {
                        "binary-index" => 0.to_string(),
                        "binary-name" => String::from("binary0"),
                        "binary-source" => String::from("$(var.CargoTargetBinDir)\\binary0.exe")
                    },
                    hashmap! {
                        "binary-index" => 1.to_string(),
                        "binary-name" => String::from("binary1"),
                        "binary-source" => String::from("$(var.CargoTargetBinDir)\\binary1.exe")
                    },
                    hashmap! {
                        "binary-index" => 2.to_string(),
                        "binary-name" => String::from("binary2"),
                        "binary-source" => String::from("$(var.CargoTargetBinDir)\\binary2.exe")
                    }
                ]
            )
        }

        #[test]
        fn manufacturer_with_defaults_works() {
            const EXPECTED: &str = "First Last";

            let project = setup_project(MIN_MANIFEST);
            let manifest = crate::manifest(Some(&project.path().join("Cargo.toml"))).unwrap();
            let package = crate::package(&manifest, None).unwrap();

            let actual = Execution::default().manufacturer(&package).unwrap();
            assert_eq!(actual, String::from(EXPECTED));
        }

        #[test]
        fn manufacturer_with_override_works() {
            const EXPECTED: &str = "Example";

            let project = setup_project(MIN_MANIFEST);
            let manifest = crate::manifest(Some(&project.path().join("Cargo.toml"))).unwrap();
            let package = crate::package(&manifest, None).unwrap();

            let actual = Builder::default()
                .manufacturer(Some(EXPECTED))
                .build()
                .manufacturer(&package)
                .unwrap();
            assert_eq!(actual, String::from(EXPECTED));
        }

        #[test]
        fn help_url_with_defaults_works() {
            let project = setup_project(MIN_MANIFEST);
            let manifest = crate::manifest(Some(&project.path().join("Cargo.toml"))).unwrap();
            let package = crate::package(&manifest, None).unwrap();

            let actual = Builder::default().build().help_url(&package);
            assert!(actual.is_none());
        }

        #[test]
        fn help_url_with_documentation_works() {
            const EXPECTED: &str = "http://www.example.com";

            let project = setup_project(DOCUMENTATION_MANIFEST);
            let manifest = crate::manifest(Some(&project.path().join("Cargo.toml"))).unwrap();
            let package = crate::package(&manifest, None).unwrap();

            let actual = Builder::default().build().help_url(&package);
            assert_eq!(actual, Some(String::from(EXPECTED)));
        }

        #[test]
        fn help_url_with_homepage_works() {
            const EXPECTED: &str = "http://www.example.com";

            let project = setup_project(HOMEPAGE_MANIFEST);
            let manifest = crate::manifest(Some(&project.path().join("Cargo.toml"))).unwrap();
            let package = crate::package(&manifest, None).unwrap();

            let actual = Builder::default().build().help_url(&package);
            assert_eq!(actual, Some(String::from(EXPECTED)));
        }

        #[test]
        fn help_url_with_repository_works() {
            const EXPECTED: &str = "http://www.example.com";

            let project = setup_project(REPOSITORY_MANIFEST);
            let manifest = crate::manifest(Some(&project.path().join("Cargo.toml"))).unwrap();
            let package = crate::package(&manifest, None).unwrap();

            let actual = Builder::default().build().help_url(&package);
            assert_eq!(actual, Some(String::from(EXPECTED)));
        }

        #[test]
        fn eula_with_defaults_works() {
            let project = setup_project(MIN_MANIFEST);
            let manifest = crate::manifest(Some(&project.path().join("Cargo.toml"))).unwrap();
            let package = crate::package(&manifest, None).unwrap();

            let actual = Execution::default().eula(&package).unwrap();
            assert_eq!(actual, Eula::Disabled);
        }

        #[test]
        fn eula_with_mit_license_field_works() {
            let project = setup_project(MIT_MANIFEST);
            let manifest = crate::manifest(Some(&project.path().join("Cargo.toml"))).unwrap();
            let package = crate::package(&manifest, None).unwrap();

            let actual = Execution::default().eula(&package).unwrap();
            assert_eq!(actual, Eula::Generate(Template::Mit));
        }

        #[test]
        fn eula_with_apache2_license_field_works() {
            let project = setup_project(APACHE2_MANIFEST);
            let manifest = crate::manifest(Some(&project.path().join("Cargo.toml"))).unwrap();
            let package = crate::package(&manifest, None).unwrap();

            let actual = Execution::default().eula(&package).unwrap();
            assert_eq!(actual, Eula::Generate(Template::Apache2));
        }

        #[test]
        fn eula_with_gpl3_license_field_works() {
            let project = setup_project(GPL3_MANIFEST);
            let manifest = crate::manifest(Some(&project.path().join("Cargo.toml"))).unwrap();
            let package = crate::package(&manifest, None).unwrap();

            let actual = Execution::default().eula(&package).unwrap();
            assert_eq!(actual, Eula::Generate(Template::Gpl3));
        }

        #[test]
        fn eula_with_unknown_license_field_works() {
            let project = setup_project(UNKNOWN_MANIFEST);
            let manifest = crate::manifest(Some(&project.path().join("Cargo.toml"))).unwrap();
            let package = crate::package(&manifest, None).unwrap();

            let actual = Execution::default().eula(&package).unwrap();
            assert_eq!(actual, Eula::Disabled);
        }

        #[test]
        fn eula_with_override_works() {
            let project = setup_project(MIT_MANIFEST);
            let license_file_path = project.path().join("Example.rtf");
            let _license_file_handle = File::create(&license_file_path).expect("Create file");

            let manifest = crate::manifest(Some(&project.path().join("Cargo.toml"))).unwrap();
            let package = crate::package(&manifest, None).unwrap();

            let actual = Builder::default()
                .eula(license_file_path.to_str())
                .build()
                .eula(&package)
                .unwrap();
            assert_eq!(actual, Eula::CommandLine(license_file_path));
        }

        #[test]
        fn eula_with_license_file_field_works() {
            let project = setup_project(LICENSE_FILE_RTF_MANIFEST);
            let license_file_path = project.path().join("Example.rtf");
            let _license_file_handle = File::create(&license_file_path).expect("Create file");

            let manifest = crate::manifest(Some(&project.path().join("Cargo.toml"))).unwrap();
            let package = crate::package(&manifest, None).unwrap();

            let actual = Execution::default().eula(&package).unwrap();
            assert_eq!(actual, Eula::Manifest(license_file_path));
        }

        #[test]
        fn eula_with_license_file_extension_works() {
            let project = setup_project(LICENSE_FILE_TXT_MANIFEST);
            let license_file_path = project.path().join("Example.txt");
            let _license_file_handle = File::create(&license_file_path).expect("Create file");

            let manifest = crate::manifest(Some(&project.path().join("Cargo.toml"))).unwrap();
            let package = crate::package(&manifest, None).unwrap();

            let actual = Execution::default().eula(&package).unwrap();
            assert_eq!(actual, Eula::Disabled);
        }

        #[test]
        fn eula_with_wrong_file_extension_override_works() {
            let project = setup_project(LICENSE_FILE_TXT_MANIFEST);
            let license_file_path = project.path().join("Example.txt");
            let _license_file_handle = File::create(&license_file_path).expect("Create file");

            let manifest = crate::manifest(Some(&project.path().join("Cargo.toml"))).unwrap();
            let package = crate::package(&manifest, None).unwrap();

            let actual = Builder::default()
                .eula(license_file_path.to_str())
                .build()
                .eula(&package)
                .unwrap();
            assert_eq!(actual, Eula::CommandLine(license_file_path));
        }

        #[test]
        fn path_guid_with_override_works() {
            let expected = Uuid::new_v4().to_hyphenated().to_string().to_uppercase();

            let project = setup_project(MIN_MANIFEST);
            let manifest = crate::manifest(Some(&project.path().join("Cargo.toml"))).unwrap();
            let package = crate::package(&manifest, None).unwrap();

            let actual = Builder::default()
                .path_guid(Some(&expected))
                .build()
                .path_guid(&package)
                .unwrap();
            assert_eq!(actual, expected);
        }

        #[test]
        fn path_guid_metadata_works() {
            let project = setup_project(PATH_GUID_MANIFEST);
            let manifest = crate::manifest(Some(&project.path().join("Cargo.toml"))).unwrap();
            let package = crate::package(&manifest, None).unwrap();

            let actual = Builder::default().build().path_guid(&package).unwrap();
            assert_eq!(actual, PATH_GUID);
        }

        #[test]
        fn path_guid_metadata_and_override_works() {
            let expected = Uuid::new_v4().to_hyphenated().to_string().to_uppercase();

            let project = setup_project(PATH_GUID_MANIFEST);
            let manifest = crate::manifest(Some(&project.path().join("Cargo.toml"))).unwrap();
            let package = crate::package(&manifest, None).unwrap();

            let actual = Builder::default()
                .path_guid(Some(&expected))
                .build()
                .path_guid(&package)
                .unwrap();
            assert_eq!(actual, expected);
        }

        #[test]
        fn upgrade_guid_with_override_works() {
            let expected = Uuid::new_v4().to_hyphenated().to_string().to_uppercase();

            let project = setup_project(MIN_MANIFEST);
            let manifest = crate::manifest(Some(&project.path().join("Cargo.toml"))).unwrap();
            let package = crate::package(&manifest, None).unwrap();

            let actual = Builder::default()
                .upgrade_guid(Some(&expected))
                .build()
                .upgrade_guid(&package)
                .unwrap();
            assert_eq!(actual, expected);
        }

        #[test]
        fn upgrade_guid_metadata_works() {
            let project = setup_project(UPGRADE_GUID_MANIFEST);
            let manifest = crate::manifest(Some(&project.path().join("Cargo.toml"))).unwrap();
            let package = crate::package(&manifest, None).unwrap();

            let actual = Builder::default().build().upgrade_guid(&package).unwrap();
            assert_eq!(actual, UPGRADE_GUID);
        }

        #[test]
        fn upgrade_guid_metadata_and_override_works() {
            let expected = Uuid::new_v4().to_hyphenated().to_string().to_uppercase();

            let project = setup_project(UPGRADE_GUID_MANIFEST);
            let manifest = crate::manifest(Some(&project.path().join("Cargo.toml"))).unwrap();
            let package = crate::package(&manifest, None).unwrap();

            let actual = Builder::default()
                .upgrade_guid(Some(&expected))
                .build()
                .upgrade_guid(&package)
                .unwrap();
            assert_eq!(actual, expected);
        }
    }
}
