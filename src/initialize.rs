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

//! The implementation for the `init` command. The `init` command for the `cargo
//! wix` subcommand is focused on creating a WiX Source file (wxs) based on the
//! contents of the Cargo manifest file (Cargo.toml) for the project and any
//! run-time based settings.
//!
//! The `init` command should generally be called before any other commands and
//! it should only be called once per project. Once a WiX Source file (wxs)
//! exists for the project, the `init` command does not need to be executed
//! again.

use cargo_metadata::Package;

use crate::eula::Eula;
use crate::print;
use crate::Error;
use crate::Result;
use crate::LICENSE_FILE_NAME;
use crate::RTF_FILE_EXTENSION;
use crate::WIX;
use crate::WIX_SOURCE_FILE_EXTENSION;
use crate::WIX_SOURCE_FILE_NAME;

use std::fs;
use std::path::{Path, PathBuf};

/// A builder for running the `cargo wix init` subcommand.
#[derive(Debug, Clone)]
pub struct Builder<'a> {
    banner: Option<&'a str>,
    binaries: Option<Vec<&'a str>>,
    copyright_year: Option<&'a str>,
    copyright_holder: Option<&'a str>,
    description: Option<&'a str>,
    dialog: Option<&'a str>,
    eula: Option<&'a str>,
    force: bool,
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
            copyright_year: None,
            copyright_holder: None,
            description: None,
            dialog: None,
            eula: None,
            force: false,
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

    /// Sets the path to the binaries.
    ///
    /// The default is to, first, collect all of the `bin` sections and use the
    /// `name` field within each `bin` section of the package's manifest for
    /// each binary's name and create the following source with the `.exe` file
    /// extension: `target\release\<binary-name>.exe`, where `<binary-name>` is
    /// replaced with the name obtained from each `bin` section. All binaries
    /// are included in the installer. If no `bin` sections exist, then the
    /// package's `name` field is used and only one binary is included in the
    /// installer.
    ///
    /// This method skips creating the binary names and sources from the
    /// package's manifest (Cargo.toml) and uses the supplied paths, regardless
    /// of the number of `bin` sections in the package's manifest. The binary
    /// name is extracted from each supplied path as the file stem (file name
    /// without extension).
    ///
    /// This method is useful for including binaries, a.k.a. executables, in the
    /// installer that are necessary for the application to run but are not
    /// necessarily Rust/Cargo built binaries. However, this method overrides
    /// _all_ binaries in the Cargo-based project, so if the installer is to
    /// include a mixture of external and internal binaries, the internal
    /// binaries must be explicitly included in this method.
    pub fn binaries(&mut self, b: Option<Vec<&'a str>>) -> &mut Self {
        self.binaries = b;
        self
    }

    /// Sets the copyright holder for the generated license file and EULA.
    ///
    /// The default is to use the first author from the `authors` field of the
    /// package's manifest (Cargo.toml). This method can be used to override the
    /// default and set a different copyright holder if and when a Rich Text
    /// Format (RTF) license and EULA are generated based on the value of the
    /// `license` field in the package's manifest (Cargo.toml).
    ///
    /// This value is ignored and not used if an EULA is set with the [`eula`]
    /// method, if a custom EULA is set using the `license-file` field in the
    /// package's manfiest (Cargo.toml), or an EULA is _not_ generated from the
    /// `license` field in the package's manifest (Cargo.toml).
    ///
    /// ['eula']: https://volks73.github.io/cargo-wix/cargo_wix/initialize.html#eula
    pub fn copyright_holder(&mut self, h: Option<&'a str>) -> &mut Self {
        self.copyright_holder = h;
        self
    }

    /// Sets the copyright year for the generated license file and EULA.
    ///
    /// The default is to use the current year. This method can be used to
    /// override the default and set a specific year if and when a Rich Text
    /// Format (RTF) license and EULA are generated based on the value of the
    /// `license` field in the package's manifest (Cargo.toml).
    ///
    /// This value is ignored and not used if an EULA is set with the [`eula`]
    /// method, if a custom EULA is set using the `license-file` field in the
    /// package's manfiest (Cargo.toml), or an EULA is _not_ generated from the
    /// `license` field in the package's manifest (Cargo.toml).
    ///
    /// ['eula']: https://volks73.github.io/cargo-wix/cargo_wix/initialize.html#eula
    pub fn copyright_year(&mut self, y: Option<&'a str>) -> &mut Self {
        self.copyright_year = y;
        self
    }

    /// Sets the description.
    ///
    /// This overrides the description determined from the `description` field
    /// in the package's manifest (Cargo.toml).
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

    /// Sets the path to a custom End User License Agreement (EULA).
    ///
    /// The EULA is the text that appears in the license agreement dialog of the
    /// installer, where a checkbox is present for the user to agree to the
    /// terms of the license. Typically, this is the same as the license file
    /// that is included as a [sidecar] file in the installation destination of
    /// the executable.
    ///
    /// The default is to generate an EULA from an embedded template as a RTF
    /// file based on the name of the license specified in the `license` field
    /// of the package's manifest (Cargo.toml). This method can be used to
    /// override the default and specify a custom EULA. A custom EULA must be in
    /// the RTF format and have the `.rtf` file extension.
    ///
    /// If the `license` field is not specified or a template for the license
    /// does not exist but the `license-file` field does specify a path to a
    /// file with the RTF extension, then that RTF file is used as the EULA for
    /// the license agreement dialog in the installer. Finally, if the
    /// `license-file` does not exist or it specifies a file that does not have
    /// the `.rtf` extension, then the license agreement dialog is skipped and
    /// there is no EULA for the installer. This would override the default
    /// behavior and ensure the license agreement dialog is used.
    pub fn eula(&mut self, e: Option<&'a str>) -> &mut Self {
        self.eula = e;
        self
    }

    /// Forces the generation of new output even if the various outputs already
    /// exists at the destination.
    pub fn force(&mut self, f: bool) -> &mut Self {
        self.force = f;
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
    ///
    /// The help URL is the URL that appears in the Add/Remove Program control
    /// panel, a.k.a. `ARPHELPLINK`.
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

    /// Sets the path to a file to be used as the [sidecar] license file.
    ///
    /// This will override the `license-file` field in the package's manifest
    /// (Cargo.toml). If the file has the `.rtf` extension, then it will also be
    /// used for the EULA in the license agreement dialog for the installer.
    /// Otherwise, the [`eula`] method can be used to set an RTF file as the
    /// EULA for the license agreement dialog that is indepenent of the sidecar
    /// license file.
    ///
    /// The default is to use the value specified in the `license-file` field of
    /// the package's manifest or generate a license file and EULA from an
    /// embedded template based on the license ID used in the `license` field
    /// of the package's manifest. If none of these fields are specified or
    /// overriden, then a license file is _not_ included in the installation
    /// directory and the license agreement dialog is skipped in the installer.
    ///
    /// [sidecar]: https://en.wikipedia.org/wiki/Sidecar_file
    /// [`eula`]: #eula
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
    /// The default is to create all initialization output in the same folder as
    /// the package's manifest (Cargo.toml). Thus, a `wix` folder will be
    /// created within the same folder as the `Cargo.toml` file and all
    /// initialization created files will be placed in the `wix` folder.
    ///
    /// This method can be used to override the default output destination and
    /// have the files related to creating an installer placed in a different
    /// location inside or outside of the package's project folder.
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

    /// Sets the package within a workspace to initialize an installer.
    ///
    /// Each package within a workspace has its own package manifest, i.e.
    /// `Cargo.toml`. This indicates within package manifest within a workspace
    /// should be used when initializing an installer.
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

    /// Sets the product name.
    ///
    /// The default is to use the `name` field under the `package` section of
    /// the package's manifest (Cargo.toml). This overrides that value. An error
    /// occurs if the `name` field is not found in the manifest.
    ///
    /// The product name is also used for the disk prompt during installation
    /// and the name of the default installation destination. For example, a
    /// product anme of `Example` will have an installation destination of
    /// `C:\Program Files\Example` as the default during installation.
    pub fn product_name(&mut self, p: Option<&'a str>) -> &mut Self {
        self.product_name = p;
        self
    }

    /// Sets the Upgrade Code GUID.
    ///
    /// The default automatically generates the need GUID for the `UpgradeCode`
    /// attribute to the `Product` tag. The Upgrade Code uniquely identifies the
    /// installer. It is used to determine if the new installer is the same
    /// product and the current installation should be removed and upgraded to
    /// this version. If the GUIDs of the current product and new product do
    /// _not_ match, then Windows will treat the two installers as separate
    /// products.
    ///
    /// Generally, the upgrade code should be generated only one per
    /// project/product and then the same code used every time the installer is
    /// created and the GUID is stored in the WiX Source (WXS) file. However,
    /// this allows the user to provide an existing GUID for the upgrade code.
    pub fn upgrade_guid(&mut self, u: Option<&'a str>) -> &mut Self {
        self.upgrade_guid = u;
        self
    }

    /// Builds a read-only initialization execution.
    pub fn build(&mut self) -> Execution {
        // let mut wxs_printer = print::wxs::Builder::new();
        // wxs_printer.binaries(self.binaries);
        Execution {
            banner: self.banner.map(PathBuf::from),
            binaries: self
                .binaries
                .as_ref()
                .map(|b| b.iter().map(PathBuf::from).collect()),
            copyright_year: self.copyright_year.map(String::from),
            copyright_holder: self.copyright_holder.map(String::from),
            description: self.description.map(String::from),
            dialog: self.dialog.map(PathBuf::from),
            eula: self.eula.map(PathBuf::from),
            force: self.force,
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

/// A context for creating the necessary files to eventually build an installer.
#[derive(Debug)]
pub struct Execution {
    banner: Option<PathBuf>,
    binaries: Option<Vec<PathBuf>>,
    copyright_holder: Option<String>,
    copyright_year: Option<String>,
    description: Option<String>,
    dialog: Option<PathBuf>,
    eula: Option<PathBuf>,
    force: bool,
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
    /// Generates the necessary files to eventually create, or build, an
    /// installer based on a built context.
    pub fn run(self) -> Result<()> {
        debug!("banner = {:?}", self.banner);
        debug!("binaries = {:?}", self.binaries);
        debug!("copyright_holder = {:?}", self.copyright_holder);
        debug!("copyright_year = {:?}", self.copyright_year);
        debug!("description = {:?}", self.description);
        debug!("dialog = {:?}", self.dialog);
        debug!("eula = {:?}", self.eula);
        debug!("force = {:?}", self.force);
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
        let manifest = super::manifest(self.input.as_ref())?;
        let package = super::package(&manifest, self.package.as_deref())?;
        let mut destination = self.destination(&package)?;
        debug!("destination = {:?}", destination);
        if !destination.exists() {
            info!("Creating the '{}' directory", destination.display());
            fs::create_dir(&destination)?;
        }
        let (eula_wxs_path, license_wxs_path) = match Eula::new(self.eula.as_ref(), &package)? {
            Eula::CommandLine(path) => (Some(path), self.license),
            Eula::Manifest(path) => (Some(path), self.license),
            Eula::Generate(template) => {
                destination.push(LICENSE_FILE_NAME);
                destination.set_extension(RTF_FILE_EXTENSION);
                if destination.exists() && !self.force {
                    return Err(Error::already_exists(&destination));
                } else {
                    info!("Generating an EULA");
                    let mut eula_printer = print::license::Builder::new();
                    eula_printer
                        .copyright_holder(self.copyright_holder.as_ref().map(String::as_ref));
                    eula_printer.copyright_year(self.copyright_year.as_ref().map(String::as_ref));
                    eula_printer.input(self.input.as_deref().and_then(Path::to_str));
                    eula_printer.output(destination.as_path().to_str());
                    eula_printer.build().run(template)?;
                }
                destination.pop();
                let mut relative = destination
                    .strip_prefix(&super::package_root(self.input.as_ref())?)?
                    .to_owned();
                relative.push(LICENSE_FILE_NAME);
                relative.set_extension(RTF_FILE_EXTENSION);
                (Some(relative.clone()), Some(relative))
            }
            Eula::Disabled => (None, self.license),
        };
        debug!("eula_wxs_path = {:?}", eula_wxs_path);
        destination.push(WIX_SOURCE_FILE_NAME);
        destination.set_extension(WIX_SOURCE_FILE_EXTENSION);
        if destination.exists() && !self.force {
            return Err(Error::already_exists(&destination));
        } else {
            info!("Creating the '{}' file", destination.display());
            let mut wxs_printer = print::wxs::Builder::new();
            wxs_printer.banner(self.banner.as_deref().and_then(Path::to_str));
            wxs_printer.binaries(self.binaries.as_ref().map(|b| {
                b.iter()
                    .map(PathBuf::as_path)
                    .map(|p| p.to_str().unwrap())
                    .collect()
            }));
            wxs_printer.description(self.description.as_ref().map(String::as_ref));
            wxs_printer.dialog(self.dialog.as_deref().and_then(Path::to_str));
            wxs_printer.eula(eula_wxs_path.as_deref().and_then(Path::to_str));
            wxs_printer.help_url(self.help_url.as_ref().map(String::as_ref));
            wxs_printer.input(self.input.as_deref().and_then(Path::to_str));
            wxs_printer.license(license_wxs_path.as_deref().and_then(Path::to_str));
            wxs_printer.manufacturer(self.manufacturer.as_ref().map(String::as_ref));
            wxs_printer.output(destination.as_path().to_str());
            wxs_printer.package(self.package.as_deref());
            wxs_printer.path_guid(self.path_guid.as_ref().map(String::as_ref));
            wxs_printer.product_icon(self.product_icon.as_deref().and_then(Path::to_str));
            wxs_printer.product_name(self.product_name.as_ref().map(String::as_ref));
            wxs_printer.upgrade_guid(self.upgrade_guid.as_ref().map(String::as_ref));
            wxs_printer.build().run()?;
        }
        Ok(())
    }

    fn destination(&self, package: &Package) -> Result<PathBuf> {
        if let Some(ref output) = self.output {
            trace!("An output path has been explicitly specified");
            Ok(output.to_owned())
        } else {
            trace!("An output path has NOT been explicitly specified. Implicitly determine output from manifest location.");
            Ok(package
                .manifest_path
                .parent()
                .map(|p| p.to_path_buf())
                .map(|mut p| {
                    p.push(WIX);
                    p
                })
                .unwrap())
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

        const UPGRADE_GUID: &str = "0631BBDF-4079-4C20-823F-7EA8DE40BF08";

        #[test]
        fn defaults_are_correct() {
            let actual = Builder::new();
            assert!(actual.banner.is_none());
            assert!(actual.binaries.is_none());
            assert!(actual.copyright_year.is_none());
            assert!(actual.copyright_holder.is_none());
            assert!(actual.description.is_none());
            assert!(actual.dialog.is_none());
            assert!(actual.eula.is_none());
            assert!(!actual.force);
            assert!(actual.help_url.is_none());
            assert!(actual.input.is_none());
            assert!(actual.license.is_none());
            assert!(actual.manufacturer.is_none());
            assert!(actual.output.is_none());
            assert!(actual.product_icon.is_none());
            assert!(actual.product_name.is_none());
            assert!(actual.upgrade_guid.is_none());
        }

        #[test]
        fn banner_works() {
            const EXPECTED: &str = "img\\Banner.bmp";
            let mut actual = Builder::new();
            actual.banner(Some(EXPECTED));
            assert_eq!(actual.banner, Some(EXPECTED));
        }

        #[test]
        fn binaries_works() {
            const EXPECTED: &str = "bin\\Example.exe";
            let mut actual = Builder::new();
            actual.binaries(Some(vec![EXPECTED]));
            assert_eq!(actual.binaries, Some(vec![EXPECTED]));
        }

        #[test]
        fn copyright_holder_works() {
            const EXPECTED: &str = "holder";
            let mut actual = Builder::new();
            actual.copyright_holder(Some(EXPECTED));
            assert_eq!(actual.copyright_holder, Some(EXPECTED));
        }

        #[test]
        fn copyright_year_works() {
            const EXPECTED: &str = "2018";
            let mut actual = Builder::new();
            actual.copyright_year(Some(EXPECTED));
            assert_eq!(actual.copyright_year, Some(EXPECTED));
        }

        #[test]
        fn description_works() {
            const EXPECTED: &str = "description";
            let mut actual = Builder::new();
            actual.description(Some(EXPECTED));
            assert_eq!(actual.description, Some(EXPECTED));
        }

        #[test]
        fn dialog_works() {
            const EXPECTED: &str = "img\\Dialog.bmp";
            let mut actual = Builder::new();
            actual.dialog(Some(EXPECTED));
            assert_eq!(actual.dialog, Some(EXPECTED));
        }

        #[test]
        fn eula_works() {
            const EXPECTED: &str = "eula.rtf";
            let mut actual = Builder::new();
            actual.eula(Some(EXPECTED));
            assert_eq!(actual.eula, Some(EXPECTED));
        }

        #[test]
        fn force_works() {
            let mut actual = Builder::new();
            actual.force(true);
            assert!(actual.force);
        }

        #[test]
        fn help_url_works() {
            const EXPECTED: &str = "http://github.com/volks73/cargo-wix";
            let mut actual = Builder::new();
            actual.help_url(Some(EXPECTED));
            assert_eq!(actual.help_url, Some(EXPECTED));
        }

        #[test]
        fn input_works() {
            const EXPECTED: &str = "input.wxs";
            let mut actual = Builder::new();
            actual.input(Some(EXPECTED));
            assert_eq!(actual.input, Some(EXPECTED));
        }

        #[test]
        fn license_works() {
            const EXPECTED: &str = "License.txt";
            let mut actual = Builder::new();
            actual.license(Some(EXPECTED));
            assert_eq!(actual.license, Some(EXPECTED));
        }

        #[test]
        fn manufacturer_works() {
            const EXPECTED: &str = "manufacturer";
            let mut actual = Builder::new();
            actual.manufacturer(Some(EXPECTED));
            assert_eq!(actual.manufacturer, Some(EXPECTED));
        }

        #[test]
        fn output_works() {
            const EXPECTED: &str = "output";
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
            const EXPECTED: &str = "product name";
            let mut actual = Builder::new();
            actual.product_name(Some(EXPECTED));
            assert_eq!(actual.product_name, Some(EXPECTED));
        }

        #[test]
        fn upgrade_code_works() {
            let mut actual = Builder::new();
            actual.upgrade_guid(Some(UPGRADE_GUID));
            assert_eq!(actual.upgrade_guid, Some(UPGRADE_GUID));
        }

        #[test]
        fn build_with_defaults_works() {
            let mut b = Builder::new();
            let default_execution = b.build();
            assert!(default_execution.binaries.is_none());
            assert!(default_execution.copyright_year.is_none());
            assert!(default_execution.copyright_holder.is_none());
            assert!(default_execution.description.is_none());
            assert!(default_execution.eula.is_none());
            assert!(!default_execution.force);
            assert!(default_execution.help_url.is_none());
            assert!(default_execution.input.is_none());
            assert!(default_execution.license.is_none());
            assert!(default_execution.manufacturer.is_none());
            assert!(default_execution.output.is_none());
            assert!(default_execution.product_icon.is_none());
            assert!(default_execution.product_name.is_none());
            assert!(default_execution.upgrade_guid.is_none());
        }

        #[test]
        fn build_with_all_works() {
            const EXPECTED_BINARY: &str = "bin\\Example.exe";
            const EXPECTED_COPYRIGHT_HOLDER: &str = "Copyright Holder";
            const EXPECTED_COPYRIGHT_YEAR: &str = "Copyright Year";
            const EXPECTED_DESCRIPTION: &str = "Description";
            const EXPECTED_EULA: &str = "C:\\tmp\\eula.rtf";
            const EXPECTED_URL: &str = "http://github.com/volks73/cargo-wix";
            const EXPECTED_INPUT: &str = "C:\\tmp\\hello_world";
            const EXPECTED_LICENSE: &str = "C:\\tmp\\hello_world\\License.rtf";
            const EXPECTED_MANUFACTURER: &str = "Manufacturer";
            const EXPECTED_OUTPUT: &str = "C:\\tmp\\output";
            const EXPECTED_PRODUCT_ICON: &str = "img\\Product.ico";
            const EXPECTED_PRODUCT_NAME: &str = "Product Name";
            let mut b = Builder::new();
            b.binaries(Some(vec![EXPECTED_BINARY]));
            b.copyright_holder(Some(EXPECTED_COPYRIGHT_HOLDER));
            b.copyright_year(Some(EXPECTED_COPYRIGHT_YEAR));
            b.description(Some(EXPECTED_DESCRIPTION));
            b.eula(Some(EXPECTED_EULA));
            b.force(true);
            b.help_url(Some(EXPECTED_URL));
            b.input(Some(EXPECTED_INPUT));
            b.license(Some(EXPECTED_LICENSE));
            b.manufacturer(Some(EXPECTED_MANUFACTURER));
            b.output(Some(EXPECTED_OUTPUT));
            b.product_icon(Some(EXPECTED_PRODUCT_ICON));
            b.product_name(Some(EXPECTED_PRODUCT_NAME));
            b.upgrade_guid(Some(UPGRADE_GUID));
            let execution = b.build();
            assert_eq!(
                execution.binaries,
                Some(vec![EXPECTED_BINARY]).map(|s| s.iter().map(PathBuf::from).collect())
            );
            assert_eq!(
                execution.copyright_year,
                Some(EXPECTED_COPYRIGHT_YEAR).map(String::from)
            );
            assert_eq!(
                execution.copyright_holder,
                Some(EXPECTED_COPYRIGHT_HOLDER).map(String::from)
            );
            assert_eq!(
                execution.description,
                Some(EXPECTED_DESCRIPTION).map(String::from)
            );
            assert_eq!(execution.eula, Some(EXPECTED_EULA).map(PathBuf::from));
            assert!(execution.force);
            assert_eq!(execution.help_url, Some(EXPECTED_URL).map(String::from));
            assert_eq!(execution.input, Some(EXPECTED_INPUT).map(PathBuf::from));
            assert_eq!(execution.license, Some(EXPECTED_LICENSE).map(PathBuf::from));
            assert_eq!(
                execution.manufacturer,
                Some(EXPECTED_MANUFACTURER).map(String::from)
            );
            assert_eq!(execution.output, Some(EXPECTED_OUTPUT).map(PathBuf::from));
            assert_eq!(
                execution.product_icon,
                Some(EXPECTED_PRODUCT_ICON).map(PathBuf::from)
            );
            assert_eq!(
                execution.product_name,
                Some(EXPECTED_PRODUCT_NAME).map(String::from)
            );
            assert_eq!(execution.upgrade_guid, Some(UPGRADE_GUID).map(String::from));
        }
    }

    mod execution {
        extern crate assert_fs;

        use super::*;
        use std::env;

        const MIN_PACKAGE: &str = r#"[package]
        name = "cargowixtest"
        version = "1.0.0"
        "#;

        #[test]
        fn destination_is_correct_with_defaults() {
            let original = env::current_dir().unwrap();
            let temp_dir = crate::tests::setup_project(MIN_PACKAGE);
            env::set_current_dir(temp_dir.path()).unwrap();
            let mut expected = env::current_dir().unwrap();
            expected.push(WIX);
            let e = Execution::default();

            let result = crate::manifest(None)
                .and_then(|manifest| crate::package(&manifest, None))
                .and_then(|package| e.destination(&package));

            env::set_current_dir(original).unwrap();
            let actual = result.unwrap();
            assert_eq!(actual, expected);
        }

        #[test]
        fn destination_is_correct_with_output() {
            let expected = PathBuf::from("output");

            let temp_dir = crate::tests::setup_project(MIN_PACKAGE);

            let mut e = Execution::default();
            e.output = Some(expected.clone());

            let actual = crate::manifest(Some(&temp_dir.path().join("Cargo.toml")))
                .and_then(|manifest| crate::package(&manifest, None))
                .and_then(|package| e.destination(&package))
                .unwrap();

            assert_eq!(actual, expected);
        }
    }
}
