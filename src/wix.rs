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

use chrono::{Datelike, Utc};
use mustache::{self, MapBuilder};
use regex::Regex;
use std::env;
use std::fs::{self, File};
use std::io::{self, Read, Write, ErrorKind};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::str::FromStr;
use super::{Error, Platform, Result, Template, TimestampServer};
use toml::Value;
use uuid::Uuid;

pub const CARGO_MANIFEST_FILE: &str = "Cargo.toml";
pub const CARGO: &str = "cargo";
pub const DEFAULT_LICENSE_FILE_NAME: &str = "LICENSE";
pub const EXE_FILE_EXTENSION: &str = "exe";
pub const RTF_FILE_EXTENSION: &str = "rtf";
pub const SIGNTOOL: &str = "signtool";
pub const SIGNTOOL_PATH_KEY: &str = "SIGNTOOL_PATH";
pub const WIX: &str = "wix";
pub const WIX_COMPILER: &str = "candle";
pub const WIX_LINKER: &str = "light";
pub const WIX_PATH_KEY: &str = "WIX_PATH";
pub const WIX_SOURCE_FILE_EXTENSION: &str = "wxs";
pub const WIX_SOURCE_FILE_NAME: &str = "main";

/// A builder for running the subcommand.
#[derive(Debug, Clone)]
pub struct Wix<'a> {
    bin_path: Option<&'a str>,
    binary_name: Option<&'a str>,
    capture_output: bool,
    copyright_year: Option<&'a str>,
    copyright_holder: Option<&'a str>,
    description: Option<&'a str>,
    input: Option<&'a str>,
    license_path: Option<&'a str>,
    manufacturer: Option<&'a str>,
    no_build: bool,
    output: Option<&'a str>,
    product_name: Option<&'a str>,
    sign: bool,
    sign_path: Option<&'a str>,
    timestamp: Option<&'a str>,
}

impl<'a> Wix<'a> {
    /// Creates a new `Wix` instance.
    pub fn new() -> Self {
        Wix {
            bin_path: None,
            binary_name: None,
            capture_output: true,
            copyright_year: None,
            copyright_holder: None,
            description: None,
            input: None,
            license_path: None,
            manufacturer: None,
            no_build: false,
            output: None,
            product_name: None,
            sign: false,
            sign_path: None,
            timestamp: None,
        }
    }

    /// Sets the path to the WiX Toolset's `bin` folder.
    ///
    /// The WiX Toolset's `bin` folder should contain the needed `candle.exe` and `light.exe`
    /// applications. The default is to use the PATH system environment variable. This will
    /// override any value obtained from the environment.
    pub fn bin_path(mut self, b: Option<&'a str>) -> Self {
        self.bin_path = b;
        self
    }

    /// Sets the binary name.
    ///
    /// This overrides the binary name determined from the package's manifest (Cargo.toml).
    pub fn binary_name(mut self, b: Option<&'a str>) -> Self {
        self.binary_name = b;
        self
    }

    /// Enables or disables capturing of the output from the builder (`cargo`), compiler
    /// (`candle`), linker (`light`), and signer (`signtool`).
    ///
    /// The default is to capture all output, i.e. display nothing in the console but the log
    /// statements.
    pub fn capture_output(mut self, c: bool) -> Self {
        self.capture_output = c;
        self
    }

    /// Sets the copyright holder in the license dialog of the Windows installer (msi).
    pub fn copyright_holder(mut self, h: Option<&'a str>) -> Self {
        self.copyright_holder = h;
        self
    }

    /// Sets the copyright year in the license dialog of the Windows installer (msi).
    pub fn copyright_year(mut self, y: Option<&'a str>) -> Self {
        self.copyright_year = y;
        self
    }

    /// Sets the description.
    ///
    /// This override the description determined from the `description` field in the package's
    /// manifest (Cargo.toml).
    pub fn description(mut self, d: Option<&'a str>) -> Self {
        self.description = d;
        self
    }

    /// Sets the path to a file to be used as the WiX Source (wxs) file instead of `wix\main.rs`.
    pub fn input(mut self, i: Option<&'a str>) -> Self {
        self.input = i;
        self
    }
    
    /// Sets the path to a file to used as the `License.txt` file within the installer.
    ///
    /// The `License.txt` file is installed into the installation location along side the `bin`
    /// folder. Note, the file can be in any format with any name, but it is automatically renamed
    /// to `License.txt` during creation of the installer.
    pub fn license_file(mut self, l: Option<&'a str>) -> Self {
        self.license_path = l;
        self
    }

    /// Overrides the first author in the `authors` field of the package's manifest (Cargo.toml) as
    /// the manufacturer within the installer.
    pub fn manufacturer(mut self, m: Option<&'a str>) -> Self {
        self.manufacturer = m;
        self
    }

    /// Skips the building of the project with the release profile.
    ///
    /// If `true`, the project will _not_ be built using the release profile, i.e. the `cargo build
    /// --release` command will not be executed. The default is to build the project before each
    /// creation. This is useful if building the project is more involved or is handled in
    /// a separate process.
    pub fn no_build(mut self, n: bool) -> Self {
        self.no_build = n;
        self
    }

    /// Sets the output file.
    ///
    /// The default is to create a MSI file with the `<product-name>-<version>-<arch>.msi` file
    /// name and extension in the `target\wix` folder. Use this method to override the destination
    /// and file name of the Windows installer.
    pub fn output(mut self, o: Option<&'a str>) -> Self {
        self.output = o;
        self
    }

    /// Renders the template for the license and writes it to stdout.
    fn print_license(&self, license: Template) -> Result<()> {
        let manifest = get_manifest()?;
        self.write_license(&mut io::stdout(), &license, &manifest)?;
        Ok(())
    }

    /// Prints a template to stdout.
    ///
    /// In the case of a license template, the copyright year and holder are filled by the current
    /// year and the manufacturer. The output for a license template is also in the Rich Text
    /// Format (RTF). In the case of a WiX Source (WXS) template, the output is in XML. The
    /// UpgradeCode's and Path Component's GUIDs are generated each time the WXS template is printed. 
    pub fn print_template(self, template: Template) -> Result<()> {
        match template {
            t @ Template::Apache2 => self.print_license(t),
            t @ Template::Gpl3 => self.print_license(t),
            t @ Template::Mit => self.print_license(t),
            Template::Wxs => self.print_wix_source(),
        }
    }

    /// Generates unique GUIDs for appropriate values and writes the rendered template to stdout.
    fn print_wix_source(&self) -> Result<()> {
        self.write_wix_source(&mut io::stdout())
    }

    /// Sets the product name.
    ///
    /// This override the product name determined from the `name` field in the package's
    /// manifest (Cargo.toml).
    pub fn product_name(mut self, p: Option<&'a str>) -> Self {
        self.product_name = p;
        self
    }

