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

#[macro_use] extern crate log;
extern crate mustache;
extern crate toml;
extern crate uuid;

use mustache::MapBuilder;
use std::default::Default;
use std::error::Error as StdError;
use std::fmt;
use std::fs::{self, File};
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use toml::Value;
use uuid::Uuid;

const CARGO_MANIFEST_FILE: &str = "Cargo.toml";
const CARGO: &str = "cargo";
const SIGNTOOL: &str = "signtool";
const WIX: &str = "wix";
const WIX_COMPILER: &str = "candle";
const WIX_LINKER: &str = "light";
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

#[derive(Debug)]
pub enum Error {
    /// A build operation for the release binary failed.
    Build(String),
    /// A compiler operation failed.
    Compile(String),
    /// A generic or custom error occurred. The message should contain the detailed information.
    Generic(String),
    /// An I/O operation failed.
    Io(io::Error),
    /// A linker operation failed.
    Link(String),
    /// A needed field within the `Cargo.toml` manifest could not be found.
    Manifest(String),
    /// An error occurred with rendering the template using the mustache renderer.
    Mustache(mustache::Error),
    /// A signing operation failed.
    Sign(String),
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
            Error::Build(..) => 1,
            Error::Compile(..) => 2,
            Error::Generic(..) => 3,
            Error::Io(..) => 4,
            Error::Link(..) => 5,
            Error::Manifest(..) => 6,
            Error::Mustache(..) => 7,
            Error::Sign(..) => 8,
            Error::Toml(..) => 9,
        }
    }
}

