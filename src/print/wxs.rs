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
use crate::eula::License;
use crate::eula::Licenses;
use crate::manifest;
use crate::package;
use crate::product_name;
use crate::Error;
use crate::Result;
use crate::StoredPathBuf;
use crate::Template;
use crate::EXE_FILE_EXTENSION;

use camino::Utf8Path;
use log::{debug, trace, warn};

use mustache::{self, MapBuilder};

use std::path::Path;
use std::{collections::HashMap, str::FromStr};

use cargo_metadata::Package;

use uuid::Uuid;

use super::RenderOutput;

/// A builder for creating an execution context to print a WiX Toolset source file (wxs).
#[derive(Debug, Clone)]
pub struct Builder<'a> {
    banner: Option<&'a str>,
    binaries: Option<Vec<&'a str>>,
    copyright_year: Option<&'a str>,
    copyright_holder: Option<&'a str>,
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
            copyright_year: None,
            copyright_holder: None,
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

    /// Sets the copyright holder for the generated license file and EULA.
    ///
    /// The default is to use the `authors` field of the
    /// package's manifest (Cargo.toml). This method can be used to override the
    /// default and set a different copyright holder if and when a Rich Text
    /// Format (RTF) license and EULA are generated based on the value of the
    /// `license` field in the package's manifest (Cargo.toml).
    ///
    /// This value is ignored and not used if an EULA is set with the [`eula`]
    /// method, if a custom EULA is set using the `license-file` field in the
    /// package's manifest (Cargo.toml), or an EULA is _not_ generated from the
    /// `license` field in the package's manifest (Cargo.toml).
    ///
    /// [`eula`]: https://volks73.github.io/cargo-wix/cargo_wix/initialize.html#eula
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
    /// package's manifest (Cargo.toml), or an EULA is _not_ generated from the
    /// `license` field in the package's manifest (Cargo.toml).
    ///
    /// [`eula`]: https://volks73.github.io/cargo-wix/cargo_wix/initialize.html#eula
    pub fn copyright_year(&mut self, y: Option<&'a str>) -> &mut Self {
        self.copyright_year = y;
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
    /// `repository`. If none of these are specified, then the default is to
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
    /// are specified or overridden, then a license sidecar file is _not_
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
    /// Default is to use the `authors` field of the
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
            banner: self.banner.map(StoredPathBuf::from),
            binaries: self
                .binaries
                .as_ref()
                .map(|b| b.iter().copied().map(StoredPathBuf::from).collect()),
            copyright_holder: self.copyright_holder.map(String::from),
            copyright_year: self.copyright_year.map(String::from),
            description: self.description.map(String::from),
            dialog: self.dialog.map(StoredPathBuf::from),
            eula: self.eula.map(StoredPathBuf::from),
            help_url: self.help_url.map(String::from),
            input: self.input.map(std::path::PathBuf::from),
            license: self.license.map(StoredPathBuf::from),
            manufacturer: self.manufacturer.map(String::from),
            output: self.output.map(std::path::PathBuf::from),
            package: self.package.map(String::from),
            path_guid: self.path_guid.map(String::from),
            product_icon: self.product_icon.map(StoredPathBuf::from),
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
    banner: Option<StoredPathBuf>,
    binaries: Option<Vec<StoredPathBuf>>,
    copyright_holder: Option<String>,
    copyright_year: Option<String>,
    description: Option<String>,
    dialog: Option<StoredPathBuf>,
    eula: Option<StoredPathBuf>,
    help_url: Option<String>,
    input: Option<std::path::PathBuf>,
    license: Option<StoredPathBuf>,
    manufacturer: Option<String>,
    output: Option<std::path::PathBuf>,
    package: Option<String>,
    path_guid: Option<String>,
    product_icon: Option<StoredPathBuf>,
    product_name: Option<String>,
    upgrade_guid: Option<String>,
}

pub struct WxsRenders {
    pub wxs: RenderOutput,
    pub license: Option<RenderOutput>,
    pub eula: Option<RenderOutput>,
}

impl Execution {
    /// Prints a WiX Source (wxs) file based on the built context.
    pub fn run(self) -> Result<()> {
        let renders = self.render()?;
        renders.wxs.write()?;
        if let Some(license) = renders.license {
            license.write_disk_only()?;
        }
        if let Some(eula) = renders.eula {
            eula.write_disk_only()?;
        }
        Ok(())
    }