    /// Enables or disables signing of the installer after creation with the `signtool`
    /// application.
    pub fn sign(mut self, s: bool) -> Self {
        self.sign = s;
        self
    }

    /// Sets the path to the folder containing the `signtool.exe` file.
    ///
    /// Normally the `signtool.exe` is installed in the `bin` folder of the Windows SDK
    /// installation. THe default is to use the PATH system environment variable. This will
    /// override any value obtained from the environment.
    pub fn sign_path(mut self, s: Option<&'a str>) -> Self {
        self.sign_path = s;
        self
    }

    /// Sets the URL for the timestamp server used when signing an installer.
    ///
    /// The default is to _not_ use a timestamp server, even though it is highly recommended. Use
    /// this method to enable signing with the timestamp.
    pub fn timestamp(mut self, t: Option<&'a str>) -> Self {
        self.timestamp = t;
        self
    }

    /// Creates the necessary sub-folders and files to immediately use the `cargo wix` subcommand to
    /// create an installer for the package.
    pub fn init(self, force: bool) -> Result<()> {
        let mut main_wxs_path = PathBuf::from(WIX);
        if !main_wxs_path.exists() {
            info!("Creating the '{}' directory", main_wxs_path.display());
            fs::create_dir(&main_wxs_path)?;
        }
        main_wxs_path.push(WIX_SOURCE_FILE_NAME);
        main_wxs_path.set_extension(WIX_SOURCE_FILE_EXTENSION);
        if main_wxs_path.exists() && !force {
            return Err(Error::Generic(
                format!("The '{}' file already exists. Use the '--force' flag to overwrite the contents.", 
                    main_wxs_path.display())
            ));
        } else {
            info!("Creating the 'wix\\main.wxs' file");
            let mut main_wxs = File::create(main_wxs_path)?;
            self.write_wix_source(&mut main_wxs)?;
        }
        let manifest = get_manifest()?;
        if self.get_description(&manifest).is_err() {
            warn!("The 'description' field is missing from the package's manifest (Cargo.toml). \
                  Please consider adding the field with a non-empty value to avoid errors during \
                  installer creation.");
        }
        let license_name = self.get_manifest_license_name(&manifest);
        debug!("license_name = {:?}", license_name);
        if let Some(l) = license_name {
            info!("Creating the 'wix\\License.{}' file", RTF_FILE_EXTENSION);
            let license = Template::from_str(&l)?;
            let mut license_path = PathBuf::from(WIX);
            license_path.push("License");
            license_path.set_extension(RTF_FILE_EXTENSION);
            let mut rtf = File::create(license_path)?;
            self.write_license(&mut rtf, &license, &manifest)?;
        } else {
            warn!("Could not generate an appropriate EULA in the Rich Text Format (RTF) from the \
                  project's 'license' field. Please manually create one to avoid errors during \
                  installer creation.");
        }
        if manifest.get("package").and_then(|p| p.as_table())
            .and_then(|t| t.get("license-file")).is_none() {
            let license_file = Path::new(DEFAULT_LICENSE_FILE_NAME);
            if !license_file.exists() {
                warn!("A '{}' file does not exist in the project root. Please consider adding such \
                      a file to avoid errors during installer creation.",
                      DEFAULT_LICENSE_FILE_NAME);
            }
        }
        Ok(())
    }
   