impl StdError for Error {
    fn description(&self) -> &str {
        match *self {
            Error::Build(..) => "Build",
            Error::Compile(..) => "Compile",
            Error::Generic(..) => "Generic",
            Error::Io(..) => "Io",
            Error::Link(..) => "Link",
            Error::Manifest(..) => "Manifest",
            Error::Mustache(..) => "Mustache",
            Error::Sign(..) => "Sign",
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
            Error::Build(ref msg) => write!(f, "{}", msg),
            Error::Compile(ref msg) => write!(f, "{}", msg),
            Error::Generic(ref msg) => write!(f, "{}", msg),
            Error::Io(ref err) => write!(f, "{}", err),
            Error::Link(ref msg) => write!(f, "{}", msg),
            Error::Manifest(ref var) => write!(f, "No '{}' field found in the package's manifest (Cargo.toml)", var),
            Error::Mustache(ref err) => write!(f, "{}", err),
            Error::Sign(ref msg) => write!(f, "{}", msg),
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

/// The builder for running the subcommand.
#[derive(Debug, Clone)]
pub struct Wix {
    capture_output: bool,
    input: Option<PathBuf>,
    manufacturer: Option<String>,
    sign: bool,
    timestamp: Option<String>,
}

impl Wix {
    /// Creates a new `Wix` instance.
    pub fn new() -> Self {
        Wix {
            capture_output: true,
            input: None,
            manufacturer: None,
            sign: false,
            timestamp: None,
        }
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

    /// Sets the path to a file to be used as the WiX Source (wxs) file instead of `wix\main.rs`.
    pub fn input(mut self, i: Option<&str>) -> Self {
        self.input = i.map(|i| PathBuf::from(i));
        self
    }

    /// Overrides the first author in the `authors` field of the package's manifest (Cargo.toml) as
    /// the manufacturer within the installer.
    pub fn manufacturer(mut self, m: Option<&str>) -> Self {
        self.manufacturer = m.map(|s| String::from(s));
        self
    }

    /// Enables or disables signing of the installer after creation with the `signtool`
    /// application.
    pub fn sign(mut self, s: bool) -> Self {
        self.sign = s;
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
        let pkg_name = cargo_values
            .get("package")
            .and_then(|p| p.as_table())
            .and_then(|t| t.get("name"))
            .and_then(|n| n.as_str())
            .ok_or(Error::Manifest(String::from("name")))?;
        debug!("pkg_name = {:?}", pkg_name);
        let pkg_description = cargo_values
            .get("package")
            .and_then(|p| p.as_table())
            .and_then(|t| t.get("description"))
            .and_then(|d| d.as_str())
            .ok_or(Error::Manifest(String::from("description")))?;
        let manufacturer = if let Some(m) = self.manufacturer {
            Ok(m)
        } else {
            cargo_values.get("package")
            .and_then(|p| p.as_table())
            .and_then(|t| t.get("authors"))
            .and_then(|a| a.as_array())
            .and_then(|a| a.get(0)) // For now, just use the first author
            .and_then(|f| f.as_str())
            .map(|m| String::from(m))
            .ok_or(Error::Manifest(String::from("authors")))
        }?;
        debug!("pkg_description = {:?}", pkg_description);
        let help_url = cargo_values
            .get("package")
            .and_then(|p| p.as_table())
            .and_then(|t| t.get("documentation").or(t.get("homepage")).or(t.get("repository")))
            .and_then(|h| h.as_str())
            .ok_or(Error::Manifest(String::from("documentation")))?;
        let bin_name = cargo_values
            .get("bin")
            .and_then(|b| b.as_table())
            .and_then(|t| t.get("name"))
            .and_then(|n| n.as_str())
            .unwrap_or(pkg_name);
        debug!("bin_name = {:?}", bin_name);
        let platform = if cfg!(target_arch = "x86_64") {
            Platform::X64
        } else {
            Platform::X86
        };
        debug!("platform = {:?}", platform);
        let main_wxs = if let Some(p) = self.input {
            if p.exists() {
                trace!("Using the '{}' WiX source file", p.display());
                Ok(p)
            } else {
                Err(Error::Generic(format!("The '{}' WiX source (wxs) file does not exist", p.display())))
            }
        } else {
            trace!("Using the default 'wix\\main.wxs' WiX source file");
            let mut main_wxs = PathBuf::from(WIX);
            main_wxs.push(WIX_SOURCE_FILE_NAME);
            main_wxs.set_extension(WIX_SOURCE_FILE_EXTENSION);
            Ok(main_wxs)
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
        main_msi.push(&format!("{}-{}-{}.msi", pkg_name, pkg_version, platform.arch()));
        debug!("main_msi = {:?}", main_msi);
        // Build the binary with the release profile. If a release binary has already been built, then
        // this will essentially do nothing.
        info!("Building release binary");
        if let Some(status) = {
            let mut builder = Command::new(CARGO);
            if self.capture_output {
                trace!("Capturing the '{}' output", CARGO);
                builder.stdout(Stdio::null());
                builder.stderr(Stdio::null());
            }
            builder.arg("build")
                .arg("--release")
                .status()
        }.ok() {
            if !status.success() {
                // TODO: Add better error message
                return Err(Error::Build(String::from("Failed to build the release executable")));
            }
        }
        // Compile the installer
        info!("Compiling installer");
        if let Some(status) = {
            let mut compiler = Command::new(WIX_COMPILER);
            if self.capture_output {
                trace!("Capturing the '{}' output", WIX_COMPILER);
                compiler.stdout(Stdio::null());
                compiler.stderr(Stdio::null());
            } 
            compiler.arg(format!("-dVersion={}", pkg_version))
                .arg(format!("-dPlatform={}", platform))
                .arg(format!("-dProductName={}", pkg_name))
                .arg(format!("-dBinaryName={}", bin_name))
                .arg(format!("-dDescription={}", pkg_description))
                .arg(format!("-dManufacturer={}", manufacturer))
                .arg(format!("-dHelp={}", help_url))
                .arg("-o")
                .arg(&main_wixobj)
                .arg(&main_wxs)
                .status()
        }.ok() {
            if !status.success() {
                // TODO: Add better error message
                return Err(Error::Compile(String::from("Failed to compile the installer")));
            }
        }
        // Link the installer
        info!("Linking the installer");
        if let Some(status) = {
            let mut linker = Command::new(WIX_LINKER);
            if self.capture_output {
                trace!("Capturing the '{}' output", WIX_LINKER);
                linker.stdout(Stdio::null());
                linker.stderr(Stdio::null());
            }
            linker.arg("-ext")
                .arg("WixUIExtension")
                .arg("-cultures:en-us")
                .arg(&main_wixobj)
                .arg("-out")
                .arg(&main_msi)
                .status()
        }.ok() {
            if !status.success() {
                // TODO: Add better error message
                return Err(Error::Link(String::from("Failed to link the installer")));
            }
        }
        // Sign the installer
        if self.sign {
            info!("Signing the installer");
            if let Some(status) = {
                let mut signer = Command::new(SIGNTOOL);
                if self.capture_output {
                    trace!("Capturing the {} output", SIGNTOOL);
                    signer.stdout(Stdio::null());
                    signer.stderr(Stdio::null());
                }
                signer.arg("sign").arg("/a");
                if let Some(t) = self.timestamp {
                    trace!("Using the '{}' timestamp server to sign the installer", t); 
                    signer.arg("/t");
                    signer.arg(t);
                }
                signer.arg(&main_msi).status()
            }.ok() {
                if !status.success() {
                    // TODO: Add better error message
                    return Err(Error::Sign(String::from("Failed to sign the installer")));
                }
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

