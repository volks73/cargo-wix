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

//! # cargo-wix
//!
//! The goal of the cargo-wix project and `cargo wix` subcommand is to make it easy to create
//! a Windows installer (msi) for any Rust project.
//!
//! ## Quick Start
//!
//! Ensure the [WiX Toolset](http://wixtoolset.org) is installed and the `C:\Program Files\WiX
//! Toolset\bin` folder has been added to the PATH system environment variable. Then start
//! a commmand prompt (cmd.exe) and execute the following commands:
//!
//! ```dos
//! C:\>cargo install cargo-wix
//! C:\>cd Path\To\Project
//! C:\Path\To\Project\>cargo wix --init
//! C:\Path\To\Project\>cargo wix
//! ```
//!
//! The Windows installer (msi) will be in the `C:\Path\To\Project\target\wix` folder.
//!
//! ## Concepts
//!
//! The cargo-wix project is primarily implemented as a cargo subcommand, but the core
//! functionality is provided in a library (crate). Documentation for the binary and Command Line
//! Interface (CLI) are provided with the `cargo wix --help` command, but documentation is provided
//! here for the concepts and core functionality that govern the subcommand.
//!
//! The cargo-wix binary, and related `cargo wix` subcommand, use the WiX Toolset and
//! [SignTool](https://msdn.microsoft.com/en-us/library/windows/desktop/aa387764(v=vs.85).aspx)
//! application available in the [Windows 10
//! SDK](https://developer.microsoft.com/en-us/windows/downloads/windows-10-sdk). These are
//! obviously Windows-only applications, so while the crate and binary can be built on any platform
//! supported by the [Rust](https://www.rust-lang.org) programming language, the `cargo wix`
//! subcommand is only really useful on a Windows machine.
//!
//! The WiX Toolset provides a compiler (`candle.exe`) and linker (`light.exe`). These can be found
//! in the `bin` directory of the installation location for the WiX Toolset. This subcommand uses
//! these two applications with the `std::process::Command` module to create an installer. The WiX
//! Toolset requires a WiX Source (wxs) file, which is an XML file. A template is provided with
//! this subcommand that attempts to meet the majority of use cases for developers, so extensive
//! knowledge of the WiX Toolset and Windows installer technologies is not required (but always
//! recommended). Modification of the template is encouraged, but please consult the WiX Toolset's
//! extensive documentation and tutorials for information about writing, customizing, and using wxs
//! files. The documentation here is only for this subcommand.
//!
//! The [template](https://github.com/volks73/cargo-wix/blob/master/src/template.wxs) is embedded
//! in the binary installation of the subcommand and it can be printed using the `cargo wix
//! --print-template` command from the command prompt (cmd.exe). Note, each time the `cargo wix
//! --print-template` command is invoked, new GUIDs are generated for fields that require them.
//! Thus, a developer does not need to worry about generating GUIDs and can begin using the
//! template immediately with this subcommand or the WiX Toolset's `candle.exe` and `light.exe`
//! applications.

#[macro_use] extern crate log;
extern crate mustache;
extern crate regex;
extern crate toml;
extern crate uuid;

use mustache::MapBuilder;
use std::default::Default;
use regex::Regex;
use std::env;
use std::error::Error as StdError;
use std::fmt;
use std::fs::{self, File};
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::str::FromStr;
use toml::Value;
use uuid::Uuid;

const CARGO_MANIFEST_FILE: &str = "Cargo.toml";
const CARGO: &str = "cargo";
const DEFAULT_LICENSE_FILE_NAME: &str = "LICENSE";
const SIGNTOOL: &str = "signtool";
const WIX: &str = "wix";
const WIX_COMPILER: &str = "candle";
const WIX_LINKER: &str = "light";
const WIX_PATH_KEY: &str = "WIX_PATH";
const WIX_SOURCE_FILE_EXTENSION: &str = "wxs";
const WIX_SOURCE_FILE_NAME: &str = "main";

/// The template, or example, WiX Source (WXS) file.
static TEMPLATE: &str = include_str!("template.wxs");

/// Generates unique GUIDs for appropriate values in the template and renders to a writer.
fn write_template<W: Write>(writer: &mut W) -> Result<(), Error> {
    let template = mustache::compile_str(TEMPLATE)?;
    let data = MapBuilder::new()
        .insert_str("upgrade-code-guid", Uuid::new_v4().hyphenated().to_string().to_uppercase())
        .insert_str("path-component-guid", Uuid::new_v4().hyphenated().to_string().to_uppercase())
        .build();
    template.render_data(writer, &data)?;
    Ok(())
}