    /// Builds the project using the release profile, creates the installer (msi), and optionally
    /// signs the output. 
    pub fn create(self) -> Result<()> {
        debug!("binary_name = {:?}", self.binary_name);
        debug!("capture_output = {:?}", self.capture_output);
        debug!("description = {:?}", self.description);
        debug!("input = {:?}", self.input);
        debug!("manufacturer = {:?}", self.manufacturer);
        debug!("product_name = {:?}", self.product_name);
        debug!("sign = {:?}", self.sign);
        debug!("timestamp = {:?}", self.timestamp);
        let manifest = get_manifest()?;
        let version = self.get_version(&manifest)?;
        debug!("version = {:?}", version);
        let product_name = self.get_product_name(&manifest)?;
        debug!("product_name = {:?}", product_name);
        let description = self.get_description(&manifest)?;
        debug!("description = {:?}", description);
        let homepage = self.get_homepage(&manifest);
        debug!("homepage = {:?}", homepage);
        let license_name = self.get_license_name(&manifest)?;
        debug!("license_name = {:?}", license_name);
        let license_source = self.get_license_source(&manifest);
        debug!("license_source = {:?}", license_source);
        let manufacturer = self.get_manufacturer(&manifest)?;
        debug!("manufacturer = {}", manufacturer);
        let help_url = self.get_help_url(&manifest);
        debug!("help_url = {:?}", help_url);
        let binary_name = self.get_binary_name(&manifest)?;
        debug!("binary_name = {:?}", binary_name);
        let platform = self.get_platform();
        debug!("platform = {:?}", platform);
        let source_wxs = self.get_wxs_source()?;
        debug!("source_wxs = {:?}", source_wxs);
        let source_wixobj = self.get_source_wixobj(); 
        debug!("source_wixobj = {:?}", source_wixobj);
        let destination_msi = self.get_destination_msi(&product_name, &version, &platform);
        debug!("destination_msi = {:?}", destination_msi);
        if self.no_build {
            warn!("Skipped building the release binary");
        } else {
            // Build the binary with the release profile. If a release binary has already been built, then
            // this will essentially do nothing.
            info!("Building the release binary");
            let mut builder = Command::new(CARGO);
            debug!("builder = {:?}", builder);
            if self.capture_output {
                trace!("Capturing the '{}' output", CARGO);
                builder.stdout(Stdio::null());
                builder.stderr(Stdio::null());
            }
            let status = builder.arg("build").arg("--release").status()?;
            if !status.success() {
                return Err(Error::Command(CARGO, status.code().unwrap_or(100)));
            }
        }
        // Compile the installer
        info!("Compiling the installer");
        let mut compiler = self.get_compiler()?;
        debug!("compiler = {:?}", compiler);
        if self.capture_output {
            trace!("Capturing the '{}' output", WIX_COMPILER);
            compiler.stdout(Stdio::null());
            compiler.stderr(Stdio::null());
        } 
        compiler.arg(format!("-dVersion={}", version))
            .arg(format!("-dPlatform={}", platform))
            .arg(format!("-dProductName={}", product_name))
            .arg(format!("-dBinaryName={}", binary_name))
            .arg(format!("-dDescription={}", description))
            .arg(format!("-dManufacturer={}", manufacturer))
            .arg(format!("-dLicenseName={}", license_name))
            .arg(format!("-dLicenseSource={}", license_source.display()));
        if let Some(h) = help_url {
            trace!("Using '{}' for the help URL", h);
            compiler.arg(format!("-dHelp={}", h));
        } else {
            warn!("A help URL could not be found. Considering adding the 'documentation', \
                  'homepage', or 'repository' fields to the package's manifest.");
        }
        let status = compiler.arg("-o")
            .arg(&source_wixobj)
            .arg(&source_wxs)
            .status().map_err(|err| {
                if err.kind() == ErrorKind::NotFound {
                    Error::Generic(format!(
                        "The compiler application ({}) could not be found in the PATH environment \
                        variable. Please check the WiX Toolset (http://wixtoolset.org/) is \
                        installed and the WiX Toolset's 'bin' folder has been added to the PATH \
                        environment variable, or use the '-B,--bin-path' command line argument or \
                        the {} environment variable.", 
                        WIX_COMPILER,
                        WIX_PATH_KEY
                    ))
                } else {
                    err.into()
                }
            })?;
        if !status.success() {
            return Err(Error::Command(WIX_COMPILER, status.code().unwrap_or(100)));
        }
        // Link the installer
        info!("Linking the installer");
        let mut linker = self.get_linker()?; 
        debug!("linker = {:?}", linker);
        if self.capture_output {
            trace!("Capturing the '{}' output", WIX_LINKER);
            linker.stdout(Stdio::null());
            linker.stderr(Stdio::null());
        }
        let status = linker
            .arg("-ext")
            .arg("WixUIExtension")
            .arg("-cultures:en-us")
            .arg(&source_wixobj)
            .arg("-out")
            .arg(&destination_msi)
            .status().map_err(|err| {
                if err.kind() == ErrorKind::NotFound {
                    Error::Generic(format!(
                        "The linker application ({}) could not be found in the PATH environment \
                        variable. Please check the WiX Toolset (http://wixtoolset.org/) is \
                        installed and the WiX Toolset's 'bin' folder has been added to the PATH \
                        environment variable, or use the '-B,--bin-path' command line argument or \
                        the {} environment variable.", 
                        WIX_LINKER,
                        WIX_PATH_KEY
                    ))
                } else {
                    err.into()
                }
            })?;
        if !status.success() {
            return Err(Error::Command(WIX_LINKER, status.code().unwrap_or(100)));
        }
        // Sign the installer
        if self.sign {
            info!("Signing the installer");
            let mut signer = self.get_signer()?;
            debug!("signer = {:?}", signer);
            if self.capture_output {
                trace!("Capturing the {} output", SIGNTOOL);
                signer.stdout(Stdio::null());
                signer.stderr(Stdio::null());
            }
            signer.arg("sign")
                .arg("/a")
                .arg("/d")
                .arg(format!("{} - {}", product_name, description));
            if let Some(h) = homepage {
                trace!("Using the '{}' URL for the expanded description", h);
                signer.arg("/du").arg(h);
            }
            if let Some(t) = self.timestamp {
                let server = TimestampServer::from_str(&t)?;
                trace!("Using the '{}' timestamp server to sign the installer", server); 
                signer.arg("/t");
                signer.arg(server.url());
            }
            let status = signer.arg(&destination_msi).status().map_err(|err| {
                if err.kind() == ErrorKind::NotFound {
                    Error::Generic(format!(
                        "The {0} application could not be found. Please check the Windows 10 SDK \
                        (https://developer.microsoft.com/en-us/windows/downloads/windows-10-sdk) is \
                        installed and you are using the x64 or x86 Native Build Tools prompt so the \
                        {0} application is available.",
                        SIGNTOOL
                    ))
                } else {
                    err.into()
                }
            })?;
            if !status.success() {
                return Err(Error::Command(SIGNTOOL, status.code().unwrap_or(100)));
            }
        }
        Ok(())
    }

    /// Gets the binary name.
    ///
    /// This is the name of the executable (exe) file.
    fn get_binary_name(&self, manifest: &Value) -> Result<String> {
        if let Some(b) = self.binary_name {
            Ok(b.to_owned())
        } else {
            manifest.get("bin")
                .and_then(|b| b.as_table())
                .and_then(|t| t.get("name")) 
                .and_then(|n| n.as_str())
                .map_or(self.get_product_name(manifest), |s| Ok(String::from(s)))
        }
    }

    /// Gets the command for the compiler application (`candle.exe`).
    fn get_compiler(&self) -> Result<Command> {
        if let Some(mut path) = self.bin_path.map(|s| {
            let mut p = PathBuf::from(s);
            trace!("Using the '{}' path to the WiX Toolset's 'bin' folder for the compiler", p.display());
            p.push(WIX_COMPILER);
            p.set_extension(EXE_FILE_EXTENSION);
            p
        }) {
            if !path.exists() {
                path.pop(); // Remove the `candle` application from the path
                Err(Error::Generic(format!(
                    "The compiler application ('{}') does not exist at the '{}' path specified via \
                    the '-B, --bin-path' command line argument. Please check the path is correct and \
                    the compiler application exists at the path.",
                    WIX_COMPILER, 
                    path.display()
                )))
            } else {
                Ok(Command::new(path))
            }
        } else {
            if let Some(mut path) = env::var_os(WIX_PATH_KEY).map(|s| {
                let mut p = PathBuf::from(s);
                trace!("Using the '{}' path to the WiX Toolset's 'bin' folder for the compiler", p.display());
                p.push(WIX_COMPILER);
                p.set_extension(EXE_FILE_EXTENSION);
                p
            }) {
                if !path.exists() {
                    path.pop(); // Remove the `candle` application from the path
                    Err(Error::Generic(format!(
                        "The compiler application ('{}') does not exist at the '{}' path specified \
                        via the {} environment variable. Please check the path is correct and the \
                        compiler application exists at the path.",
                        WIX_COMPILER,
                        path.display(),
                        WIX_PATH_KEY
                    )))
                } else {
                    Ok(Command::new(path))
                }
            } else {
                Ok(Command::new(WIX_COMPILER))
            }
        }
    }

    /// Gets the copyright holder.
    ///
    /// The default is use the manufacturer.
    fn get_copyright_holder(&self, manifest: &Value) -> Result<String> {
        if let Some(h) = self.copyright_holder {
            Ok(h.to_owned())
        } else {
            self.get_manufacturer(manifest)
        }
    }

    /// Gets the copyright year.
    ///
    /// The default is to use the current year.
    fn get_copyright_year(&self) -> String {
        self.copyright_year
            .map(|y| String::from(y))
            .unwrap_or(Utc::now().year().to_string())
    }