    /// Instead of printing the output like [`Execution::run`][], return it as a String
    pub fn render(self) -> Result<WxsRenders> {
        debug!("banner = {:?}", self.banner);
        debug!("binaries = {:?}", self.binaries);
        debug!("copyright_holder = {:?}", self.copyright_holder);
        debug!("copyright_year = {:?}", self.copyright_year);
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
        let binaries = self.binaries(&package)?;
        let licenses = self.licenses(&package)?;
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
                product_name(self.product_name.as_ref(), &package),
            )
            .insert_str("manufacturer", self.manufacturer(&package)?)
            .insert_str("upgrade-code-guid", self.upgrade_guid(&package)?)
            .insert_str("path-component-guid", self.path_guid(&package)?);
        if let Some(banner) = self.banner_image(&package) {
            map = map.insert_str("banner", banner);
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
        if let Some(dialog) = self.dialog_image(&package) {
            map = map.insert_str("dialog", dialog);
        }
        if let Some(eula) = &licenses.end_user_license {
            map = map.insert_str("eula", &eula.stored_path);
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
        if let Some(license) = &licenses.source_license {
            map = map.insert_str("license-source", &license.stored_path);
            if let Some(name) = &license.name {
                map = map.insert_str("license-name", name);
            }
        }
        if let Some(icon) = self.product_icon(&package) {
            map = map.insert_str("product-icon", icon);
        }

        let wxs = {
            let data = map.build();
            let main_destination = self.output.clone();
            let template = mustache::compile_str(Template::Wxs.to_str())?;
            let rendered = template.render_data_to_string(&data).map_err(Error::from)?;
            RenderOutput {
                path: main_destination,
                rendered,
            }
        };
        let license = self.render_license_string(licenses.source_license.as_ref())?;
        let eula = self.render_license_string(licenses.end_user_license.as_ref())?;
        Ok(WxsRenders { wxs, license, eula })
    }

    fn render_license_string(&self, license: Option<&License>) -> Result<Option<RenderOutput>> {
        let Some(license) = license else {
            return Ok(None);
        };
        let Some((output, template)) = &license.generate else {
            return Ok(None);
        };

        let mut printer = crate::print::license::Builder::new();
        printer.copyright_holder(self.copyright_holder.as_ref().map(String::as_ref));
        printer.copyright_year(self.copyright_year.as_ref().map(String::as_ref));
        printer.input(self.input.as_deref().and_then(Path::to_str));
        printer.output(Some(output.as_str()));
        printer.package(self.package.as_deref());

        let render = printer.build().render(template)?;
        Ok(Some(render))
    }

    fn binaries(&self, package: &Package) -> Result<Vec<HashMap<&'static str, String>>> {
        let mut binaries = Vec::new();
        if let Some(binary_paths) = &self.binaries {
            let mut map = HashMap::with_capacity(3);
            for (index, binary) in binary_paths.iter().enumerate() {
                let binary_file_stem = binary.file_stem().ok_or_else(|| {
                    Error::Generic(format!(
                        "The '{}' binary path does not have a file name",
                        binary
                    ))
                })?;
                map.insert("binary-index", index.to_string());
                map.insert("binary-name", binary_file_stem.to_owned());
                map.insert("binary-source", binary.to_string());
            }
            binaries.push(map);
        } else {
            // cargo-metadata attempts to sort binaries by name to keep things stable,
            // but for whatever reason it internally uses the platform-specific binary name
            // with ".exe" appended, even though the output doesn't refer to that extension.
            // As such, the ordering ends up being platform-specific, because any time one
            // binary has a name that's a prefix of the other (e.g. "app" and "app-helper")
            // the `.exe` extension changes the ordering of the sort. We sort the list again
            // with the agnostic name to avoid this issue.
            let mut binaries_list = package
                .targets
                .iter()
                .filter(|v| v.kind.iter().any(|v| v == "bin"))
                .collect::<Vec<_>>();
            binaries_list.sort_by_key(|k| &k.name);

            for (index, binary) in binaries_list.into_iter().enumerate() {
                let mut map = HashMap::with_capacity(3);
                map.insert("binary-index", index.to_string());
                map.insert("binary-name", binary.name.clone());
                map.insert(
                    "binary-source",
                    Self::default_binary_path(&binary.name).to_string(),
                );
                binaries.push(map);
            }
        }
        Ok(binaries)
    }

    fn default_binary_path(name: &str) -> StoredPathBuf {
        // Use hardcoded path separator here to avoid platform-specific output
        StoredPathBuf::from(format!(
            "$(var.CargoTargetBinDir)\\{name}.{EXE_FILE_EXTENSION}"
        ))
    }