/// Generates unique GUIDs for appropriate values in the template and prints to stdout.
pub fn print_template() -> Result<(), Error> {
    write_template(&mut io::stdout())
}

/// Creates the necessary sub-folders and files to immediately use the `cargo wix` subcommand to
/// create an installer for the package.
pub fn init(force: bool) -> Result<(), Error> {
    let mut main_wxs_path = PathBuf::from(WIX);
    if !main_wxs_path.exists() {
        fs::create_dir(&main_wxs_path)?;
    }
    main_wxs_path.push(WIX_SOURCE_FILE_NAME);
    main_wxs_path.set_extension(WIX_SOURCE_FILE_EXTENSION);
    if main_wxs_path.exists() && !force {
        Err(Error::Generic(
            format!("The '{}' file already exists. Use the '--force' flag to overwrite the contents.", 
                main_wxs_path.display())
        ))
    } else {
        let mut main_wxs = File::create(main_wxs_path)?;
        write_template(&mut main_wxs)?;
        Ok(())
    }
}

/// The error type for cargo-wix-related operations and associated traits.
///
/// Errors mostly originate from the dependencies, but custom instances of `Error` can be created
/// with the `Generic` variant and a message.
#[derive(Debug)]
pub enum Error {
    /// A command operation failed.
    Command(&'static str, i32),
    /// A generic or custom error occurred. The message should contain the detailed information.
    Generic(String),
    /// An I/O operation failed.
    Io(io::Error),
    /// A needed field within the `Cargo.toml` manifest could not be found.
    Manifest(String),
    /// An error occurred with rendering the template using the mustache renderer.
    Mustache(mustache::Error),
    /// Parsing of the `Cargo.toml` manifest failed.
    Toml(toml::de::Error),
}

impl Error {
    /// Gets an error code related to the error.
    ///
    /// This is useful as a return, or exit, code for a command line application, where a non-zero
    /// integer indicates a failure in the application. it can also be used for quickly and easily
    /// testing equality between two errors.
    pub fn code(&self) -> i32 {
        match *self{
            Error::Command(..) => 1,
            Error::Generic(..) => 2,
            Error::Io(..) => 3,
            Error::Manifest(..) => 4,
            Error::Mustache(..) => 5,
            Error::Toml(..) => 6,
        }
    }
}

impl StdError for Error {
    fn description(&self) -> &str {
        match *self {
            Error::Command(..) => "Command",
            Error::Generic(..) => "Generic",
            Error::Io(..) => "Io",
            Error::Manifest(..) => "Manifest",
            Error::Mustache(..) => "Mustache",
            Error::Toml(..) => "TOML",
        }
    }

    fn cause(&self) -> Option<&StdError> {
        match *self {
            Error::Io(ref err) => Some(err),
            Error::Toml(ref err) => Some(err),
            Error::Mustache(ref err) => Some(err),
            _ => None
        }
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            Error::Command(ref command, ref code) => 
                write!(f, "The '{}' application failed with exit code = {}. Consider using the '--nocapture' flag to obtain more information.", command, code),
            Error::Generic(ref msg) => write!(f, "{}", msg),
            Error::Io(ref err) => write!(f, "{}", err),
            Error::Manifest(ref var) => 
                write!(f, "No '{}' field found in the package's manifest (Cargo.toml)", var),
            Error::Mustache(ref err) => write!(f, "{}", err),
            Error::Toml(ref err) => write!(f, "{}", err),
        }
    }
}

impl From<io::Error> for Error {
    fn from(err: io::Error) -> Error {
        Error::Io(err)
    }
}

impl From<toml::de::Error> for Error {
    fn from(err: toml::de::Error) -> Error {
        Error::Toml(err)
    }
}

impl From<mustache::Error> for Error {
    fn from(err: mustache::Error) -> Error {
        Error::Mustache(err)
    }
}

/// The different values for the `Platform` attribute of the `Package` element.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Platform {
    /// The `x86` WiX Toolset value.
    X86,
    /// The `x64` WiX Toolset value.
    X64,
}

