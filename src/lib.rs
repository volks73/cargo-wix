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
//! these two applications with the `std::process::Command` module to create an installer.
//! Generally, it is best to add the WiX Toolset `bin` folder to the PATH system environment
//! variable, but a WIX_PATH environment variable or the `-B,--bin-path` option can be used to
//! specify a path (relative or absolute) to the WiX Toolset `bin` folder. The order of precedence
//! (descending) is: `-B,--bin-path` option, WIX_PATH environment variable, and then the PATH
//! system environment variable.
//!
//! The Windows SDK provides a signer (`signtool`) application for signing installers. The
//! application is installed in the `bin` folder of the Windows SDK installation. The location of
//! the `bin` folder varies depending on the version. It is recommended to use the Developer Prompt
//! to ensure the `signtool` application is available; however, it is possible to specify the path
//! to the Windows SDK `bin` folder using the `-S,--sign-path` option at the command line. Since
//! signing is optional.
//!
//! The WiX Toolset requires a WiX Source (wxs) file, which is an XML file. A template is provided
//! with this subcommand that attempts to meet the majority of use cases for developers, so
//! extensive knowledge of the WiX Toolset and Windows installer technologies is not required (but
//! always recommended). Modification of the template is encouraged, but please consult the WiX
//! Toolset's extensive documentation and tutorials for information about writing, customizing, and
//! using wxs files. The documentation here is only for this subcommand.
//!
//! The [WXS template](https://github.com/volks73/cargo-wix/blob/master/src/main.wxs.mustache) is
//! embedded in the binary installation of the subcommand and it can be printed to stdout using the
//! `cargo wix --print-template wxs` command from the command prompt (cmd.exe). Note, each time the
//! `cargo wix --print-template wxs` command is invoked, new GUIDs are generated for fields that
//! require them.  Thus, a developer does not need to worry about generating GUIDs and can begin
//! using the template immediately with this subcommand or the WiX Toolset's `candle.exe` and
//! `light.exe` applications.
//!
//! In addition to the WXS template, there are several license templates which are used to generate
//! an End User License Agreement (EULA) during the `cargo wix --init` command. Depending on the
//! license ID(s) in the `license` field for a package's manifest (Cargo.toml), a license file
//! in the Rich Text Format (RTF) is generated from a template and placed in the `wix` folder. This
//! RTF file is then displayed in the license dialog of the installer. See the help information on
//! the `--print-template` option for information about supported licenses. If the `license` field
//! is not used or the license ID is not supported, then the EULA will not be automatically created
//! during initialization and it will have to be created manually with a text editor or other
//! authoring tool.
//!
//! During creation of the installer, the `cargo wix` subcommand will look for a LICENSE file in
//! the root of the project. If the file exists, then it is used as the source file for the
//! `License.txt` file that is placed alongside the `bin` folder during installation of the project
//! with the Windows installer (msi). If a LICENSE file does not exist, then the `-l,--license`
//! option should be used to specify a path to a file that can be used.
//!
//! If a custom license is used, then the `license-file` field should be used with the package's
//! manifest. The path specified for the `license-file` field is used to create a "sidecar" license
//! file that is installed alongside the `bin` folder during installation. Basically, if a custom
//! license is used, everything should be manually setup and modified.
//!
//! Generally, any value that is obtained from the package's manifest (Cargo.toml) can be
//! overridden at the command line with an appropriate option. For example, the manufacturer, which
//! is displayed as the "Publisher" in the Add/Remove Programs (ARP) control panel is obtained from
//! the first author listed in the `authors` field of a package's manifest, but it can be
//! overridden using the `-m,--manufacturer` option. The default in most cases is to use a value
//! from a field in the package's manifest.
//!
//! The `cargo wix` subcommand uses the package name for the product name. The default install
//! location is at `C:\Program Files\[Product Name]`, where `[Product Name]` is replaced with the
//! product name determined from the package name. This can be overridden with the
//! `-p,--product-name` option. The binary name, which is the `name` field for the `[[bin]]`
//! section, is used for the executable file name, i.e. "name.exe". This can also be overridden
//! using the `-b,--bin-name` option. The package description is used in multiple places for the
//! installer, including the text that appears in the blue UAC dialog when using a signed
//! installer. This can be overridden using the `-d,--description` option.
//!
//! An unmodified WXS file from the embedded template will create an installer that installs the
//! executable file in a `bin` folder within the installation directory selected by the end-user
//! during installation. It will add a `License.txt` file to the same folder as the `bin` folder
//! from a `LICENSE` file, and it will add the `bin` folder to the PATH system environment variable
//! so that the executable can be called from anywhere with a commmand prompt. Most of these
//! behaviors can be adjusted during the installation process of the Windows installer.