    /// Gets the description.
    ///
    /// If no description is explicitly set using the builder pattern, then the description from
    /// the package's manifest (Cargo.toml) is used.
    fn get_description(&self, manifest: &Value) -> Result<String> {
        self.description.or_else(|| manifest.get("package")
            .and_then(|p| p.as_table())
            .and_then(|t| t.get("description"))
            .and_then(|d| d.as_str()))
            .map(|s| String::from(s))
            .ok_or(Error::Manifest("description"))
    }

    /// Gets the URL of the project's homepage from the manifest (Cargo.toml).
    fn get_homepage(&self, manifest: &Value) -> Option<String> {
        manifest.get("package")
            .and_then(|p| p.as_table())
            .and_then(|t| t.get("homepage"))
            .and_then(|d| d.as_str())
            .map(|s| String::from(s))
    }

    /// Gets the license name.
    ///
    /// If a path to a license file is specified, then the file name will be used. If no license
    /// path is specified, then the file name from the `license-file` field in the package's
    /// manifest is used.
    fn get_license_name(&self, manifest: &Value) -> Result<String> {
        if let Some(l) = self.license_path.map(|s| PathBuf::from(s)) {
            l.file_name()
                .and_then(|f| f.to_str())
                .map(|s| String::from(s))
                .ok_or(Error::Generic(
                    format!("The '{}' license path does not contain a file name.", l.display())
                ))
        } else {
            manifest.get("package")
                .and_then(|p| p.as_table())
                .and_then(|t| t.get("license-file"))
                .and_then(|l| l.as_str())
                .map_or(Ok(String::from("License.txt")), |l| {
                    Path::new(l).file_name()
                        .and_then(|f| f.to_str())
                        .map(|s| String::from(s))
                        .ok_or(Error::Generic(
                            format!("The 'license-file' field value does not contain a file name.")))
                })
        }
    }

    /// Gets the name of the license from the `license` field in the package's manifest
    /// (Cargo.toml).
    fn get_manifest_license_name(&self, manifest: &Value) -> Option<String> {
        manifest.get("package")
            .and_then(|p| p.as_table())
            .and_then(|t| t.get("license"))
            .and_then(|l| l.as_str())
            .and_then(|s| s.split("/").next())
            .map(|s| String::from(s))
    }

    /// Gets the license source file.
    ///
    /// This is the license file that is placed in the installation location alongside the `bin`
    /// folder for the executable (exe) file.
    fn get_license_source(&self, manifest: &Value) -> PathBuf {
        // Order of precedence:
        //
        // 1. CLI (-l,--license)
        // 2. Manifest `license-file`
        // 3. LICENSE file in root
        self.license_path.map(|l| PathBuf::from(l)).unwrap_or(
            manifest.get("package")
                .and_then(|p| p.as_table())
                .and_then(|t| t.get("license-file"))
                .and_then(|l| l.as_str())
                .map(|s| PathBuf::from(s))
                // TODO: Add check that the default license exists
                .unwrap_or(PathBuf::from(DEFAULT_LICENSE_FILE_NAME))
        )
    }

    /// Gets the command for the linker application (`light.exe`).
    fn get_linker(&self) -> Result<Command> {
        if let Some(mut path) = self.bin_path.map(|s| {
            let mut p = PathBuf::from(s);
            trace!("Using the '{}' path to the WiX Toolset 'bin' folder for the linker", p.display());
            p.push(WIX_LINKER);
            p.set_extension(EXE_FILE_EXTENSION);
            p
        }) {
            if !path.exists() {
                path.pop(); // Remove the 'light' application from the path
                Err(Error::Generic(format!(
                    "The linker application ('{}') does not exist at the '{}' path specified via \
                    the '-B, --bin-path' command line argument. Please check the path is correct \
                    and the linker application exists at the path.",
                    WIX_LINKER,
                    path.display()
                )))
            } else {
                Ok(Command::new(path))
            }
        } else {
            if let Some(mut path) = env::var_os(WIX_PATH_KEY).map(|s| {
                let mut p = PathBuf::from(s);
                trace!("Using the '{}' path to the WiX Toolset's 'bin' folder for the linker", p.display());
                p.push(WIX_LINKER);
                p.set_extension(EXE_FILE_EXTENSION);
                p
            }) {
                if !path.exists() {
                    path.pop(); // Remove the `candle` application from the path
                    Err(Error::Generic(format!(
                        "The linker application ('{}') does not exist at the '{}' path specified \
                        via the {} environment variable. Please check the path is correct and the \
                        linker application exists at the path.",
                        WIX_LINKER,
                        path.display(),
                        WIX_PATH_KEY
                    )))
                } else {
                    Ok(Command::new(path))
                }
            } else {
                Ok(Command::new(WIX_LINKER))
            }
        }
    }