impl Platform {
    /// Gets the name of the platform as an architecture string as used in Rust toolchains.
    ///
    /// This is different from the string used in WiX Source (wxs) files. This is the string
    /// commonly used for the `target_arch` conditional compilation attribute. To get the string
    /// recognized in wxs files, use `format!("{}", Platform::X86)`.
    ///
    /// # Example
    ///
    /// ```rust
    /// extern crate cargo_wix;
    /// 
    /// use cargo_wix::Platform;
    ///
    /// fn main() {
    ///     assert_eq!(Platform::X86.arch(), "i686");
    ///     assert_eq!(Platform::X64.arch(), "x86_64");
    /// }
    /// ```
    pub fn arch(&self) -> &'static str {
        match *self {
            Platform::X86 => "i686",
            Platform::X64 => "x86_64",
        }
    }
}

impl fmt::Display for Platform {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            Platform::X86 => write!(f, "x86"),
            Platform::X64 => write!(f, "x64"),
        }
    }
}

impl Default for Platform {
    fn default() -> Self {
        if cfg!(target_arch = "x86_64") {
            Platform::X64
        } else {
            Platform::X86
        }
    }
}

/// The aliases for the URLs to different Microsoft Authenticode timestamp servers.
#[derive(Debug, Clone, PartialEq)]
pub enum TimestampServer {
    /// A URL to a timestamp server.
    Custom(String),
    /// The alias for the Comodo timestamp server.
    Comodo,
    /// The alias for the Verisign timestamp server.
    Verisign,
}

impl TimestampServer {
    /// Gets the URL of the timestamp server for an alias.
    pub fn url(&self) -> &str {
        match *self {
            TimestampServer::Custom(ref url) => url,
            TimestampServer::Comodo => "http://timestamp.comodoca.com/",
            TimestampServer::Verisign => "http://timestamp.verisign.com/scripts/timstamp.dll",
        }
    }
}
 
impl fmt::Display for TimestampServer {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.url())
    }
}

impl FromStr for TimestampServer {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().trim() {
            "comodo" => Ok(TimestampServer::Comodo),
            "verisign" => Ok(TimestampServer::Verisign),
            u @ _ => Ok(TimestampServer::Custom(String::from(u)))
        }
    }
}

/// A builder for running the subcommand.
#[derive(Debug, Clone)]
pub struct Wix {
    bin_path: Option<PathBuf>,
    binary_name: Option<String>,
    capture_output: bool,
    description: Option<String>,
    input: Option<PathBuf>,
    license_path: Option<PathBuf>,
    manufacturer: Option<String>,
    product_name: Option<String>,
    sign: bool,
    sign_path: Option<PathBuf>,
    timestamp: Option<String>,
}