extern crate chrono;
#[macro_use] extern crate log;
extern crate mustache;
extern crate regex;
extern crate toml;
extern crate uuid;

use std::default::Default;
use std::error::Error as StdError;
use std::fmt;
use std::fs;
use std::io;
use std::path::PathBuf;
use std::str::FromStr;

pub use self::wix::Wix;

mod wix;

/// A specialized `Result` type for cargo-wix operations.
pub type Result<T> = std::result::Result<T, Error>;

use wix::WIX;

/// The WiX Source (wxs) template.
static WIX_SOURCE_TEMPLATE: &str = include_str!("main.wxs.mustache");

/// The Apache-2.0 Rich Text Format (RTF) license template.
static APACHE2_LICENSE_TEMPLATE: &str = include_str!("Apache-2.0.rtf.mustache");

/// The GPL-3.0 Rich Text Format (RTF) license template.
static GPL3_LICENSE_TEMPLATE: &str = include_str!("GPL-3.0.rtf.mustache");

/// The MIT Rich Text Format (RTF) license template.
static MIT_LICENSE_TEMPLATE: &str = include_str!("MIT.rtf.mustache");

/// Removes the `target\wix` folder.
pub fn clean() -> Result<()> {
    let mut target_wix = PathBuf::from("target");
    target_wix.push(WIX);
    if target_wix.exists() {
        trace!("The 'target\\wix' folder exists");
        warn!("Removing the 'target\\wix' folder");
        fs::remove_dir_all(target_wix)?;
    } else {
        trace!("The 'target\\wix' folder does not exist");
    }
    Ok(())
}

/// Removes the `target\wix` folder and the `wix` folder.
///
/// __Use with caution!__ All contents of both folders are removed, including files that may be
/// located in the folders but not used or related to the creation of Windows installers via the
/// WiX Toolset.
pub fn purge() -> Result<()> {
    clean()?;
    let wix = PathBuf::from(WIX);
    if wix.exists() {
        trace!("The 'wix' folder exists");
        warn!("Removing the 'wix' folder");
        fs::remove_dir_all(wix)?;
    } else {
        trace!("The 'wix' folder does not exist");
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
    Manifest(&'static str),
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

/// The different templates that can be printed using the `--print-template` option.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Template {
    Apache2,
    Gpl3,
    Mit,
    Wxs,
}

impl Template {
    /// Gets the ID for the template.
    ///
    /// In the case of a license template, the ID is the SPDX ID which is also used for the
    /// `license` field in the package's manifest (Cargo.toml). This is also the same value used
    /// with the `--print-template` option.
    pub fn id(&self) -> &str {
        match *self {
            Template::Apache2 => "Apache-2.0",
            Template::Gpl3 => "GPL-3.0",
            Template::Mit => "MIT",
            Template::Wxs => "WXS",
        }
    }

    /// Gets the possible string representations of each variant.
    pub fn possible_values() -> Vec<String> {
        vec![
            Template::Apache2.id().to_owned(), 
            Template::Apache2.id().to_lowercase(), 
            Template::Gpl3.id().to_owned(), 
            Template::Gpl3.id().to_lowercase(), 
            Template::Mit.id().to_owned(), 
            Template::Mit.id().to_lowercase(), 
            Template::Wxs.id().to_owned(),
            Template::Wxs.id().to_lowercase(),
        ]
    }

    /// Gets the embedded contents of the template as a string.
    pub fn to_str(&self) -> &str {
        match *self {
            Template::Apache2 => APACHE2_LICENSE_TEMPLATE,
            Template::Gpl3 => GPL3_LICENSE_TEMPLATE,
            Template::Mit => MIT_LICENSE_TEMPLATE,
            Template::Wxs => WIX_SOURCE_TEMPLATE,
        }
    }
}

impl fmt::Display for Template {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.id())
    }
}

impl FromStr for Template {
    type Err = Error;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        match s.to_lowercase().trim() {
            "apache-2.0" => Ok(Template::Apache2),
            "gpl-3.0" => Ok(Template::Gpl3),
            "mit" => Ok(Template::Mit),
            "wxs" => Ok(Template::Wxs),
            _ => Err(Error::Generic(format!("Cannot convert from '{}' to a Template variant", s))),
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

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        match s.to_lowercase().trim() {
            "comodo" => Ok(TimestampServer::Comodo),
            "verisign" => Ok(TimestampServer::Verisign),
            u @ _ => Ok(TimestampServer::Custom(String::from(u)))
        }
    }
}

