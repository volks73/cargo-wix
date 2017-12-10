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
use std::error::Error as StdError;
use std::fmt;
use std::fs::{self, File};
use std::io::{self, Write};
use std::path::PathBuf;
use std::str::FromStr;
use uuid::Uuid;

pub use self::wix::Wix;

mod wix;

use wix::{WIX, WIX_SOURCE_FILE_EXTENSION, WIX_SOURCE_FILE_NAME};

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

/// Removes the `target\wix` folder.
pub fn clean() -> Result<(), Error> {
    let mut target_wix = PathBuf::from("target");
    target_wix.push(WIX);
    if target_wix.exists() {
        trace!("The 'target\\wix' folder exists");
        info!("Removing the 'target\\wix' folder");
        fs::remove_dir_all(target_wix)?;
    } else {
        warn!("The 'target\\wix' folder does not exist");
    }
    Ok(())
}

/// Removes the `target\wix` folder and the `wix` folder.
///
/// __Use with caution!__ All contents of both folders are removed, including files that may be
/// located in the folders but not used or related to the creation of Windows installers via the
/// WiX Toolset.
pub fn purge() -> Result<(), Error> {
    clean()?;
    let wix = PathBuf::from(WIX);
    if wix.exists() {
        trace!("The 'wix' folder exists");
        info!("Removing the 'wix' folder");
        fs::remove_dir_all(wix)?;
    } else {
        warn!("The 'wix' folder does not exist");
    }
    Ok(())
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