    fn help_url(&self, manifest: &Package) -> Option<String> {
        self.help_url
            .as_ref()
            .map(String::from)
            .or_else(|| manifest.documentation.clone())
            .or_else(|| manifest.homepage.clone())
            .or_else(|| manifest.repository.clone())
    }

    fn licenses(&self, manifest: &Package) -> Result<Licenses> {
        let output_dir = self
            .output
            .as_deref()
            .and_then(|p| p.parent())
            .and_then(Utf8Path::from_path);
        let licenses = Licenses::new(
            output_dir,
            self.license.as_deref(),
            self.eula.as_deref(),
            manifest,
        )?;
        Ok(licenses)
    }

    fn manufacturer(&self, manifest: &Package) -> Result<String> {
        if let Some(ref m) = self.manufacturer {
            Ok(m.to_owned())
        } else {
            super::authors(manifest)
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
                .map(|u| u.as_hyphenated().to_string().to_uppercase())
                .map_err(Error::from)
        } else {
            Ok(Uuid::new_v4().as_hyphenated().to_string().to_uppercase())
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
                .map(|u| u.as_hyphenated().to_string().to_uppercase())
                .map_err(Error::from)
        } else {
            Ok(Uuid::new_v4().as_hyphenated().to_string().to_uppercase())
        }
    }

    fn banner_image(&self, manifest: &Package) -> Option<StoredPathBuf> {
        if let Some(path) = &self.banner {
            trace!("A banner image has been explicitly specified");
            Some(path.clone())
        } else {
            manifest
                .metadata
                .get("wix")
                .and_then(|w| w.as_object())
                .and_then(|t| t.get("banner"))
                .and_then(|p| p.as_str())
                .map(|p| StoredPathBuf::new(p.to_owned()))
        }
    }

    fn dialog_image(&self, manifest: &Package) -> Option<StoredPathBuf> {
        if let Some(path) = &self.dialog {
            trace!("A dialog image has been explicitly specified");
            Some(path.clone())
        } else {
            manifest
                .metadata
                .get("wix")
                .and_then(|w| w.as_object())
                .and_then(|t| t.get("dialog"))
                .and_then(|p| p.as_str())
                .map(|p| StoredPathBuf::new(p.to_owned()))
        }
    }

    fn product_icon(&self, manifest: &Package) -> Option<StoredPathBuf> {
        if let Some(path) = &self.product_icon {
            trace!("A product icon has been explicitly specified");
            Some(path.clone())
        } else {
            manifest
                .metadata
                .get("wix")
                .and_then(|w| w.as_object())
                .and_then(|t| t.get("product-icon"))
                .and_then(|p| p.as_str())
                .map(|p| StoredPathBuf::new(p.to_owned()))
        }
    }