    /// Gets the manufacturer.
    ///
    /// The manufacturer is displayed in the Add/Remove Programs (ARP) control panel under the
    /// "Publisher" column. If no manufacturer is explicitly set using the builder pattern, then
    /// the first author from the `authors` field in the package's manifest (Cargo.toml) is used.
    fn get_manufacturer(&self, manifest: &Value) -> Result<String> {
        if let Some(m) = self.manufacturer {
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

    /// Gets the platform.
    fn get_platform(&self) -> Platform {
        if cfg!(target_arch = "x86_64") {
            Platform::X64
        } else {
            Platform::X86
        }
    }

    /// Gets the product name.
    fn get_product_name(&self, manifest: &Value) -> Result<String> {
        if let Some(p) = self.product_name {
            Ok(p.to_owned())
        } else {
            manifest.get("package")
                .and_then(|p| p.as_table())
                .and_then(|t| t.get("name"))
                .and_then(|n| n.as_str())
                .map(|s| String::from(s))
                .ok_or(Error::Manifest("name"))
        }
    }

    /// Gets the command for the `signtool` application.
    fn get_signer(&self) -> Result<Command> {
        if let Some(mut path) = self.sign_path.map(|s| {
            let mut p = PathBuf::from(s);
            trace!("Using the '{}' path to the Windows SDK 'bin' folder for the signer", p.display());
            p.push(SIGNTOOL);
            p.set_extension(EXE_FILE_EXTENSION);
            p
        }) {
            if !path.exists() {
                path.pop(); // Remove the 'signtool' application from the path
                Err(Error::Generic(format!(
                    "The signer application ('{}') does not exist at the '{}' path specified via \
                    the '-S, --sign-path' command line argument. Please check the path is correct and \
                    the signer application exists at the path.",
                    SIGNTOOL, 
                    path.display()
                )))
            } else {
                Ok(Command::new(path))
            }
        } else {
            if let Some(mut path) = env::var_os(SIGNTOOL_PATH_KEY).map(|s| {
                let mut p = PathBuf::from(s);
                trace!("Using the '{}' path to the Windows SDK 'bin' folder for the signer", p.display());
                p.push(SIGNTOOL);
                p.set_extension(EXE_FILE_EXTENSION);
                p
            }) {
                if !path.exists() {
                    path.pop(); // Remove the `signtool` application from the path
                    Err(Error::Generic(format!(
                        "The signer application ('{}') does not exist at the '{}' path specified \
                        via the {} environment variable. Please check the path is correct and the \
                        signer application exists at the path.",
                        SIGNTOOL,
                        path.display(),
                        SIGNTOOL_PATH_KEY
                    )))
                } else {
                    Ok(Command::new(path))
                }
            } else {
                Ok(Command::new(SIGNTOOL))
            }
        }
    }

    /// Gets the destination for the linker.
    fn get_destination_msi(&self, product_name: &str, version: &str, platform: &Platform) -> PathBuf {
        if let Some(o) = self.output {
            PathBuf::from(o)
        } else {
            let mut destination_msi = PathBuf::from("target");
            destination_msi.push(WIX);
            // Do NOT use the `set_extension` method for the MSI path. Since the pkg_version is in X.X.X
            // format, the `set_extension` method will replace the Patch version number and
            // architecture/platform with `msi`.  Instead, just include the extension in the formatted
            // name.
            destination_msi.push(&format!("{}-{}-{}.msi", product_name, version, platform.arch()));
            destination_msi
        }
    }

    /// Gets the destination for the compiler output/linker input.
    fn get_source_wixobj(&self) -> PathBuf {
        let mut source_wixobj = PathBuf::from("target");
        source_wixobj.push(WIX);
        source_wixobj.push(WIX_SOURCE_FILE_NAME);
        source_wixobj.set_extension("wixobj");
        source_wixobj
    }

    /// Gets the URL that appears in the installer's extended details.
    fn get_help_url(&self, manifest: &Value) -> Option<String> {
        manifest.get("package")
            .and_then(|p| p.as_table())
            .and_then(|t| t.get("documentation").or(t.get("homepage")).or(t.get("repository")))
            .and_then(|h| h.as_str())
            .map(|s| String::from(s))
    }

    /// Gets the WiX Source (wxs) file.
    fn get_wxs_source(&self) -> Result<PathBuf> {
        if let Some(p) = self.input.map(|s| PathBuf::from(s)) {
            if p.exists() {
                if p.is_dir() {
                    Err(Error::Generic(format!(
                        "The '{}' path is not a file. Please check the path and ensure it is to \
                        a WiX Source (wxs) file.", 
                        p.display()
                    )))
                } else {
                    trace!("Using the '{}' WiX source file", p.display());
                    Ok(p)
                }
            } else {
                Err(Error::Generic(format!(
                    "The '{0}' file does not exist. Consider using the 'cargo wix --print-template \
                    WXS > {0}' command to create it.", 
                    p.display()
                )))
            }
        } else {
            trace!("Using the default WiX source file");
            let mut main_wxs = PathBuf::from(WIX);
            main_wxs.push(WIX_SOURCE_FILE_NAME);
            main_wxs.set_extension(WIX_SOURCE_FILE_EXTENSION);
            if main_wxs.exists() {
                Ok(main_wxs)
            } else {
               Err(Error::Generic(format!(
                   "The '{0}' file does not exist. Consider using the 'cargo wix --init' command to \
                   create it.", 
                   main_wxs.display()
               )))
            }
        }
    }

    /// Gets the package version.
    fn get_version(&self, cargo_toml: &Value) -> Result<String> {
        cargo_toml.get("package")
            .and_then(|p| p.as_table())
            .and_then(|t| t.get("version"))
            .and_then(|v| v.as_str())
            .map(|s| String::from(s))
            .ok_or(Error::Manifest("version"))
    }

    /// Renders the license in the Rich Text Format (RTF) as an End User's License Agreement (EULA).
    ///
    /// The EULA is automatically included in the Windows installer (msi) and displayed in the license
    /// dialog.
    fn write_license<W: Write>(&self, writer: &mut W, template: &Template, manifest: &Value) -> Result<()> {
        let template = mustache::compile_str(template.to_str())?;
        let data = MapBuilder::new()
            .insert_str("copyright-year", self.get_copyright_year())
            .insert_str("copyright-holder", self.get_copyright_holder(manifest)?)
            .build();
        template.render_data(writer, &data)?;
        Ok(())
    }

    /// Generates unique GUIDs for appropriate values and renders the template.
    fn write_wix_source<W: Write>(&self, writer: &mut W) -> Result<()> {
        let template = mustache::compile_str(Template::Wxs.to_str())?;
        let data = MapBuilder::new()
            .insert_str("upgrade-code-guid", Uuid::new_v4().hyphenated().to_string().to_uppercase())
            .insert_str("path-component-guid", Uuid::new_v4().hyphenated().to_string().to_uppercase())
            .build();
        template.render_data(writer, &data)?;
        Ok(())
    }
}

impl<'a> Default for Wix<'a> {
    fn default() -> Self {
        Wix::new()
    }
}

/// Gets the parsed package's manifest (Cargo.toml).
fn get_manifest() -> Result<Value> {
    let cargo_file_path = Path::new(CARGO_MANIFEST_FILE);
    debug!("cargo_file_path = {:?}", cargo_file_path);
    let mut cargo_file = File::open(cargo_file_path)?;
    let mut cargo_file_content = String::new();
    cargo_file.read_to_string(&mut cargo_file_content)?;
    let manifest = cargo_file_content.parse::<Value>()?;
    Ok(manifest)
}

#[cfg(test)]
mod tests {
    use super::*;

    static COMPLETE_MANIFEST: &str = r#"[package]
name = "cargo-wix"
description = "Build Windows installers using the Wix Toolset"
version = "0.0.2"
authors = ["Christopher Field <cfield2@gmail.com>"]
license = "Apache-2.0/MIT"
repository = "https://github.com/volks73/cargo-wix"
documentation = "https://volks73.github.io/cargo-wix"
homepage = "https://github.com/volks73/cargo-wix"
readme = "README.md"

[[bin]]
name = "cargo-wix"
doc = false

[lib]
name = "cargo_wix""#;

    static MINIMAL_MANIFEST: &str = r#"[package]
name = "minimal-project"
version = "0.0.1"
authors = ["Christopher Field"]"#;

    static LICENSE_FILE_MANIFEST: &str = r#"[package]
license-file = "LICENSE-CUSTOM""#;

    fn complete_manifest() -> Value {
        COMPLETE_MANIFEST.parse::<Value>().unwrap()
    }

    fn minimal_manifest() -> Value {
        MINIMAL_MANIFEST.parse::<Value>().unwrap()
    }

    fn license_file_manifest() -> Value {
        LICENSE_FILE_MANIFEST.parse::<Value>().unwrap()
    }

    #[test]
    fn defaults_are_correct() {
        let wix = Wix::new();
        assert!(wix.bin_path.is_none());
        assert!(wix.binary_name.is_none());
        assert!(wix.capture_output);
        assert!(wix.copyright_holder.is_none());
        assert!(wix.copyright_year.is_none());
        assert!(wix.description.is_none());
        assert!(wix.input.is_none());
        assert!(wix.license_path.is_none());
        assert!(wix.manufacturer.is_none());
        assert!(!wix.no_build);
        assert!(wix.product_name.is_none());
        assert!(!wix.sign);
        assert!(wix.timestamp.is_none());
    }

    #[test]
    fn bin_path_works() {
        let mut expected = PathBuf::from("C:");
        expected.push("WiX Toolset");
        expected.push("bin");
        let wix = Wix::new().bin_path(expected.to_str());
        assert_eq!(wix.bin_path, expected.to_str());
    }

    #[test]
    fn binary_name_works() {
        const EXPECTED: Option<&str> = Some("test");
        let wix = Wix::new().binary_name(EXPECTED);
        assert_eq!(wix.binary_name, EXPECTED);
    }

    #[test]
    fn capture_output_works() {
        let wix = Wix::new().capture_output(false);
        assert!(!wix.capture_output);
    }

    #[test]
    fn copyright_holder_works() {
        const EXPECTED: Option<&str> = Some("Test");
        let wix = Wix::new().copyright_holder(EXPECTED);
        assert_eq!(wix.copyright_holder, EXPECTED);
    }

    #[test]
    fn copyright_year_works() {
        const EXPECTED: Option<&str> = Some("2013");
        let wix = Wix::new().copyright_year(EXPECTED);
        assert_eq!(wix.copyright_year, EXPECTED);
    }

    #[test]
    fn description_works() {
        const EXPECTED: Option<&str> = Some("test description");
        let wix = Wix::new().description(EXPECTED);
        assert_eq!(wix.description, EXPECTED);
    }

    #[test]
    fn input_works() {
        const EXPECTED: Option<&str> = Some("test.wxs");
        let wix = Wix::new().input(EXPECTED);
        assert_eq!(wix.input, EXPECTED);
    }

    #[test]
    fn license_file_works() {
        const EXPECTED: Option<&str> = Some("MIT-LICENSE");
        let wix = Wix::new().license_file(EXPECTED);
        assert_eq!(wix.license_path, EXPECTED);
    }

    #[test]
    fn manufacturer_works() {
        const EXPECTED: Option<&str> = Some("Tester");
        let wix = Wix::new().manufacturer(EXPECTED);
        assert_eq!(wix.manufacturer, EXPECTED);
    }

    #[test]
    fn no_build_works() {
        let wix = Wix::new().no_build(true);
        assert!(wix.no_build);
    }

    #[test]
    fn product_name_works() {
        const EXPECTED: Option<&str> = Some("Test Product Name");
        let wix = Wix::new().product_name(EXPECTED);
        assert_eq!(wix.product_name, EXPECTED);
    }

    #[test]
    fn sign_works() {
        let wix = Wix::new().sign(true);
        assert!(wix.sign);
    }

    #[test]
    fn sign_path_works() {
        let mut expected = PathBuf::from("C:");
        expected.push("Program Files");
        expected.push("Windows Kit");
        expected.push("bin");
        let wix = Wix::new().sign_path(expected.to_str());
        assert_eq!(wix.sign_path, expected.to_str());
    }

    #[test]
    fn timestamp_works() {
        const EXPECTED: Option<&str> = Some("http://timestamp.comodoca.com/");
        let wix = Wix::new().timestamp(EXPECTED);
        assert_eq!(wix.timestamp, EXPECTED);
    }

    #[test]
    fn strip_email_works() {
        const EXPECTED: &str = "Christopher R. Field";
        let re = Regex::new(r"<(.*?)>").unwrap();
        let actual = re.replace_all("Christopher R. Field <user2@example.com>", "");
        assert_eq!(actual.trim(), EXPECTED);
    }

    #[test]
    fn strip_email_works_without_email() {
        const EXPECTED: &str = "Christopher R. Field";
        let re = Regex::new(r"<(.*?)>").unwrap();
        let actual = re.replace_all("Christopher R. Field", "");
        assert_eq!(actual.trim(), EXPECTED);
    }

    #[test]
    fn strip_email_works_with_only_email() {
        const EXPECTED: &str = "user1@example.com";
        let re = Regex::new(r"<(.*?)>").unwrap();
        let actual = re.replace_all("user1@example.com", "");
        assert_eq!(actual.trim(), EXPECTED);
    }

    #[test]
    fn get_binary_name_is_correct_with_default() {
        const EXPECTED: &str = "cargo-wix";
        let wix = Wix::new();
        let actual = wix.get_binary_name(&complete_manifest()).unwrap();
        assert_eq!(&actual, EXPECTED); 
    }

    #[test]
    fn get_binary_name_is_correct_with_some_value() {
        const EXPECTED: &str = "test";
        let wix = Wix::new().binary_name(Some(EXPECTED));
        let actual = wix.get_binary_name(&complete_manifest()).unwrap();
        assert_eq!(&actual, EXPECTED);
    }

    #[test]
    fn get_compiler_is_correct_with_default() {
        let compiler = Wix::new().get_compiler().unwrap();
        assert_eq!(format!("{:?}", compiler), format!("{:?}", Command::new(WIX_COMPILER)));
    }

    fn wix_toolset_bin_folder() -> PathBuf {
        let mut wix_toolset_bin = PathBuf::from("C:\\");
        wix_toolset_bin.push("Program Files (x86)");
        // TODO: Change this to be version independent
        wix_toolset_bin.push("WiX Toolset v3.11");
        wix_toolset_bin.push("bin");
        wix_toolset_bin
    }

    #[test]
    fn get_compiler_is_correct_with_some_value() {
        let test_path = wix_toolset_bin_folder();
        let mut expected = PathBuf::from(&test_path);
        expected.push(WIX_COMPILER);
        expected.set_extension(EXE_FILE_EXTENSION);
        let compiler = Wix::new().bin_path(test_path.to_str()).get_compiler().unwrap();
        assert_eq!(format!("{:?}", compiler), format!("{:?}", Command::new(expected)));
    }

    #[test]
    #[should_panic]
    fn get_compiler_fails_with_nonexistent_path_from_some_value() {
        let test_path = PathBuf::from("C:\\");
        let mut expected = PathBuf::from(&test_path);
        expected.push(WIX_COMPILER);
        expected.set_extension(EXE_FILE_EXTENSION);
        Wix::new().bin_path(test_path.to_str()).get_compiler().unwrap();
    }

    #[test]
    fn get_compiler_is_correct_with_environment_variable() {
        let test_path = wix_toolset_bin_folder();
        env::set_var(WIX_PATH_KEY, &test_path);
        let mut expected = PathBuf::from(&test_path);
        expected.push(WIX_COMPILER);
        expected.set_extension(EXE_FILE_EXTENSION);
        let compiler = Wix::new().get_compiler().unwrap();
        env::remove_var(WIX_PATH_KEY);
        assert_eq!(format!("{:?}", compiler), format!("{:?}", Command::new(expected)));
    }

    #[test]
    #[should_panic]
    fn get_compiler_fails_with_nonexistent_path_from_environment_variable() {
        let test_path = PathBuf::from("C:\\");
        env::set_var(WIX_PATH_KEY, &test_path);
        let mut expected = PathBuf::from(&test_path);
        expected.push(WIX_COMPILER);
        expected.set_extension(EXE_FILE_EXTENSION);
        let result = Wix::new().get_compiler();
        env::remove_var(WIX_PATH_KEY);
        result.unwrap();
    }

    #[test]
    fn get_copyright_holder_is_correct_with_default() {
        let wix = Wix::new();
        let actual = wix.get_copyright_holder(&complete_manifest()).unwrap();
        assert_eq!(&actual, "Christopher Field");
    }

    #[test]
    fn get_copyright_holder_is_correct_with_some_value() {
        const EXPECTED: &str = "Test";
        let wix = Wix::new().copyright_holder(Some(EXPECTED));
        let actual = wix.get_copyright_holder(&complete_manifest()).unwrap();
        assert_eq!(&actual, EXPECTED);
    }

    #[test]
    fn get_copyright_year_is_correct_with_default() {
        let wix = Wix::new();
        let actual = wix.get_copyright_year();
        assert_eq!(actual, Utc::now().year().to_string());
    }

    #[test]
    fn get_copyright_year_is_correct_with_some_value() {
        const EXPECTED: &str = "1980";
        let wix = Wix::new().copyright_year(Some(EXPECTED));
        let actual = wix.get_copyright_year();
        assert_eq!(&actual, EXPECTED);
    }

    #[test]
    fn get_description_is_correct_with_default() {
        const EXPECTED: &str = "Build Windows installers using the Wix Toolset";
        let wix = Wix::new();
        let actual = wix.get_description(&complete_manifest()).unwrap();
        assert_eq!(&actual, EXPECTED);
    }

    #[test]
    fn get_description_is_correct_with_some_value() {
        const EXPECTED: &str = "description";
        let wix = Wix::new().description(Some(EXPECTED));
        let actual = wix.get_description(&complete_manifest()).unwrap();
        assert_eq!(&actual, EXPECTED);
    }

    #[test]
    #[should_panic]
    fn get_description_fails_with_minimal_manifest() {
        Wix::new().get_description(&minimal_manifest()).unwrap();
    }

    #[test]
    fn get_homepage_is_correct_with_default() {
        assert!(Wix::new().get_homepage(&minimal_manifest()).is_none());
    }

    #[test]
    fn get_homepage_is_correct_with_complete_manifest() {
        let actual = Wix::new().get_homepage(&complete_manifest());
        assert_eq!(actual, Some(String::from("https://github.com/volks73/cargo-wix")));
    }

    #[test]
    fn get_license_name_is_correct_with_complete_manifest() {
        let actual = Wix::new().get_license_name(&complete_manifest()).unwrap();
        assert_eq!(&actual, "License.txt");
    }

    #[test]
    fn get_license_name_is_correct_with_minimal_manifest() {
        let actual = Wix::new().get_license_name(&minimal_manifest()).unwrap();
        assert_eq!(&actual, "License.txt");
    }

    #[test]
    fn get_license_name_is_correct_with_license_file_manifest() {
        let actual = Wix::new().get_license_name(&license_file_manifest()).unwrap();
        assert_eq!(&actual, "LICENSE-CUSTOM");
    }

    #[test]
    fn get_license_name_is_correct_with_license_file() {
        const EXPECTED: &str = "License.doc";
        let test_path = env::home_dir().map(|h| {
            h.join(EXPECTED)
        }).unwrap();
        let actual = Wix::new().license_file(test_path.to_str())
            .get_license_name(&license_file_manifest())
            .unwrap();
        assert_eq!(&actual, EXPECTED);
    }

    #[test]
    fn get_manifest_license_name_is_correct_with_complete_manifest() {
        const EXPECTED: &str = "Apache-2.0";
        let actual = Wix::new().get_manifest_license_name(&complete_manifest()).unwrap();
        assert_eq!(&actual, EXPECTED);
    }

    #[test]
    fn get_manifest_license_name_is_none_with_minimal_manifest() {
        let actual = Wix::new().get_manifest_license_name(&minimal_manifest());
        assert!(actual.is_none());
    }

    #[test]
    fn get_license_source_is_correct_with_complete_manifest() {
        let actual = Wix::new().get_license_source(&complete_manifest());
        assert_eq!(actual.to_str().unwrap(), DEFAULT_LICENSE_FILE_NAME);
    }

    #[test]
    fn get_license_source_is_correct_with_minimal_manifest() {
        let actual = Wix::new().get_license_source(&minimal_manifest());
        assert_eq!(actual.to_str().unwrap(), DEFAULT_LICENSE_FILE_NAME);
    }

    #[test]
    fn get_license_source_is_correct_with_some_value() {
        let expected = env::home_dir().map(|h| {
            h.join(DEFAULT_LICENSE_FILE_NAME)
        }).unwrap();
        let actual = Wix::new().license_file(expected.to_str())
            .get_license_source(&minimal_manifest());
        assert_eq!(actual, expected);
    }

    #[test]
    fn get_linker_is_correct_with_default() {
        let linker = Wix::new().get_linker().unwrap();
        assert_eq!(format!("{:?}", linker), format!("{:?}", Command::new(WIX_LINKER)));
    }

    #[test]
    fn get_linker_is_correct_with_some_value() {
        let test_path = wix_toolset_bin_folder();
        let mut expected = PathBuf::from(&test_path);
        expected.push(WIX_LINKER);
        expected.set_extension(EXE_FILE_EXTENSION);
        let linker = Wix::new().bin_path(test_path.to_str()).get_linker().unwrap();
        assert_eq!(format!("{:?}", linker), format!("{:?}", Command::new(expected)));
    }

    #[test]
    #[should_panic]
    fn get_linker_fails_with_nonexistent_path_from_some_value() {
        let test_path = PathBuf::from("C:\\");
        let mut expected = PathBuf::from(&test_path);
        expected.push(WIX_LINKER);
        expected.set_extension(EXE_FILE_EXTENSION);
        Wix::new().bin_path(test_path.to_str()).get_linker().unwrap();
    }

    #[test]
    fn get_linker_is_correct_with_environment_variable() {
        let test_path = wix_toolset_bin_folder();
        env::set_var(WIX_PATH_KEY, &test_path);
        let mut expected = PathBuf::from(&test_path);
        expected.push(WIX_LINKER);
        expected.set_extension(EXE_FILE_EXTENSION);
        let linker = Wix::new().get_linker().unwrap();
        env::remove_var(WIX_PATH_KEY);
        assert_eq!(format!("{:?}", linker), format!("{:?}", Command::new(expected)));
    }

    #[test]
    #[should_panic]
    fn get_linker_fails_with_nonexistent_path_from_environment_variable() {
        let test_path = PathBuf::from("C:\\");
        env::set_var(WIX_PATH_KEY, &test_path);
        let mut expected = PathBuf::from(&test_path);
        expected.push(WIX_LINKER);
        expected.set_extension(EXE_FILE_EXTENSION);
        let result = Wix::new().get_linker();
        env::remove_var(WIX_PATH_KEY);
        result.unwrap();
    }

    #[test]
    fn get_manufacturer_is_correct_with_complete_manifest() {
        let actual = Wix::new().get_manufacturer(&complete_manifest()).unwrap();
        assert_eq!(&actual, "Christopher Field");
    }

    #[test]
    fn get_manufacturer_is_correct_with_minimal_manifest() {
        let actual = Wix::new().get_manufacturer(&minimal_manifest()).unwrap();
        assert_eq!(&actual, "Christopher Field");
    }

    #[test]
    fn get_manufacturer_is_correct_with_some_value() {
        const EXPECTED: &str = "Test Manufacturer";
        let actual = Wix::new().manufacturer(Some(EXPECTED))
            .get_manufacturer(&complete_manifest()).unwrap();
        assert_eq!(&actual, EXPECTED);
    }

    #[test]
    fn get_product_name_is_correct_with_complete_manifest() {
        let actual = Wix::new().get_product_name(&complete_manifest()).unwrap();
        assert_eq!(&actual, "cargo-wix");
    }

    #[test]
    fn get_product_name_is_correct_with_minimal_manifest() {
        let actual = Wix::new().get_product_name(&minimal_manifest()).unwrap();
        assert_eq!(&actual, "minimal-project");
    }

    #[test]
    fn get_product_name_is_correct_with_some_value() {
        const EXPECTED: &str = "Test Product Name";
        let actual = Wix::new().product_name(Some(EXPECTED))
            .get_product_name(&complete_manifest()).unwrap();
        assert_eq!(&actual, EXPECTED);
    }

    #[test]
    fn get_signer_is_correct_with_default() {
        let signer = Wix::new().get_signer().unwrap();
        assert_eq!(format!("{:?}", signer), format!("{:?}", Command::new(SIGNTOOL)));
    }

    fn windows_10_bin_folder() -> PathBuf {
        let mut windows_10_bin = PathBuf::from("C:\\");
        windows_10_bin.push("Program Files (x86)");
        windows_10_bin.push("Windows Kits");
        windows_10_bin.push("10");
        windows_10_bin.push("bin");
        let version_folder = fs::read_dir(&windows_10_bin).unwrap().filter_map(|entry| {
            let path = entry.unwrap().path();
            if path.is_dir() {
                Some(path)
            } else {
                None
            }
        }).find(|path| {
            let name = path.file_name().unwrap();
            name != "arm" && name != "arm64" && name != "x64" && name != "x86" 
        }).unwrap();
        windows_10_bin.push(version_folder);
        windows_10_bin.push("x64");
        windows_10_bin
    }

    #[test]
    fn get_signer_is_correct_with_some_value() {
        let test_path = windows_10_bin_folder();
        let mut expected = PathBuf::from(&test_path);
        expected.push(SIGNTOOL);
        expected.set_extension(EXE_FILE_EXTENSION);
        let signer = Wix::new().sign_path(test_path.to_str()).get_signer().unwrap();
        assert_eq!(format!("{:?}", signer), format!("{:?}", Command::new(expected)));
    }

    #[test]
    #[should_panic]
    fn get_signer_fails_with_nonexistent_path_from_some_value() {
        let test_path = PathBuf::from("C:\\");
        let mut expected = PathBuf::from(&test_path);
        expected.push(SIGNTOOL);
        expected.set_extension(EXE_FILE_EXTENSION);
        Wix::new().sign_path(test_path.to_str()).get_signer().unwrap();
    }

    #[test]
    fn get_signer_is_correct_with_environment_variable() {
        let test_path = windows_10_bin_folder();
        env::set_var(SIGNTOOL_PATH_KEY, &test_path);
        let mut expected = PathBuf::from(&test_path);
        expected.push(SIGNTOOL);
        expected.set_extension(EXE_FILE_EXTENSION);
        let signer = Wix::new().get_signer().unwrap();
        env::remove_var(SIGNTOOL_PATH_KEY);
        assert_eq!(format!("{:?}", signer), format!("{:?}", Command::new(expected)));
    }

    #[test]
    #[should_panic]
    fn get_signer_fails_with_nonexistent_path_from_environment_variable() {
        let test_path = PathBuf::from("C:\\");
        env::set_var(SIGNTOOL_PATH_KEY, &test_path);
        let mut expected = PathBuf::from(&test_path);
        expected.push(SIGNTOOL);
        expected.set_extension(EXE_FILE_EXTENSION);
        let result = Wix::new().get_signer();
        env::remove_var(SIGNTOOL_PATH_KEY);
        result.unwrap();
    }

    #[test]
    fn get_destination_msi_is_correct_with_defaults() {
        const PRODUCT_NAME: &str = "test";
        const VERSION: &str = "1.2.3";
        const PLATFORM: Platform = Platform::X64;
        let mut expected = PathBuf::from("target");
        expected.push(WIX);
        expected.push(format!("{}-{}-{}.msi", PRODUCT_NAME, VERSION, PLATFORM.arch()));
        let actual = Wix::new().get_destination_msi(PRODUCT_NAME, VERSION, &PLATFORM);
        assert_eq!(actual, expected);
    }

    #[test]
    fn get_destination_msi_is_correct_with_some_value() {
        const PRODUCT_NAME: &str = "test";
        const VERSION: &str = "1.2.3";
        const PLATFORM: Platform = Platform::X64;
        let mut expected = PathBuf::from("C:");
        expected.push("output");
        expected.push("installer.msi");
        let actual = Wix::new().output(expected.to_str())
            .get_destination_msi(PRODUCT_NAME, VERSION, &PLATFORM);
        assert_eq!(actual, expected);
    }
}