impl Wix {
    /// Creates a new `Wix` instance.
    pub fn new() -> Self {
        Wix {
            bin_path: None,
            binary_name: None,
            capture_output: true,
            description: None,
            input: None,
            license_path: None,
            manufacturer: None,
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
    pub fn bin_path(mut self, b: Option<&str>) -> Self {
        self.bin_path = b.map(|s| PathBuf::from(s));
        self
    }

    /// Sets the binary name.
    ///
    /// This overrides the binary name determined from the package's manifest (Cargo.toml).
    pub fn binary_name(mut self, b: Option<&str>) -> Self {
        self.binary_name = b.map(|s| String::from(s));
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

    /// Sets the description.
    ///
    /// This override the description determined from the `description` field in the package's
    /// manifest (Cargo.toml).
    pub fn description(mut self, d: Option<&str>) -> Self {
        self.description = d.map(|s| String::from(s));
        self
    }

    /// Sets the path to a file to be used as the WiX Source (wxs) file instead of `wix\main.rs`.
    pub fn input(mut self, i: Option<&str>) -> Self {
        self.input = i.map(|i| PathBuf::from(i));
        self
    }
    
    /// Sets the path to a file to used as the `License.txt` file within the installer.
    ///
    /// The `License.txt` file is installed into the installation location along side the `bin`
    /// folder. Note, the file can be in any format with any name, but it is automatically renamed
    /// to `License.txt` during creation of the installer.
    pub fn license_file(mut self, l: Option<&str>) -> Self {
        self.license_path = l.map(|l| PathBuf::from(l));
        self
    }

    /// Overrides the first author in the `authors` field of the package's manifest (Cargo.toml) as
    /// the manufacturer within the installer.
    pub fn manufacturer(mut self, m: Option<&str>) -> Self {
        self.manufacturer = m.map(|s| String::from(s));
        self
    }

    /// Sets the product name.
    ///
    /// This override the product name determined from the `name` field in the package's
    /// manifest (Cargo.toml).
    pub fn product_name(mut self, p: Option<&str>) -> Self {
        self.product_name = p.map(|s| String::from(s));
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
    pub fn sign_path(mut self, s: Option<&str>) -> Self {
        self.sign_path = s.map(|s| PathBuf::from(s));
        self
    }

    /// Sets the URL for the timestamp server used when signing an installer.
    ///
    /// The default is to _not_ use a timestamp server, even though it is highly recommended. Use
    /// this method to enable signing with the timestamp.
    pub fn timestamp(mut self, t: Option<&str>) -> Self {
        self.timestamp = t.map(|t| String::from(t));
        self
    }

    /// Runs the subcommand to build the release binary, compile, link, and possibly sign the installer
    /// (msi).
    pub fn run(self) -> Result<(), Error> {
        debug!("binary_name = {:?}", self.binary_name);
        debug!("capture_output = {:?}", self.capture_output);
        debug!("description = {:?}", self.description);
        debug!("input = {:?}", self.input);
        debug!("manufacturer = {:?}", self.manufacturer);
        debug!("product_name = {:?}", self.product_name);
        debug!("sign = {:?}", self.sign);
        debug!("timestamp = {:?}", self.timestamp);
        let cargo_file_path = Path::new(CARGO_MANIFEST_FILE);
        debug!("cargo_file_path = {:?}", cargo_file_path);
        let mut cargo_file = File::open(cargo_file_path)?;
        let mut cargo_file_content = String::new();
        cargo_file.read_to_string(&mut cargo_file_content)?;
        let cargo_values = cargo_file_content.parse::<Value>()?;
        let pkg_version = cargo_values
            .get("package")
            .and_then(|p| p.as_table())
            .and_then(|t| t.get("version"))
            .and_then(|v| v.as_str())
            .ok_or(Error::Manifest(String::from("version")))?;
        debug!("pkg_version = {:?}", pkg_version);
        let product_name = if let Some(p) = self.product_name {
            Ok(p) 
        } else {
            cargo_values.get("package")
                .and_then(|p| p.as_table())
                .and_then(|t| t.get("name"))
                .and_then(|n| n.as_str())
                .map(|s| String::from(s))
                .ok_or(Error::Manifest(String::from("name")))
        }?;
        debug!("product_name = {:?}", product_name);
        let description = if let Some(d) = self.description {
            Ok(d)
        } else {
            cargo_values.get("package")
                .and_then(|p| p.as_table())
                .and_then(|t| t.get("description"))
                .and_then(|d| d.as_str())
                .map(|s| String::from(s))
                .ok_or(Error::Manifest(String::from("description")))
        }?;
        debug!("description = {:?}", description);
        let homepage = cargo_values.get("package")
            .and_then(|p| p.as_table())
            .and_then(|t| t.get("homepage"))
            .and_then(|d| d.as_str());
        debug!("homepage = {:?}", homepage);
        let license_name = if let Some(ref l) = self.license_path {
            l.file_name()
                .and_then(|f| f.to_str())
                .map(|s| String::from(s))
                .ok_or(Error::Generic(
                    format!("The '{}' license path does not contain a file name.", l.display())
                ))
        } else {
            cargo_values.get("package")
                .and_then(|p| p.as_table())
                .and_then(|t| t.get("license-file"))
                .and_then(|l| l.as_str())
                .and_then(|s| Path::new(s).file_name().and_then(|f| f.to_str()))
                .or(Some("License.txt"))
                .map(|s| String::from(s))
                .ok_or(Error::Generic(
                    format!("The 'license-file' field value does not contain a file name.")
                )) 
        }?;
        debug!("license_name = {:?}", license_name);
        let license_source = self.license_path.unwrap_or(
            // TODO: Add generation of license file from `license` field
            cargo_values.get("package")
                .and_then(|p| p.as_table())
                .and_then(|t| t.get("license-file"))
                .and_then(|l| l.as_str())
                .map(|s| PathBuf::from(s))
                .unwrap_or(PathBuf::from(DEFAULT_LICENSE_FILE_NAME))
        );
        debug!("license_source = {:?}", license_source);
        let manufacturer = if let Some(m) = self.manufacturer {
            Ok(m)
        } else {
            cargo_values.get("package")
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
                .ok_or(Error::Manifest(String::from("authors")))
        }?;
        debug!("manufacturer = {}", manufacturer);
        let help_url = cargo_values
            .get("package")
            .and_then(|p| p.as_table())
            .and_then(|t| t.get("documentation").or(t.get("homepage")).or(t.get("repository")))
            .and_then(|h| h.as_str())
            .ok_or(Error::Manifest(String::from("documentation")))?;
        let binary_name = self.binary_name.unwrap_or(
            cargo_values.get("bin")
                .and_then(|b| b.as_table())
                .and_then(|t| t.get("name")) 
                .and_then(|n| n.as_str())
                .map(|s| String::from(s))
                .unwrap_or(product_name.clone())
        );
        debug!("binary_name = {:?}", binary_name);
        let platform = if cfg!(target_arch = "x86_64") {
            Platform::X64
        } else {
            Platform::X86
        };
        debug!("platform = {:?}", platform);
        let main_wxs = if let Some(p) = self.input {
            if p.exists() {
                if p.is_dir() {
                    Err(Error::Generic(format!("The '{}' path is not a file. Please check the path and ensure it is to a WiX Source (wxs) file.", p.display())))
                } else {
                    trace!("Using the '{}' WiX source file", p.display());
                    Ok(p)
                }
            } else {
                Err(Error::Generic(format!("The '{0}' file does not exist. Consider using the 'cargo wix --print-template > {0}' command to create it.", p.display())))
            }
        } else {
            trace!("Using the default WiX source file");
            let mut main_wxs = PathBuf::from(WIX);
            main_wxs.push(WIX_SOURCE_FILE_NAME);
            main_wxs.set_extension(WIX_SOURCE_FILE_EXTENSION);
            if main_wxs.exists() {
                Ok(main_wxs)
            } else {
               Err(Error::Generic(format!("The '{0}' file does not exist. Consider using the 'cargo wix --init' command to create it.", main_wxs.display())))
            }
        }?;
        debug!("main_wxs = {:?}", main_wxs);
        let mut main_wixobj = PathBuf::from("target");
        main_wixobj.push(WIX);
        main_wixobj.push("build");
        main_wixobj.push(WIX_SOURCE_FILE_NAME);
        main_wixobj.set_extension("wixobj");
        debug!("main_wixobj = {:?}", main_wixobj);
        let mut main_msi = PathBuf::from("target");
        main_msi.push(WIX);
        // Do NOT use the `set_extension` method for the MSI path. Since the pkg_version is in X.X.X
        // format, the `set_extension` method will replace the Patch version number and
        // architecture/platform with `msi`.  Instead, just include the extension in the formatted
        // name.
        main_msi.push(&format!("{}-{}-{}.msi", product_name, pkg_version, platform.arch()));
        debug!("main_msi = {:?}", main_msi);
        // Build the binary with the release profile. If a release binary has already been built, then
        // this will essentially do nothing.
        info!("Building release binary");
        let mut builder = Command::new(CARGO);
        if self.capture_output {
            trace!("Capturing the '{}' output", CARGO);
            builder.stdout(Stdio::null());
            builder.stderr(Stdio::null());
        }
        let status = builder.arg("build").arg("--release").status()?;
        if !status.success() {
            return Err(Error::Command(CARGO, status.code().unwrap_or(0)));
        }
        // Compile the installer
        info!("Compiling installer");
        let mut compiler = if let Some(ref b) = self.bin_path {
            trace!("Using the '{}' path to the WiX Toolset compiler", b.display());
            Command::new(b.join(WIX_COMPILER))
        } else {
            env::var(WIX_PATH_KEY).map(|p| {
                Command::new(PathBuf::from(p).join(WIX_COMPILER))
            }).unwrap_or(Command::new(WIX_COMPILER))
        };
        debug!("compiler = {:?}", compiler);
        if self.capture_output {
            trace!("Capturing the '{}' output", WIX_COMPILER);
            compiler.stdout(Stdio::null());
            compiler.stderr(Stdio::null());
        } 
        let status = compiler
            .arg(format!("-dVersion={}", pkg_version))
            .arg(format!("-dPlatform={}", platform))
            .arg(format!("-dProductName={}", product_name))
            .arg(format!("-dBinaryName={}", binary_name))
            .arg(format!("-dDescription={}", description))
            .arg(format!("-dManufacturer={}", manufacturer))
            .arg(format!("-dLicenseName={}", license_name))
            .arg(format!("-dLicenseSource={}", license_source.display()))
            .arg(format!("-dHelp={}", help_url))
            .arg("-o")
            .arg(&main_wixobj)
            .arg(&main_wxs)
            .status()?;
        if !status.success() {
            return Err(Error::Command(WIX_COMPILER, status.code().unwrap_or(0)));
        }
        // Link the installer
        info!("Linking the installer");
        let mut linker = if let Some(ref b) = self.bin_path {
            trace!("Using the '{}' path to the WiX Toolset linker", b.display());
            Command::new(b.join(WIX_LINKER))
        } else {
            env::var(WIX_PATH_KEY).map(|p| {
                Command::new(PathBuf::from(p).join(WIX_LINKER))
            }).unwrap_or(Command::new(WIX_LINKER))
        };
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
            .arg(&main_wixobj)
            .arg("-out")
            .arg(&main_msi)
            .status()?;
        if !status.success() {
            return Err(Error::Command(WIX_LINKER, status.code().unwrap_or(0)));
        }
        // Sign the installer
        if self.sign {
            info!("Signing the installer");
            let mut signer = if let Some(ref s) = self.sign_path {
                trace!("Using the '{}' path to the Windows SDK signtool", s.display());
                Command::new(s.join(SIGNTOOL))
            } else {
                Command::new(SIGNTOOL)
            };
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
            let status = signer.arg(&main_msi).status()?;
            if !status.success() {
                return Err(Error::Command(SIGNTOOL, status.code().unwrap_or(0)));
            }
        }
        Ok(())
    }
}

impl Default for Wix {
    fn default() -> Self {
        Wix::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_are_correct() {
        let wix = Wix::new();
        assert_eq!(wix.bin_path, None);
        assert_eq!(wix.binary_name, None);
        assert!(wix.capture_output);
        assert_eq!(wix.description, None);
        assert_eq!(wix.input, None);
        assert_eq!(wix.license_path, None);
        assert_eq!(wix.manufacturer, None);
        assert_eq!(wix.product_name, None);
        assert!(!wix.sign);
        assert_eq!(wix.timestamp, None);
    }

    #[test]
    fn bin_path_works() {
        const EXPECTED: &str = "C:\\WiX Toolset\\bin";
        let wix = Wix::new().bin_path(Some(EXPECTED));
        assert_eq!(wix.bin_path, Some(PathBuf::from(EXPECTED)));
    }

    #[test]
    fn binary_name_works() {
        const EXPECTED: &str = "test";
        let wix = Wix::new().binary_name(Some(EXPECTED));
        assert_eq!(wix.binary_name, Some(String::from(EXPECTED)));
    }

    #[test]
    fn capture_output_works() {
        let wix = Wix::new().capture_output(false);
        assert!(!wix.capture_output);
    }

    #[test]
    fn description_works() {
        const EXPECTED: &str = "test description";
        let wix = Wix::new().description(Some(EXPECTED));
        assert_eq!(wix.description, Some(String::from(EXPECTED)));
    }

    #[test]
    fn input_works() {
        const EXPECTED: &str = "test.wxs";
        let wix = Wix::new().input(Some(EXPECTED));
        assert_eq!(wix.input, Some(PathBuf::from(EXPECTED)));
    }

    #[test]
    fn license_file_works() {
        const EXPECTED: &str = "MIT-LICENSE";
        let wix = Wix::new().license_file(Some(EXPECTED));
        assert_eq!(wix.license_path, Some(PathBuf::from(EXPECTED)));
    }

    #[test]
    fn manufacturer_works() {
        const EXPECTED: &str = "Tester";
        let wix = Wix::new().manufacturer(Some(EXPECTED));
        assert_eq!(wix.manufacturer, Some(String::from(EXPECTED)));
    }

    #[test]
    fn product_name_works() {
        const EXPECTED: &str = "Test Product Name";
        let wix = Wix::new().product_name(Some(EXPECTED));
        assert_eq!(wix.product_name, Some(String::from(EXPECTED)));
    }

    #[test]
    fn sign_works() {
        let wix = Wix::new().sign(true);
        assert!(wix.sign);
    }

    #[test]
    fn timestamp_works() {
        const EXPECTED: &str = "http://timestamp.comodoca.com/";
        let wix = Wix::new().timestamp(Some(EXPECTED));
        assert_eq!(wix.timestamp, Some(String::from(EXPECTED)));
    }

    #[test]
    fn strip_email_works() {
        const EXPECTED: &str = "Christopher R. Field";
        let re = Regex::new(r"<(.*?)>").unwrap();
        let actual = re.replace_all("Christopher R. Field <cfield2@gmail.com>", "");
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
        const EXPECTED: &str = "cfield2@gmail.com";
        let re = Regex::new(r"<(.*?)>").unwrap();
        let actual = re.replace_all("cfield2@gmail.com", "");
        assert_eq!(actual.trim(), EXPECTED);
    }
}