    #[cfg(test)]
    pub fn for_test(input: &Path) -> Self {
        let input = Utf8Path::from_path(input).expect("utf8 path");
        let output = input
            .parent()
            .expect("Cargo.toml to not be a root")
            .join(crate::WIX)
            .join(format!(
                "{}.{}",
                crate::WIX_SOURCE_FILE_NAME,
                crate::WIX_SOURCE_FILE_EXTENSION
            ));
        Builder::new()
            .input(Some(input.as_str()))
            .output(Some(output.as_str()))
            .build()
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
            let expected = Uuid::new_v4().as_hyphenated().to_string().to_uppercase();
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
            let expected = Uuid::new_v4().as_hyphenated().to_string().to_uppercase();
            let mut actual = Builder::new();
            actual.upgrade_guid(Some(&expected));
            assert_eq!(actual.upgrade_guid, Some(expected.as_ref()));
        }
    }

    mod execution {
        extern crate assert_fs;

        use super::*;
        use crate::tests::setup_project;
        use crate::{LICENSE_FILE_NAME, RTF_FILE_EXTENSION, WIX};
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

        const LICENSE_FALSE_MANIFEST: &str = r#"[package]
            name = "Example"
            version = "0.1.0"
            authors = ["First Last <first.last@example.com>"]
            license = "MIT"

            [package.metadata.wix]
            license = false
            eula = true
        "#;

        const EULA_FALSE_MANIFEST: &str = r#"[package]
            name = "Example"
            version = "0.1.0"
            authors = ["First Last <first.last@example.com>"]
            license = "MIT"

            [package.metadata.wix]
            license = true
            eula = false
        "#;

        const EULA_AND_LICENSE_FALSE_MANIFEST: &str = r#"[package]
            name = "Example"
            version = "0.1.0"
            authors = ["First Last <first.last@example.com>"]
            license = "MIT"

            [package.metadata.wix]
            license = false
            eula = false
        "#;

        const LICENSE_PATH_RTF_MANIFEST: &str = r#"[package]
            name = "Example"
            version = "0.1.0"
            authors = ["First Last <first.last@example.com>"]
            license = "MIT"

            [package.metadata.wix]
            license = "MyLicense.rtf"
        "#;

        const LICENSE_PATH_TXT_MANIFEST: &str = r#"[package]
            name = "Example"
            version = "0.1.0"
            authors = ["First Last <first.last@example.com>"]
            license = "MIT"

            [package.metadata.wix]
            license = "MyLicense.txt"
        "#;

        const EULA_PATH_RTF_MANIFEST: &str = r#"[package]
            name = "Example"
            version = "0.1.0"
            authors = ["First Last <first.last@example.com>"]
            license = "MIT"

            [package.metadata.wix]
            eula = "MyEula.rtf"
        "#;

        const EULA_PATH_TXT_MANIFEST: &str = r#"[package]
            name = "Example"
            version = "0.1.0"
            authors = ["First Last <first.last@example.com>"]
            license = "MIT"

            [package.metadata.wix]
            eula = "MyEula.txt"
        "#;

        const EULA_AND_LICENSE_PATH_RTF_MANIFEST: &str = r#"[package]
            name = "Example"
            version = "0.1.0"
            authors = ["First Last <first.last@example.com>"]
            license = "MIT"

            [package.metadata.wix]
            license = "MyLicense.rtf"
            eula = "MyEula.rtf"
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

        const IMAGES_MANIFEST: &str = r#"[package]
            name = "Example"
            version = "0.1.0"
            authors = ["First Last <first.last@example.com>"]

            [package.metadata.wix]
            product-icon = "wix/product.ico"
            dialog = "wix/dialog.png"
            banner = "wix/banner.png"
        "#;

        #[test]
        fn license_name_with_mit_license_field_works() {
            let project = setup_project(MIT_MANIFEST);
            let input = project.path().join("Cargo.toml");
            let manifest = crate::manifest(Some(&input)).unwrap();
            let package = crate::package(&manifest, None).unwrap();

            let actual = Execution::for_test(&input)
                .licenses(&package)
                .expect("licenses")
                .source_license
                .expect("source license")
                .stored_path;
            assert_eq!(
                actual,
                StoredPathBuf::from(format!("{WIX}\\{LICENSE_FILE_NAME}.{RTF_FILE_EXTENSION}"))
            );
        }

        #[test]
        fn license_name_with_gpl3_license_field_works() {
            let project = setup_project(GPL3_MANIFEST);
            let input = project.path().join("Cargo.toml");
            let manifest = crate::manifest(Some(&input)).unwrap();
            let package = crate::package(&manifest, None).unwrap();

            let actual = Execution::for_test(&input)
                .licenses(&package)
                .expect("licenses")
                .source_license
                .expect("source license")
                .stored_path;
            assert_eq!(
                actual,
                StoredPathBuf::from(format!("{WIX}\\{LICENSE_FILE_NAME}.{RTF_FILE_EXTENSION}"))
            );
        }

        #[test]
        fn license_name_with_apache2_license_field_works() {
            let project = setup_project(APACHE2_MANIFEST);
            let input = project.path().join("Cargo.toml");
            let manifest = crate::manifest(Some(&input)).unwrap();
            let package = crate::package(&manifest, None).unwrap();

            let actual = Execution::for_test(&input)
                .licenses(&package)
                .expect("licenses")
                .source_license
                .expect("source license")
                .stored_path;
            assert_eq!(
                actual,
                StoredPathBuf::from(format!("{WIX}\\{LICENSE_FILE_NAME}.{RTF_FILE_EXTENSION}"))
            );
        }

        #[test]
        fn license_name_with_unknown_license_field_works() {
            let project = setup_project(UNKNOWN_MANIFEST);
            let input = project.path().join("Cargo.toml");
            let manifest = crate::manifest(Some(&input)).unwrap();
            let package = crate::package(&manifest, None).unwrap();

            let actual = Execution::for_test(&input)
                .licenses(&package)
                .expect("licenses")
                .source_license;
            assert!(actual.is_none());
        }

        #[test]
        fn license_source_with_mit_license_field_works() {
            let project = setup_project(MIT_MANIFEST);
            let input = project.path().join("Cargo.toml");
            let manifest = crate::manifest(Some(&input)).unwrap();
            let package = crate::package(&manifest, None).unwrap();

            let actual = Execution::for_test(&input)
                .licenses(&package)
                .expect("licenses")
                .source_license
                .expect("source license")
                .stored_path;
            assert_eq!(
                actual,
                StoredPathBuf::from(format!("{WIX}\\{LICENSE_FILE_NAME}.{RTF_FILE_EXTENSION}"))
            );
        }

        #[test]
        fn license_source_with_gpl3_license_field_works() {
            let project = setup_project(GPL3_MANIFEST);
            let input = project.path().join("Cargo.toml");
            let manifest = crate::manifest(Some(&input)).unwrap();
            let package = crate::package(&manifest, None).unwrap();

            let actual = Execution::for_test(&input)
                .licenses(&package)
                .expect("licenses")
                .source_license
                .expect("source license")
                .stored_path;
            assert_eq!(
                actual,
                StoredPathBuf::from(format!("{WIX}\\{LICENSE_FILE_NAME}.{RTF_FILE_EXTENSION}"))
            );
        }

        #[test]
        fn license_source_with_apache2_license_field_works() {
            let project = setup_project(APACHE2_MANIFEST);
            let input = project.path().join("Cargo.toml");
            let manifest = crate::manifest(Some(&input)).unwrap();
            let package = crate::package(&manifest, None).unwrap();

            let actual = Execution::for_test(&input)
                .licenses(&package)
                .expect("licenses")
                .source_license
                .expect("source license")
                .stored_path;
            assert_eq!(
                actual,
                StoredPathBuf::from(format!("{WIX}\\{LICENSE_FILE_NAME}.{RTF_FILE_EXTENSION}"))
            );
        }

        #[test]
        fn license_source_with_unknown_license_field_works() {
            let project = setup_project(UNKNOWN_MANIFEST);
            let input = project.path().join("Cargo.toml");
            let manifest = crate::manifest(Some(&input)).unwrap();
            let package = crate::package(&manifest, None).unwrap();

            let actual = Execution::for_test(&input)
                .licenses(&package)
                .expect("licenses")
                .source_license;
            assert!(actual.is_none());
        }

        #[test]
        fn license_false_works() {
            let project = setup_project(LICENSE_FALSE_MANIFEST);
            let input = project.path().join("Cargo.toml");
            let manifest = crate::manifest(Some(&input)).unwrap();
            let package = crate::package(&manifest, None).unwrap();

            let licenses = Execution::for_test(&input)
                .licenses(&package)
                .expect("licenses");
            assert_eq!(licenses.source_license, None);
            assert_eq!(licenses.end_user_license, None);
        }

        #[test]
        fn eula_false_works() {
            let project = setup_project(EULA_FALSE_MANIFEST);
            let input = project.path().join("Cargo.toml");
            let manifest = crate::manifest(Some(&input)).unwrap();
            let package = crate::package(&manifest, None).unwrap();

            let licenses = Execution::for_test(&input)
                .licenses(&package)
                .expect("licenses");
            assert_eq!(
                licenses.source_license.unwrap().stored_path,
                StoredPathBuf::from(format!("{WIX}\\{LICENSE_FILE_NAME}.{RTF_FILE_EXTENSION}"))
            );
            assert_eq!(licenses.end_user_license, None);
        }

        #[test]
        fn eula_and_license_false_works() {
            let project = setup_project(EULA_AND_LICENSE_FALSE_MANIFEST);
            let input = project.path().join("Cargo.toml");
            let manifest = crate::manifest(Some(&input)).unwrap();
            let package = crate::package(&manifest, None).unwrap();

            let licenses = Execution::for_test(&input)
                .licenses(&package)
                .expect("licenses");
            assert_eq!(licenses.source_license, None);
            assert_eq!(licenses.end_user_license, None);
        }

        #[test]
        fn license_path_rtf_works() {
            let project = setup_project(LICENSE_PATH_RTF_MANIFEST);
            let license_file_path = project.path().join("MyLicense.rtf");
            let _license_file_handle = File::create(&license_file_path).expect("Create file");
            let input = project.path().join("Cargo.toml");
            let manifest = crate::manifest(Some(&input)).unwrap();
            let package = crate::package(&manifest, None).unwrap();

            let licenses = Execution::for_test(&input)
                .licenses(&package)
                .expect("licenses");
            assert_eq!(
                licenses.source_license.unwrap().stored_path.as_str(),
                "MyLicense.rtf"
            );
            assert_eq!(
                licenses.end_user_license.unwrap().stored_path.as_str(),
                "MyLicense.rtf"
            );
        }

        #[test]
        fn license_path_txt_works() {
            let project = setup_project(LICENSE_PATH_TXT_MANIFEST);
            let license_file_path = project.path().join("MyLicense.txt");
            let _license_file_handle = File::create(&license_file_path).expect("Create file");
            let input = project.path().join("Cargo.toml");
            let manifest = crate::manifest(Some(&input)).unwrap();
            let package = crate::package(&manifest, None).unwrap();

            let licenses = Execution::for_test(&input)
                .licenses(&package)
                .expect("licenses");
            assert_eq!(
                licenses.source_license.unwrap().stored_path.as_str(),
                "MyLicense.txt"
            );
            assert_eq!(licenses.end_user_license, None);
        }

        #[test]
        fn eula_path_rtf_works() {
            let project = setup_project(EULA_PATH_RTF_MANIFEST);
            let license_file_path = project.path().join("MyEula.rtf");
            let _license_file_handle = File::create(&license_file_path).expect("Create file");
            let input = project.path().join("Cargo.toml");
            let manifest = crate::manifest(Some(&input)).unwrap();
            let package = crate::package(&manifest, None).unwrap();

            let licenses = Execution::for_test(&input)
                .licenses(&package)
                .expect("licenses");
            assert_eq!(
                licenses.source_license.unwrap().stored_path.as_str(),
                format!("{WIX}\\{LICENSE_FILE_NAME}.{RTF_FILE_EXTENSION}")
            );
            assert_eq!(
                licenses.end_user_license.unwrap().stored_path.as_str(),
                "MyEula.rtf"
            );
        }

        #[test]
        fn eula_path_txt_works() {
            let project = setup_project(EULA_PATH_TXT_MANIFEST);
            let license_file_path = project.path().join("MyEula.txt");
            let _license_file_handle = File::create(&license_file_path).expect("Create file");
            let input = project.path().join("Cargo.toml");
            let manifest = crate::manifest(Some(&input)).unwrap();
            let package = crate::package(&manifest, None).unwrap();

            let licenses = Execution::for_test(&input)
                .licenses(&package)
                .expect("licenses");
            assert_eq!(
                licenses.source_license.unwrap().stored_path.as_str(),
                format!("{WIX}\\{LICENSE_FILE_NAME}.{RTF_FILE_EXTENSION}")
            );
            assert_eq!(
                licenses.end_user_license.unwrap().stored_path.as_str(),
                "MyEula.txt"
            );
        }

        #[test]
        fn eula_and_license_path_rtf_works() {
            let project = setup_project(EULA_AND_LICENSE_PATH_RTF_MANIFEST);
            let license_file_path = project.path().join("MyLicense.rtf");
            let _license_file_handle = File::create(&license_file_path).expect("Create file");
            let eula_file_path = project.path().join("MyEula.rtf");
            let _eula_file_handle = File::create(&eula_file_path).expect("Create file");
            let input = project.path().join("Cargo.toml");
            let manifest = crate::manifest(Some(&input)).unwrap();
            let package = crate::package(&manifest, None).unwrap();

            let licenses = Execution::for_test(&input)
                .licenses(&package)
                .expect("licenses");
            assert_eq!(
                licenses.source_license.unwrap().stored_path.as_str(),
                "MyLicense.rtf"
            );
            assert_eq!(
                licenses.end_user_license.unwrap().stored_path.as_str(),
                "MyEula.rtf"
            );
        }

        #[test]
        fn binaries_with_no_bin_section_works() {
            let project = setup_project(MIT_MANIFEST);
            let input = project.path().join("Cargo.toml");
            let manifest = crate::manifest(Some(&input)).unwrap();
            let package = crate::package(&manifest, None).unwrap();

            let actual = Execution::for_test(&input).binaries(&package).unwrap();
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
            let input = project.path().join("Cargo.toml");
            let manifest = crate::manifest(Some(&input)).unwrap();
            let package = crate::package(&manifest, None).unwrap();

            let actual = Execution::for_test(&input).binaries(&package).unwrap();
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
            let input = project.path().join("Cargo.toml");
            let manifest = crate::manifest(Some(&input)).unwrap();
            let package = crate::package(&manifest, None).unwrap();

            let actual = Execution::for_test(&input).binaries(&package).unwrap();
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
            let input = project.path().join("Cargo.toml");
            let manifest = crate::manifest(Some(&input)).unwrap();
            let package = crate::package(&manifest, None).unwrap();

            let actual = Execution::for_test(&input).manufacturer(&package).unwrap();
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
            let input = project.path().join("Cargo.toml");
            let manifest = crate::manifest(Some(&input)).unwrap();
            let package = crate::package(&manifest, None).unwrap();

            let actual = Execution::for_test(&input)
                .licenses(&package)
                .unwrap()
                .end_user_license;
            assert!(actual.is_none());
        }

        #[test]
        fn eula_with_mit_license_field_works() {
            let project = setup_project(MIT_MANIFEST);
            let input = project.path().join("Cargo.toml");
            let manifest = crate::manifest(Some(&input)).unwrap();
            let package = crate::package(&manifest, None).unwrap();

            let licenses = Execution::for_test(&input).licenses(&package).unwrap();
            let source = licenses.source_license.unwrap();
            let source_path = source.stored_path;
            let (template_out, source_template) = source.generate.unwrap();
            let eula_path = licenses.end_user_license.unwrap().stored_path;

            let expected_rel_path = format!("{WIX}\\{LICENSE_FILE_NAME}.{RTF_FILE_EXTENSION}");
            let expected_abs_path = Utf8Path::from_path(input.parent().unwrap())
                .unwrap()
                .join(WIX)
                .join(format!("{LICENSE_FILE_NAME}.{RTF_FILE_EXTENSION}"));
            assert_eq!(source_template, Template::Mit);
            assert_eq!(source_path.as_str(), expected_rel_path);
            assert_eq!(template_out, expected_abs_path);
            assert_eq!(eula_path.as_str(), expected_rel_path);
        }

        #[test]
        fn eula_with_apache2_license_field_works() {
            let project = setup_project(APACHE2_MANIFEST);
            let input = project.path().join("Cargo.toml");
            let manifest = crate::manifest(Some(&input)).unwrap();
            let package = crate::package(&manifest, None).unwrap();

            let licenses = Execution::for_test(&input).licenses(&package).unwrap();
            let source = licenses.source_license.unwrap();
            let source_path = source.stored_path;
            let (template_out, source_template) = source.generate.unwrap();
            let eula_path = licenses.end_user_license.unwrap().stored_path;

            let expected_rel_path = format!("{WIX}\\{LICENSE_FILE_NAME}.{RTF_FILE_EXTENSION}");
            let expected_abs_path = Utf8Path::from_path(input.parent().unwrap())
                .unwrap()
                .join(WIX)
                .join(format!("{LICENSE_FILE_NAME}.{RTF_FILE_EXTENSION}"));
            assert_eq!(source_template, Template::Apache2);
            assert_eq!(source_path.as_str(), expected_rel_path);
            assert_eq!(template_out, expected_abs_path);
            assert_eq!(eula_path.as_str(), expected_rel_path);
        }

        #[test]
        fn eula_with_gpl3_license_field_works() {
            let project = setup_project(GPL3_MANIFEST);
            let input = project.path().join("Cargo.toml");
            let manifest = crate::manifest(Some(&input)).unwrap();
            let package = crate::package(&manifest, None).unwrap();

            let licenses = Execution::for_test(&input).licenses(&package).unwrap();
            let source = licenses.source_license.unwrap();
            let source_path = source.stored_path;
            let (template_out, source_template) = source.generate.unwrap();
            let eula_path = licenses.end_user_license.unwrap().stored_path;

            let expected_rel_path = format!("{WIX}\\{LICENSE_FILE_NAME}.{RTF_FILE_EXTENSION}");
            let expected_abs_path = Utf8Path::from_path(input.parent().unwrap())
                .unwrap()
                .join(WIX)
                .join(format!("{LICENSE_FILE_NAME}.{RTF_FILE_EXTENSION}"));
            assert_eq!(source_template, Template::Gpl3);
            assert_eq!(source_path.as_str(), expected_rel_path);
            assert_eq!(template_out, expected_abs_path);
            assert_eq!(eula_path.as_str(), expected_rel_path);
        }

        #[test]
        fn eula_with_unknown_license_field_works() {
            let project = setup_project(UNKNOWN_MANIFEST);
            let input = project.path().join("Cargo.toml");
            let manifest = crate::manifest(Some(&input)).unwrap();
            let package = crate::package(&manifest, None).unwrap();

            let licenses = Execution::for_test(&input).licenses(&package).unwrap();
            let source = licenses.source_license;
            let eula = licenses.end_user_license;

            assert_eq!(source, None);
            assert_eq!(eula, None);
        }

        #[test]
        #[cfg(windows)]
        fn eula_with_override_works() {
            let project = setup_project(MIT_MANIFEST);
            let license_file_path = project.path().join("Example.rtf");
            let _license_file_handle = File::create(&license_file_path).expect("Create file");

            let manifest = crate::manifest(Some(&project.path().join("Cargo.toml"))).unwrap();
            let package = crate::package(&manifest, None).unwrap();

            let actual = Builder::default()
                .eula(license_file_path.to_str())
                .build()
                .licenses(&package)
                .unwrap()
                .end_user_license
                .unwrap()
                .stored_path;
            assert_eq!(
                actual,
                StoredPathBuf::from_std_path(&license_file_path).unwrap(),
            );
        }

        #[test]
        fn eula_with_license_file_field_works() {
            let project = setup_project(LICENSE_FILE_RTF_MANIFEST);
            let license_file_path = project.path().join("Example.rtf");
            let _license_file_handle = File::create(&license_file_path).expect("Create file");

            let input = project.path().join("Cargo.toml");
            let manifest = crate::manifest(Some(&input)).unwrap();
            let package = crate::package(&manifest, None).unwrap();

            let actual = Execution::for_test(&input)
                .licenses(&package)
                .unwrap()
                .end_user_license
                .unwrap()
                .stored_path;
            assert_eq!(actual.as_str(), "Example.rtf");
        }

        #[test]
        fn eula_with_license_file_extension_works() {
            let project = setup_project(LICENSE_FILE_TXT_MANIFEST);
            let license_file_path = project.path().join("Example.txt");
            let _license_file_handle = File::create(license_file_path).expect("Create file");

            let input = project.path().join("Cargo.toml");
            let manifest = crate::manifest(Some(&input)).unwrap();
            let package = crate::package(&manifest, None).unwrap();

            let licenses = Execution::for_test(&input).licenses(&package).unwrap();
            let source = licenses.source_license.unwrap();
            let eula = licenses.end_user_license;

            assert_eq!(source.generate, None);
            assert_eq!(source.name, None);
            assert_eq!(source.stored_path.as_str(), "Example.txt");
            assert_eq!(eula, None);
        }

        #[test]
        fn eula_with_wrong_file_extension_override_works() {
            let project = setup_project(LICENSE_FILE_TXT_MANIFEST);
            let license_file_path = project.path().join("Example.txt");
            let _license_file_handle = File::create(&license_file_path).expect("Create file");

            let manifest = crate::manifest(Some(&project.path().join("Cargo.toml"))).unwrap();
            let package = crate::package(&manifest, None).unwrap();

            // We want to check that if the user hands us an OS-specific path here then we preserve it's format
            // So we turn the input to a string without escaping.
            let input = license_file_path.to_str().unwrap();
            let expected = StoredPathBuf::new(input.to_owned());
            let licenses = Builder::default()
                .eula(Some(input))
                .build()
                .licenses(&package)
                .unwrap();
            let eula = licenses.end_user_license.unwrap();

            assert_eq!(eula.stored_path, expected);
        }

        #[test]
        fn path_guid_with_override_works() {
            let expected = Uuid::new_v4().as_hyphenated().to_string().to_uppercase();

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
            let expected = Uuid::new_v4().as_hyphenated().to_string().to_uppercase();

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
            let expected = Uuid::new_v4().as_hyphenated().to_string().to_uppercase();

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
            let expected = Uuid::new_v4().as_hyphenated().to_string().to_uppercase();

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

        #[test]
        fn image_metadata_works() {
            let project = setup_project(IMAGES_MANIFEST);
            let manifest = crate::manifest(Some(&project.path().join("Cargo.toml"))).unwrap();
            let package = crate::package(&manifest, None).unwrap();

            let actual = Builder::default().build();
            assert_eq!(
                actual.product_icon(&package).unwrap().as_str(),
                "wix/product.ico"
            );
            assert_eq!(
                actual.dialog_image(&package).unwrap().as_str(),
                "wix/dialog.png"
            );
            assert_eq!(
                actual.banner_image(&package).unwrap().as_str(),
                "wix/banner.png"
            );
        }
    }
}
