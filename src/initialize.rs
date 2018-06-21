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

use CARGO_MANIFEST_FILE;
use Error;
use eula::Eula;
use print;
use Result;
use std::env;
use std::ffi::OsStr;
use std::fs;
use std::path::{Path, PathBuf};
use WIX_SOURCE_FILE_NAME;
use WIX_SOURCE_FILE_EXTENSION; 
use WIX;
use RTF_FILE_EXTENSION;

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
    /// Creates a new `Builder` instance.
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
    /// Generally, the binary name should _not_ have spaces or special characters. The binary name
    /// is the name of the executable. This will _not_ appear in the Add/Remove Programs control
    /// panel. Use the `product_name` method to change the name that appears in the Add/Remove
    /// Programs control panel.
    pub fn binary_name(&mut self, b: Option<&'a str>) -> &mut Self {
        self.binary_name = b;
        self
    }

    /// Sets the copyright holder in the license dialog of the Windows installer (msi).
    pub fn copyright_holder(&mut self, h: Option<&'a str>) -> &mut Self {
        self.copyright_holder = h;
        self
    }

    /// Sets the copyright year in the license dialog of the Windows installer (msi).
    pub fn copyright_year(&mut self, y: Option<&'a str>) -> &mut Self {
        self.copyright_year = y;
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
   
    /// Builds a read-only initialization execution.
    pub fn build(&mut self) -> Execution {
        let mut wxs_printer = print::wxs::Builder::new();
        wxs_printer.binary_name(self.binary_name);
        Execution {
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

impl<'a> Default for Builder<'a> {
    fn default() -> Self {
        Builder::new()
    }
}

#[derive(Debug)]
pub struct Execution {
    binary_name: Option<String>,
    copyright_holder: Option<String>,
    copyright_year: Option<String>,
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

impl Execution {
    pub fn run(self) -> Result<()> {
        debug!("binary_name = {:?}", self.binary_name);
        debug!("copyright_holder = {:?}", self.copyright_holder);
        debug!("copyright_year = {:?}", self.copyright_year);
        debug!("description = {:?}", self.description);
        debug!("eula = {:?}", self.eula);
        debug!("force = {:?}", self.force);
        debug!("help_url = {:?}", self.help_url);
        debug!("input = {:?}", self.input);
        debug!("license = {:?}", self.license);
        debug!("manufacturer = {:?}", self.manufacturer);
        debug!("output = {:?}", self.output);
        debug!("product_name = {:?}", self.product_name);
        let manifest = super::manifest(self.input.as_ref())?;
        let mut destination = self.destination()?;
        debug!("destination = {:?}", destination);
        if !destination.exists() {
            info!("Creating the '{}' directory", destination.display());
            fs::create_dir(&destination)?;
        }
        destination.push(WIX_SOURCE_FILE_NAME);
        destination.set_extension(WIX_SOURCE_FILE_EXTENSION);
        if destination.exists() && !self.force {
            return Err(Error::already_exists(&destination));
        } else {
            info!("Creating the '{}\\{}.{}' file", WIX, WIX_SOURCE_FILE_NAME, WIX_SOURCE_FILE_EXTENSION);
            let mut wxs_printer = print::wxs::Builder::new();
            wxs_printer.binary_name(self.binary_name.as_ref().map(String::as_ref));
            wxs_printer.description(self.description.as_ref().map(String::as_ref));
            wxs_printer.eula(self.eula.as_ref().map(PathBuf::as_path).and_then(Path::to_str));
            wxs_printer.help_url(self.help_url.as_ref().map(String::as_ref));
            wxs_printer.input(self.input.as_ref().map(PathBuf::as_path).and_then(Path::to_str));
            wxs_printer.license(self.license.as_ref().map(PathBuf::as_path).and_then(Path::to_str));
            wxs_printer.manufacturer(self.manufacturer.as_ref().map(String::as_ref));
            wxs_printer.output(destination.as_path().to_str());
            wxs_printer.product_name(self.product_name.as_ref().map(String::as_ref));
            wxs_printer.build().run()?;
        }
        destination.pop(); // Remove main.wxs
        if let Eula::Generate(template) = Eula::new(self.eula.as_ref(), &manifest)? {
            destination.push("License");
            destination.set_extension(RTF_FILE_EXTENSION);
            if destination.exists() && !self.force {
                return Err(Error::already_exists(&destination));
            } else {
                info!("Generating a EULA");
                let mut eula_printer = print::license::Builder::new();
                eula_printer.copyright_holder(self.copyright_holder.as_ref().map(String::as_ref));
                eula_printer.copyright_year(self.copyright_year.as_ref().map(String::as_ref));
                eula_printer.input(self.input.as_ref().map(PathBuf::as_path).and_then(Path::to_str));
                eula_printer.output(destination.as_path().to_str());
                eula_printer.build().run(template)?;
            }
        }
        Ok(())
    }

    fn destination(&self) -> Result<PathBuf> {
        if let Some(ref output) = self.output {
            trace!("An output path has been explicity specified");
            Ok(output.to_owned())
        } else {
            trace!("An output path has NOT been explicity specified. Implicitly determine output.");
            if let Some(ref input) = self.input {
                trace!("An input path has been explicitly specified");
                if input.exists() && 
                   input.is_file() && 
                   input.file_name() == Some(OsStr::new(CARGO_MANIFEST_FILE)) 
                {
                    trace!("The input path exists, it is a file, and it is appears to be Cargo.toml");
                    Ok(input.parent().map(|p| p.to_path_buf()).and_then(|mut p| {
                        p.push(WIX);
                        Some(p)
                    }).unwrap())
                } else {
                    Err(Error::not_found(input))
                }
            } else {
                trace!("An input path has NOT been explicitly specified, implicitly using the \
                       current working directory");
                let mut cwd = env::current_dir()?;
                cwd.push(CARGO_MANIFEST_FILE);
                if cwd.exists() && cwd.is_file() {
                    trace!("The current working directory has a {} file", CARGO_MANIFEST_FILE);
                    Ok(cwd.parent().map(|p| p.to_path_buf()).and_then(|mut p| {
                        p.push(WIX);
                        Some(p)
                    }).unwrap())
                } else {
                    Err(Error::not_found(&cwd))
                }
            }
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
        fn defaults_are_correct() {
            let actual = Builder::new();
            assert!(actual.binary_name.is_none());
            assert!(actual.copyright_year.is_none());
            assert!(actual.copyright_holder.is_none());
            assert!(actual.description.is_none());
            assert!(actual.eula.is_none());
            assert!(!actual.force);
            assert!(actual.help_url.is_none());
            assert!(actual.input.is_none());
            assert!(actual.license.is_none());
            assert!(actual.manufacturer.is_none());
            assert!(actual.output.is_none());
            assert!(actual.product_name.is_none());
        }

        #[test]
        fn binary_name_works() {
            const EXPECTED: &str = "name";
            let mut actual = Builder::new();
            actual.binary_name(Some(EXPECTED));
            assert_eq!(actual.binary_name, Some(EXPECTED));
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
        fn product_name_works() {
            const EXPECTED: &str = "product name";
            let mut actual = Builder::new();
            actual.product_name(Some(EXPECTED));
            assert_eq!(actual.product_name, Some(EXPECTED));
        }

        #[test]
        fn build_with_defaults_works() {
            let mut b = Builder::new();
            let default_execution = b.build();
            assert!(default_execution.binary_name.is_none());
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
            assert!(default_execution.product_name.is_none());
        }

        #[test]
        fn build_with_all_works() {
            const EXPECTED_BINARY_NAME: &str = "Binary Name";
            const EXPECTED_COPYRIGHT_HOLDER: &str = "Copyright Holder";
            const EXPECTED_COPYRIGHT_YEAR: &str = "Copyright Year";
            const EXPECTED_DESCRIPTION: &str = "Description";
            const EXPECTED_EULA: &str = "C:\\tmp\\eula.rtf";
            const EXPECTED_URL: &str = "http://github.com/volks73/cargo-wix";
            const EXPECTED_INPUT: &str = "C:\\tmp\\hello_world";
            const EXPECTED_LICENSE: &str = "C:\\tmp\\hello_world\\License.rtf";
            const EXPECTED_MANUFACTURER: &str = "Manufacturer";
            const EXPECTED_OUTPUT: &str = "C:\\tmp\\output";
            const EXPECTED_PRODUCT_NAME: &str = "Product Name";
            let mut b = Builder::new();
            b.binary_name(Some(EXPECTED_BINARY_NAME));
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
            b.product_name(Some(EXPECTED_PRODUCT_NAME));
            let execution = b.build();
            assert_eq!(execution.binary_name, Some(EXPECTED_BINARY_NAME).map(String::from));
            assert_eq!(execution.copyright_year, Some(EXPECTED_COPYRIGHT_YEAR).map(String::from));
            assert_eq!(execution.copyright_holder, Some(EXPECTED_COPYRIGHT_HOLDER).map(String::from));
            assert_eq!(execution.description, Some(EXPECTED_DESCRIPTION).map(String::from));
            assert_eq!(execution.eula, Some(EXPECTED_EULA).map(PathBuf::from));
            assert!(execution.force);
            assert_eq!(execution.help_url, Some(EXPECTED_URL).map(String::from));
            assert_eq!(execution.input, Some(EXPECTED_INPUT).map(PathBuf::from));
            assert_eq!(execution.license, Some(EXPECTED_LICENSE).map(PathBuf::from));
            assert_eq!(execution.manufacturer, Some(EXPECTED_MANUFACTURER).map(String::from));
            assert_eq!(execution.output, Some(EXPECTED_OUTPUT).map(PathBuf::from));
            assert_eq!(execution.product_name, Some(EXPECTED_PRODUCT_NAME).map(String::from));
        }
    }

    mod execution {
        extern crate tempfile;

        use std::fs::File;
        use std::io::ErrorKind;
        use super::*;

        #[test]
        fn destination_is_correct_with_defaults() {
            let original = env::current_dir().unwrap();
            let temp_dir = tempfile::tempdir().unwrap();
            env::set_current_dir(temp_dir.path()).unwrap();
            let temp_cargo_toml = temp_dir.path().join("Cargo.toml");
            File::create(&temp_cargo_toml).unwrap();
            let mut expected = env::current_dir().unwrap();
            expected.push(WIX);
            let e = Execution::default();
            let result = e.destination();
            env::set_current_dir(original).unwrap();
            let actual = result.unwrap();
            assert_eq!(actual, expected);
        }

        #[test]
        fn destination_is_correct_with_output() {
            let expected = PathBuf::from("output");
            let mut e = Execution::default();
            e.output = Some(expected.clone());
            let actual = e.destination().unwrap();
            assert_eq!(actual, expected);
        }

        #[test]
        fn destination_is_correct_with_input() {
            let temp_dir = tempfile::tempdir().unwrap();
            let temp_cargo_toml = temp_dir.path().join("Cargo.toml");
            File::create(&temp_cargo_toml).unwrap();
            let expected = temp_dir.path().join(WIX);
            let mut e = Execution::default();
            e.input = Some(temp_cargo_toml);
            let actual = e.destination().unwrap();
            assert_eq!(actual, expected);
        }

        #[test]
        #[should_panic]
        fn destination_fails_with_nonexistent_input() {
            let mut e = Execution::default();
            e.input = Some(PathBuf::from("not_real.toml"));
            e.destination().unwrap();
        }

        #[test]
        #[should_panic]
        fn destination_fails_with_directory_input() {
            let temp_dir = tempfile::tempdir().unwrap();
            let mut e = Execution::default();
            e.input = Some(temp_dir.path().into());
            e.destination().unwrap();
        }

        #[test]
        #[should_panic]
        fn destination_fails_when_input_is_not_a_manifest() {
            let temp_dir = tempfile::tempdir().unwrap();
            let not_a_manifest = temp_dir.path().join("Not_a_Manifest.txt");
            File::create(&not_a_manifest).unwrap();
            let mut e = Execution::default();
            e.input = Some(not_a_manifest);
            e.destination().unwrap();
        }

        #[test]
        fn destination_fails_correctly_when_input_is_not_a_manifest() {
            let temp_dir = tempfile::tempdir().unwrap();
            let not_a_manifest = temp_dir.path().join("Not_a_Manifest.txt");
            File::create(&not_a_manifest).unwrap();
            let mut e = Execution::default();
            e.input = Some(not_a_manifest);
            if let Error::Io(e) = e.destination().err().unwrap() {
                assert_eq!(e.kind(), ErrorKind::NotFound);
            } else {
                panic!("Incorrect error");
            }
        }

        #[test]
        #[should_panic]
        fn destination_fails_when_cwd_has_no_manifest() {
            let original = env::current_dir().unwrap();
            let temp_dir = tempfile::tempdir().unwrap();
            env::set_current_dir(temp_dir.path()).unwrap();
            let e = Execution::default();
            let result = e.destination();
            env::set_current_dir(original).unwrap();
            result.unwrap();
        }

        #[test]
        fn destination_fails_correctly_when_cwd_has_no_manifest() {
            let temp_dir = tempfile::tempdir().unwrap();
            let original = env::current_dir().unwrap();
            env::set_current_dir(temp_dir.path()).unwrap();
            let e = Execution::default();
            let result = e.destination();
            env::set_current_dir(original).unwrap();
            let err = result.err().unwrap();
            if let Error::Io(e) = err {
                assert_eq!(e.kind(), ErrorKind::NotFound);
            } else {
                panic!("Incorrect error");
            }
        }
    }
}

