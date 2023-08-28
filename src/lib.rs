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

//! # `wix` Library
//!
//! The goal of the cargo-wix project and the `wix` library is to make it easy
//! to create a Windows installer (msi) for any Rust project. The cargo-wix
//! project is primarily implemented as a [cargo subcommand], but its core
//! functionality has been organized into a separate library. Documentation for
//! the binary and Command Line Interface (CLI) are provided in the module-level
//! documentation for the [binary] and the `cargo wix --help` command.
//!
//! ## Table of Contents
//!
//! - [Usage](#usage)
//! - [Organization](#organization)
//!
//! ## Usage
//!
//! First, add this to your `Cargo.toml`:
//!
//! ```toml
//! [dependencies]
//! wix = "0.1"
//! ```
//!
//! Next, if not using Rust 2018 edition, then add this to the `lib.rs` or
//! `main.rs` file for your project:
//!
//! ```ignore
//! extern crate wix;
//! ```
//!
//! ## Organization
//!
//! Each subcommand is organized into a separate module. So, there is a
//! `create`, `inititalize`, `print`, etc. module within the crate. Some of the
//! modules are in a single Rust source file, while others are organized into
//! sub-folders. Each module follows the [Builder] design pattern, so there is a
//! `Builder` and `Execution` struct for each module/subcommand. The `Builder`
//! struct is used to customize the execution of the subcommand and builds an
//! `Execution` struct. The `Execution` struct is responsible for actually
//! executing the subcommand, which generally involves executing a process with
//! the [`std::process::Command`] struct, but not always. Each method for the
//! `Builder` struct generally corresponds to a CLI option or argument found in
//! the [`cargo wix`] subcommand and binary.
//!
//! [binary]: ../cargo_wix/index.html
//! [Builder]: https://doc.rust-lang.org/1.0.0/style/ownership/builders.html
//! [cargo subcommand]: https://github.com/rust-lang/cargo/wiki/Third-party-cargo-subcommands
//! [`cargo wix`]: ../cargo_wix/index.html
//! [`std::process::Command`]: https://doc.rust-lang.org/std/process/struct.Command.html

pub use crate::templates::Template;

pub mod clean;
pub mod create;
mod eula;
pub mod initialize;
pub mod print;
pub mod purge;
pub mod sign;
mod templates;

use log::debug;

use std::convert::TryFrom;
use std::default::Default;
use std::env;
use std::error::Error as StdError;
use std::ffi::OsStr;
use std::fmt;
use std::fmt::Display;
use std::io::{self, ErrorKind};
use std::path::{Path, PathBuf};
use std::str::FromStr;

use cargo_metadata::{Metadata, MetadataCommand, Package};

use rustc_cfg::Cfg;

/// The name of the folder where binaries are typically stored.
pub const BINARY_FOLDER_NAME: &str = "bin";

/// The file name with extension for a package's manifest.
pub const CARGO_MANIFEST_FILE: &str = "Cargo.toml";

/// The name of the builder application for a Rust project.
pub const CARGO: &str = "cargo";

/// The file extension for an executable.
pub const EXE_FILE_EXTENSION: &str = "exe";

/// The file name without an extension when generating a license.
pub const LICENSE_FILE_NAME: &str = "License";

/// The file extension for a Windows installer.
pub const MSI_FILE_EXTENSION: &str = "msi";

/// The file extension for a Rich Text Format (RTF) file.
pub const RTF_FILE_EXTENSION: &str = "rtf";

/// The name of the signer application from the Windows SDK.
pub const SIGNTOOL: &str = "signtool";

/// The name of the environment variable to specify the path to the signer
/// application.
pub const SIGNTOOL_PATH_KEY: &str = "SIGNTOOL_PATH";

/// The default name of the folder for output from this subcommand.
pub const WIX: &str = "wix";

/// The application name without the file extension of the compiler for the
/// Windows installer.
pub const WIX_COMPILER: &str = "candle";

/// The application name without the file extension of the linker for the
/// Windows installer.
pub const WIX_LINKER: &str = "light";

/// The application name without the file extension of the `msiexec` utility.
pub const MSIEXEC: &str = "msiexec";

/// The file extension for a WiX Toolset object file, which is the output from
/// the WiX compiler.
pub const WIX_OBJECT_FILE_EXTENSION: &str = "wixobj";

/// The name of the environment variable created by the WiX Toolset installer
/// that points to the `bin` folder for the WiX Toolet's compiler (candle.exe)
/// and linker (light.exe).
pub const WIX_PATH_KEY: &str = "WIX";

/// The file extension of the WiX Source file, which is the input to the WiX
/// Toolset compiler.
pub const WIX_SOURCE_FILE_EXTENSION: &str = "wxs";

/// The default file name for the WiX Source file, which is the input to the WiX
/// Toolset compiler.
pub const WIX_SOURCE_FILE_NAME: &str = "main";

/// A specialized [`Result`] type for wix operations.
///
/// [`Result`]: https://doc.rust-lang.org/std/result/
pub type Result<T> = std::result::Result<T, Error>;

fn cargo_toml_file(input: Option<&PathBuf>) -> Result<PathBuf> {
    let i = match input {
        Some(i) => i.to_owned(),
        None => {
            let mut cwd = env::current_dir()?;
            cwd.push(CARGO_MANIFEST_FILE);
            cwd
        }
    };
    if i.exists() {
        if i.is_file() {
            if i.file_name() == Some(OsStr::new(CARGO_MANIFEST_FILE)) {
                Ok(i)
            } else {
                Err(Error::not_a_manifest(&i))
            }
        } else {
            Err(Error::not_a_file(&i))
        }
    } else {
        Err(Error::not_found(&i))
    }
}

fn description(description: Option<String>, manifest: &Package) -> Option<String> {
    description.or_else(|| manifest.description.clone())
}

fn manifest(input: Option<&PathBuf>) -> Result<Metadata> {
    let cargo_file_path = cargo_toml_file(input)?;
    debug!("cargo_file_path = {:?}", cargo_file_path);
    Ok(MetadataCommand::new()
        .no_deps()
        .manifest_path(cargo_file_path)
        .exec()?)
}

fn package(manifest: &Metadata, package: Option<&str>) -> Result<Package> {
    let package_id = if let Some(p) = package {
        manifest
            .workspace_members
            .iter()
            .find(|n| manifest[n].name == p)
            .ok_or_else(|| Error::Generic(format!("No `{p}` package found in the project")))?
    } else if manifest.workspace_members.len() == 1 {
        &manifest.workspace_members[0]
    } else {
        // TODO: Replace error with creating installers for all packages in a
        // workspace. I think this currently means that to create installers for
        // all packages in workspace, a `cargo wix --package <name>` must be
        // executed for each workspace member.
        return Err(Error::Generic(String::from(
            "Workspace detected. Please pass a package name.",
        )));
    };
    Ok(manifest[package_id].clone())
}

fn package_root(input: Option<&PathBuf>) -> Result<PathBuf> {
    cargo_toml_file(input).map(|p| {
        p.parent()
            .map(PathBuf::from)
            .expect("The Cargo.toml file to NOT be root.")
    })
}

fn product_name(product_name: Option<&String>, manifest: &Package) -> String {
    if let Some(p) = product_name {
        p.to_owned()
    } else {
        manifest.name.clone()
    }
}

/// The error type for wix-related operations and associated traits.
///
/// Errors mostly originate from the dependencies, but custom instances of `Error` can be created
/// with the `Generic` variant and a message.
#[derive(Debug)]
pub enum Error {
    /// Parsing of Cargo metadata failed.
    CargoMetadata(cargo_metadata::Error),
    /// A command operation failed.
    Command(&'static str, i32, bool),
    /// A generic or custom error occurred. The message should contain the detailed information.
    Generic(String),
    /// An I/O operation failed.
    Io(io::Error),
    /// A needed field within the `Cargo.toml` manifest could not be found.
    Manifest(&'static str),
    /// An error occurred with rendering the template using the mustache renderer.
    Mustache(mustache::Error),
    /// UUID generation or parsing failed.
    Uuid(uuid::Error),
    /// Parsing error for a version string or field.
    Version(semver::Error),
    /// Parsing the intermediate WiX Object (wixobj) file, which is XML, failed.
    Xml(sxd_document::parser::Error),
    /// Evaluation of an XPath expression failed.
    XPath(sxd_xpath::ExecutionError),
}

impl Error {
    /// Gets an error code related to the error.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use wix::Error;
    ///
    /// let err = Error::from("A generic error");
    /// assert_ne!(err.code(), 0)
    /// ```
    ///
    /// This is useful as a return, or exit, code for a command line application, where a non-zero
    /// integer indicates a failure in the application. it can also be used for quickly and easily
    /// testing equality between two errors.
    pub fn code(&self) -> i32 {
        match *self {
            Error::Command(..) => 1,
            Error::Generic(..) => 2,
            Error::Io(..) => 3,
            Error::Manifest(..) => 4,
            Error::Mustache(..) => 5,
            Error::Uuid(..) => 6,
            Error::Version(..) => 7,
            Error::Xml(..) => 8,
            Error::XPath(..) => 9,
            Error::CargoMetadata(..) => 10,
        }
    }

    /// Creates a new `Error` from a [std::io::Error] with the
    /// [std::io::ErrorKind::AlreadyExists] variant.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use std::io;
    /// use std::path::Path;
    /// use wix::Error;
    ///
    /// let path = Path::new("C:\\");
    /// let expected = Error::Io(io::Error::new(
    ///     io::ErrorKind::AlreadyExists,
    ///     path.display().to_string()
    /// ));
    /// assert_eq!(expected, Error::already_exists(path));
    /// ```
    ///
    /// [std::io::Error]: https://doc.rust-lang.org/std/io/struct.Error.html
    /// [std::io::ErrorKind::AlreadyExists]: https://doc.rust-lang.org/std/io/enum.ErrorKind.html
    pub fn already_exists(p: &Path) -> Self {
        io::Error::new(ErrorKind::AlreadyExists, p.display().to_string()).into()
    }

    /// Creates a new `Error` from a [std::io::Error] with the
    /// [std::io::ErrorKind::NotFound] variant.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use std::io;
    /// use std::path::Path;
    /// use wix::Error;
    ///
    /// let path = Path::new("C:\\Cargo\\Wix\\file.txt");
    /// let expected = Error::Io(io::Error::new(
    ///     io::ErrorKind::NotFound,
    ///     path.display().to_string()
    /// ));
    /// assert_eq!(expected, Error::not_found(path));
    /// ```
    ///
    /// [std::io::Error]: https://doc.rust-lang.org/std/io/struct.Error.html
    /// [std::io::ErrorKind::NotFound]: https://doc.rust-lang.org/std/io/enum.ErrorKind.html
    pub fn not_found(p: &Path) -> Self {
        io::Error::new(ErrorKind::NotFound, p.display().to_string()).into()
    }

    /// Creates a new `Error` from a [std::io::Error] with the
    /// [std::io::ErrorKind::InvalidInput] variant if a path is not a file.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use std::io;
    /// use std::path::Path;
    /// use wix::Error;
    ///
    /// let path = Path::new("C:\\Cargo\\Wix\\file.txt");
    /// let expected = Error::Io(io::Error::new(
    ///     io::ErrorKind::InvalidInput,
    ///     format!("The '{}' path is not a file.", path.display())
    /// ));
    /// assert_eq!(expected, Error::not_a_file(path));
    /// ```
    ///
    /// [std::io::Error]: https://doc.rust-lang.org/std/io/struct.Error.html
    /// [std::io::ErrorKind::InvalidInput]: https://doc.rust-lang.org/std/io/enum.ErrorKind.html
    pub fn not_a_file(p: &Path) -> Self {
        io::Error::new(
            ErrorKind::InvalidInput,
            format!("The '{}' path is not a file.", p.display()),
        )
        .into()
    }

    /// Creates a new `Error` from a [std::io::Error] with the
    /// [std::io::ErrorKind::InvalidInput] variant if a path is not to a
    /// `Cargo.toml` file.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use std::io;
    /// use std::path::Path;
    /// use wix::Error;
    ///
    /// let path = Path::new("C:\\Cargo\\Wix\\file.txt");
    /// let expected = Error::Io(io::Error::new(
    ///     io::ErrorKind::InvalidInput,
    ///     format!(
    ///         "The '{}' path does not appear to be to a 'Cargo.toml' file.",
    ///         path.display(),
    ///     ),
    /// ));
    /// assert_eq!(expected, Error::not_a_manifest(path));
    /// ```
    ///
    /// [std::io::Error]: https://doc.rust-lang.org/std/io/struct.Error.html
    /// [std::io::ErrorKind::InvalidInput]: https://doc.rust-lang.org/std/io/enum.ErrorKind.html
    pub fn not_a_manifest(p: &Path) -> Self {
        io::Error::new(
            ErrorKind::InvalidInput,
            format!(
                "The '{}' path does not appear to be to a '{}' file.",
                p.display(),
                CARGO_MANIFEST_FILE
            ),
        )
        .into()
    }

    /// Extracts a short, single word representation of the error.
    ///
    /// The `std::error::Error::description` method is "soft-deprecated"
    /// according to the Rust stdlib documentation. It is recommended to use the
    /// `std::fmt::Display` implementation for a "description" string. However,
    /// there is already a `std::fmt::Display` implemenation for this error
    /// type, and it is nice to have a short, single word representation for
    /// nicely formatting errors to humans. This method maintains the error
    /// message formatting.
    pub fn as_str(&self) -> &str {
        match *self {
            Error::CargoMetadata(..) => "CargoMetadata",
            Error::Command(..) => "Command",
            Error::Generic(..) => "Generic",
            Error::Io(..) => "Io",
            Error::Manifest(..) => "Manifest",
            Error::Mustache(..) => "Mustache",
            Error::Uuid(..) => "UUID",
            Error::Version(..) => "Version",
            Error::Xml(..) => "XML",
            Error::XPath(..) => "XPath",
        }
    }
}

impl StdError for Error {
    fn description(&self) -> &str {
        self.as_str()
    }

    fn source(&self) -> Option<&(dyn StdError + 'static)> {
        match *self {
            Error::CargoMetadata(ref err) => Some(err),
            Error::Io(ref err) => Some(err),
            Error::Mustache(ref err) => Some(err),
            Error::Uuid(ref err) => Some(err),
            Error::Version(ref err) => Some(err),
            Error::Xml(ref err) => Some(err),
            Error::XPath(ref err) => Some(err),
            _ => None,
        }
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            Error::CargoMetadata(ref err) => err.fmt(f),
            Error::Command(ref command, ref code, captured_output) => {
                if captured_output {
                    write!(
                        f,
                        "The '{command}' application failed with exit code = {code}. Consider using the \
                         '--nocapture' flag to obtain more information."
                    )
                } else {
                    write!(
                        f,
                        "The '{command}' application failed with exit code = {code}"
                    )
                }
            }
            Error::Generic(ref msg) => msg.fmt(f),
            Error::Io(ref err) => match err.kind() {
                ErrorKind::AlreadyExists => {
                    if let Some(path) = err.get_ref() {
                        write!(f, "The '{path}' file already exists. Use the '--force' flag to overwrite the contents.")
                    } else {
                        err.fmt(f)
                    }
                }
                ErrorKind::NotFound => {
                    if let Some(path) = err.get_ref() {
                        write!(f, "The '{path}' path does not exist")
                    } else {
                        err.fmt(f)
                    }
                }
                _ => err.fmt(f),
            },
            Error::Manifest(ref var) => write!(
                f,
                "No '{var}' field found in the package's manifest (Cargo.toml)"
            ),
            Error::Mustache(ref err) => err.fmt(f),
            Error::Uuid(ref err) => err.fmt(f),
            Error::Version(ref err) => err.fmt(f),
            Error::Xml(ref err) => err.fmt(f),
            Error::XPath(ref err) => err.fmt(f),
        }
    }
}

impl PartialEq for Error {
    fn eq(&self, other: &Error) -> bool {
        self.code() == other.code()
    }
}

impl<'a> From<&'a str> for Error {
    fn from(s: &str) -> Self {
        Error::Generic(s.to_string())
    }
}

impl From<cargo_metadata::Error> for Error {
    fn from(err: cargo_metadata::Error) -> Self {
        Error::CargoMetadata(err)
    }
}

impl From<io::Error> for Error {
    fn from(err: io::Error) -> Self {
        Error::Io(err)
    }
}

impl From<mustache::Error> for Error {
    fn from(err: mustache::Error) -> Self {
        Error::Mustache(err)
    }
}

impl From<semver::Error> for Error {
    fn from(err: semver::Error) -> Self {
        Error::Version(err)
    }
}

impl From<std::path::StripPrefixError> for Error {
    fn from(err: std::path::StripPrefixError) -> Self {
        Error::Generic(err.to_string())
    }
}

impl From<sxd_document::parser::Error> for Error {
    fn from(err: sxd_document::parser::Error) -> Self {
        Error::Xml(err)
    }
}

impl From<sxd_xpath::ExecutionError> for Error {
    fn from(err: sxd_xpath::ExecutionError) -> Self {
        Error::XPath(err)
    }
}

impl From<uuid::Error> for Error {
    fn from(err: uuid::Error) -> Self {
        Error::Uuid(err)
    }
}

/// The different architectures supported by the WiX Toolset.
///
/// These are also the valid values for the `-arch` option to the WiX compiler
/// (candle.exe).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WixArch {
    /// The x86 32-bit architecture.
    X86,
    /// The x86_64 or AMD64 64-bit architecture.
    X64,
    /// The ARM 32-bit architecture.
    Arm,
    /// The ARM 64-bit architecture.
    Arm64,
}

impl Display for WixArch {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self {
            Self::X86 => write!(f, "x86"),
            Self::X64 => write!(f, "x64"),
            Self::Arm => write!(f, "arm"),
            Self::Arm64 => write!(f, "arm64"),
        }
    }
}

impl TryFrom<&Cfg> for WixArch {
    type Error = crate::Error;

    fn try_from(c: &Cfg) -> std::result::Result<Self, Self::Error> {
        match &*c.target_arch {
            "x86" => Ok(Self::X86),
            "x86_64" => Ok(Self::X64),
            "aarch64" => Ok(Self::Arm64),
            "thumbv7a" => Ok(Self::Arm),
            a => {
                if a.starts_with("arm") {
                    Ok(Self::Arm)
                } else {
                    Err(Error::Generic(format!(
                        "Unsupported target architecture: {a}"
                    )))
                }
            }
        }
    }
}

impl FromStr for WixArch {
    type Err = crate::Error;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        Self::try_from(&Cfg::of(s).map_err(|e| Error::Generic(e.to_string()))?)
    }
}

/// The aliases for the URLs to different Microsoft Authenticode timestamp servers.
#[derive(Debug, Clone, PartialEq, Eq)]
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
    ///
    /// # Examples
    ///
    /// ```rust
    /// use std::str::FromStr;
    /// use wix::TimestampServer;
    ///
    /// assert_eq!(
    ///     TimestampServer::from_str("http://www.example.com").unwrap().url(),
    ///     "http://www.example.com"
    /// );
    /// assert_eq!(
    ///     TimestampServer::Comodo.url(),
    ///     "http://timestamp.comodoca.com/"
    /// );
    /// assert_eq!(
    ///     TimestampServer::Verisign.url(),
    ///     "http://timestamp.verisign.com/scripts/timstamp.dll"
    /// );
    /// ```
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
            u => Ok(TimestampServer::Custom(String::from(u))),
        }
    }
}

/// The various culture codes for localization.
///
/// These are taken from the table in the [WixUI localization] documentation.
///
/// [WixUI localization]: http://wixtoolset.org/documentation/manual/v3/wixui/wixui_localization.html
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub enum Cultures {
    /// Arabic, Saudi Arabia
    ArSa,
    /// Bulgarian, Bulgaria
    BgBg,
    /// Catalan, Spain
    CaEs,
    /// Croatian, Croatia
    HrHr,
    /// Czech, Czech Republic
    CsCz,
    /// Danish, Denmark
    DaDk,
    /// Dutch, Netherlands
    NlNl,
    /// English, United States
    #[default]
    EnUs,
    /// Estonian, Estonia
    EtEe,
    /// Finnish, Finland
    FiFi,
    /// French, France
    FrFr,
    /// German, Germany
    DeDe,
    /// Greek, Greece
    ElGr,
    /// Hebrew, Israel
    HeIl,
    /// Hindi, India
    HiIn,
    /// Hungarian, Hungary
    HuHu,
    /// Italian, Italy
    ItIt,
    /// Japanese, Japan
    JaJp,
    /// Kazakh, Kazakhstan
    KkKz,
    /// Korean, Korea
    KoKr,
    /// Latvian, Latvia
    LvLv,
    /// Lithuanian, Lithuania
    LtLt,
    /// Norwegian, Norway
    NbNo,
    /// Polish, Poland
    PlPl,
    /// Portuguese, Brazil
    PtBr,
    /// Portuguese, Portugal
    PtPt,
    /// Romanian, Romania
    RoRo,
    /// Russian, Russian
    RuRu,
    /// Serbian, Serbia and Montenegro
    SrLatnCs,
    /// Simplified Chinese, China
    ZhCn,
    /// Slovak, Slovak Republic
    SkSk,
    /// Solvenian, Solvenia
    SlSi,
    /// Spanish, Spain
    EsEs,
    /// Swedish, Sweden
    SvSe,
    /// Thai, Thailand
    ThTh,
    /// Traditional Chinese, Hong Kong SAR
    ZhHk,
    /// Traditional Chinese, Taiwan
    ZhTw,
    /// Turkish, Turkey
    TrTr,
    /// Ukranian, Ukraine
    UkUa,
}

impl Cultures {
    /// The language of the culture code.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use wix::Cultures;
    ///
    /// assert_eq!(Cultures::ArSa.language(), "Arabic");
    /// assert_eq!(Cultures::BgBg.language(), "Bulgarian");
    /// assert_eq!(Cultures::CaEs.language(), "Catalan");
    /// assert_eq!(Cultures::HrHr.language(), "Croatian");
    /// assert_eq!(Cultures::CsCz.language(), "Czech");
    /// assert_eq!(Cultures::DaDk.language(), "Danish");
    /// assert_eq!(Cultures::NlNl.language(), "Dutch");
    /// assert_eq!(Cultures::EnUs.language(), "English");
    /// assert_eq!(Cultures::EtEe.language(), "Estonian");
    /// assert_eq!(Cultures::FiFi.language(), "Finnish");
    /// assert_eq!(Cultures::FrFr.language(), "French");
    /// assert_eq!(Cultures::DeDe.language(), "German");
    /// assert_eq!(Cultures::ElGr.language(), "Greek");
    /// assert_eq!(Cultures::HeIl.language(), "Hebrew");
    /// assert_eq!(Cultures::HiIn.language(), "Hindi");
    /// assert_eq!(Cultures::HuHu.language(), "Hungarian");
    /// assert_eq!(Cultures::ItIt.language(), "Italian");
    /// assert_eq!(Cultures::JaJp.language(), "Japanese");
    /// assert_eq!(Cultures::KkKz.language(), "Kazakh");
    /// assert_eq!(Cultures::KoKr.language(), "Korean");
    /// assert_eq!(Cultures::LvLv.language(), "Latvian");
    /// assert_eq!(Cultures::LtLt.language(), "Lithuanian");
    /// assert_eq!(Cultures::NbNo.language(), "Norwegian");
    /// assert_eq!(Cultures::PlPl.language(), "Polish");
    /// assert_eq!(Cultures::PtBr.language(), "Portuguese");
    /// assert_eq!(Cultures::PtPt.language(), "Portuguese");
    /// assert_eq!(Cultures::RoRo.language(), "Romanian");
    /// assert_eq!(Cultures::RuRu.language(), "Russian");
    /// assert_eq!(Cultures::SrLatnCs.language(), "Serbian (Latin)");
    /// assert_eq!(Cultures::ZhCn.language(), "Simplified Chinese");
    /// assert_eq!(Cultures::SkSk.language(), "Slovak");
    /// assert_eq!(Cultures::SlSi.language(), "Slovenian");
    /// assert_eq!(Cultures::EsEs.language(), "Spanish");
    /// assert_eq!(Cultures::SvSe.language(), "Swedish");
    /// assert_eq!(Cultures::ThTh.language(), "Thai");
    /// assert_eq!(Cultures::ZhHk.language(), "Traditional Chinese");
    /// assert_eq!(Cultures::ZhTw.language(), "Traditional Chinese");
    /// assert_eq!(Cultures::TrTr.language(), "Turkish");
    /// assert_eq!(Cultures::UkUa.language(), "Ukrainian");
    /// ```
    pub fn language(&self) -> &'static str {
        match *self {
            Cultures::ArSa => "Arabic",
            Cultures::BgBg => "Bulgarian",
            Cultures::CaEs => "Catalan",
            Cultures::HrHr => "Croatian",
            Cultures::CsCz => "Czech",
            Cultures::DaDk => "Danish",
            Cultures::NlNl => "Dutch",
            Cultures::EnUs => "English",
            Cultures::EtEe => "Estonian",
            Cultures::FiFi => "Finnish",
            Cultures::FrFr => "French",
            Cultures::DeDe => "German",
            Cultures::ElGr => "Greek",
            Cultures::HeIl => "Hebrew",
            Cultures::HiIn => "Hindi",
            Cultures::HuHu => "Hungarian",
            Cultures::ItIt => "Italian",
            Cultures::JaJp => "Japanese",
            Cultures::KkKz => "Kazakh",
            Cultures::KoKr => "Korean",
            Cultures::LvLv => "Latvian",
            Cultures::LtLt => "Lithuanian",
            Cultures::NbNo => "Norwegian",
            Cultures::PlPl => "Polish",
            Cultures::PtBr => "Portuguese",
            Cultures::PtPt => "Portuguese",
            Cultures::RoRo => "Romanian",
            Cultures::RuRu => "Russian",
            Cultures::SrLatnCs => "Serbian (Latin)",
            Cultures::ZhCn => "Simplified Chinese",
            Cultures::SkSk => "Slovak",
            Cultures::SlSi => "Slovenian",
            Cultures::EsEs => "Spanish",
            Cultures::SvSe => "Swedish",
            Cultures::ThTh => "Thai",
            Cultures::ZhHk => "Traditional Chinese",
            Cultures::ZhTw => "Traditional Chinese",
            Cultures::TrTr => "Turkish",
            Cultures::UkUa => "Ukrainian",
        }
    }

    /// The location of the culture component, typically the country that speaks the language.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use wix::Cultures;
    ///
    /// assert_eq!(Cultures::ArSa.location(), "Saudi Arabia");
    /// assert_eq!(Cultures::BgBg.location(), "Bulgaria");
    /// assert_eq!(Cultures::CaEs.location(), "Spain");
    /// assert_eq!(Cultures::HrHr.location(), "Croatia");
    /// assert_eq!(Cultures::CsCz.location(), "Czech Republic");
    /// assert_eq!(Cultures::DaDk.location(), "Denmark");
    /// assert_eq!(Cultures::NlNl.location(), "Netherlands");
    /// assert_eq!(Cultures::EnUs.location(), "United States");
    /// assert_eq!(Cultures::EtEe.location(), "Estonia");
    /// assert_eq!(Cultures::FiFi.location(), "Finland");
    /// assert_eq!(Cultures::FrFr.location(), "France");
    /// assert_eq!(Cultures::DeDe.location(), "Germany");
    /// assert_eq!(Cultures::ElGr.location(), "Greece");
    /// assert_eq!(Cultures::HeIl.location(), "Israel");
    /// assert_eq!(Cultures::HiIn.location(), "India");
    /// assert_eq!(Cultures::HuHu.location(), "Hungary");
    /// assert_eq!(Cultures::ItIt.location(), "Italy");
    /// assert_eq!(Cultures::JaJp.location(), "Japan");
    /// assert_eq!(Cultures::KkKz.location(), "Kazakhstan");
    /// assert_eq!(Cultures::KoKr.location(), "Korea");
    /// assert_eq!(Cultures::LvLv.location(), "Latvia");
    /// assert_eq!(Cultures::LtLt.location(), "Lithuania");
    /// assert_eq!(Cultures::NbNo.location(), "Norway");
    /// assert_eq!(Cultures::PlPl.location(), "Poland");
    /// assert_eq!(Cultures::PtBr.location(), "Brazil");
    /// assert_eq!(Cultures::PtPt.location(), "Portugal");
    /// assert_eq!(Cultures::RoRo.location(), "Romania");
    /// assert_eq!(Cultures::RuRu.location(), "Russia");
    /// assert_eq!(Cultures::SrLatnCs.location(), "Serbia and Montenegro");
    /// assert_eq!(Cultures::ZhCn.location(), "China");
    /// assert_eq!(Cultures::SkSk.location(), "Slovak Republic");
    /// assert_eq!(Cultures::SlSi.location(), "Solvenia");
    /// assert_eq!(Cultures::EsEs.location(), "Spain");
    /// assert_eq!(Cultures::SvSe.location(), "Sweden");
    /// assert_eq!(Cultures::ThTh.location(), "Thailand");
    /// assert_eq!(Cultures::ZhHk.location(), "Hong Kong SAR");
    /// assert_eq!(Cultures::ZhTw.location(), "Taiwan");
    /// assert_eq!(Cultures::TrTr.location(), "Turkey");
    /// assert_eq!(Cultures::UkUa.location(), "Ukraine");
    /// ```
    pub fn location(&self) -> &'static str {
        match *self {
            Cultures::ArSa => "Saudi Arabia",
            Cultures::BgBg => "Bulgaria",
            Cultures::CaEs => "Spain",
            Cultures::HrHr => "Croatia",
            Cultures::CsCz => "Czech Republic",
            Cultures::DaDk => "Denmark",
            Cultures::NlNl => "Netherlands",
            Cultures::EnUs => "United States",
            Cultures::EtEe => "Estonia",
            Cultures::FiFi => "Finland",
            Cultures::FrFr => "France",
            Cultures::DeDe => "Germany",
            Cultures::ElGr => "Greece",
            Cultures::HeIl => "Israel",
            Cultures::HiIn => "India",
            Cultures::HuHu => "Hungary",
            Cultures::ItIt => "Italy",
            Cultures::JaJp => "Japan",
            Cultures::KkKz => "Kazakhstan",
            Cultures::KoKr => "Korea",
            Cultures::LvLv => "Latvia",
            Cultures::LtLt => "Lithuania",
            Cultures::NbNo => "Norway",
            Cultures::PlPl => "Poland",
            Cultures::PtBr => "Brazil",
            Cultures::PtPt => "Portugal",
            Cultures::RoRo => "Romania",
            Cultures::RuRu => "Russia",
            Cultures::SrLatnCs => "Serbia and Montenegro",
            Cultures::ZhCn => "China",
            Cultures::SkSk => "Slovak Republic",
            Cultures::SlSi => "Solvenia",
            Cultures::EsEs => "Spain",
            Cultures::SvSe => "Sweden",
            Cultures::ThTh => "Thailand",
            Cultures::ZhHk => "Hong Kong SAR",
            Cultures::ZhTw => "Taiwan",
            Cultures::TrTr => "Turkey",
            Cultures::UkUa => "Ukraine",
        }
    }
}

impl fmt::Display for Cultures {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            Cultures::ArSa => write!(f, "ar-SA"),
            Cultures::BgBg => write!(f, "bg-BG"),
            Cultures::CaEs => write!(f, "ca-ES"),
            Cultures::HrHr => write!(f, "hr-HR"),
            Cultures::CsCz => write!(f, "cs-CZ"),
            Cultures::DaDk => write!(f, "da-DK"),
            Cultures::NlNl => write!(f, "nl-NL"),
            Cultures::EnUs => write!(f, "en-US"),
            Cultures::EtEe => write!(f, "et-EE"),
            Cultures::FiFi => write!(f, "fi-FI"),
            Cultures::FrFr => write!(f, "fr-FR"),
            Cultures::DeDe => write!(f, "de-DE"),
            Cultures::ElGr => write!(f, "el-GR"),
            Cultures::HeIl => write!(f, "he-IL"),
            Cultures::HiIn => write!(f, "hi-IN"),
            Cultures::HuHu => write!(f, "hu-HU"),
            Cultures::ItIt => write!(f, "it-IT"),
            Cultures::JaJp => write!(f, "ja-JP"),
            Cultures::KkKz => write!(f, "kk-KZ"),
            Cultures::KoKr => write!(f, "ko-KR"),
            Cultures::LvLv => write!(f, "lv-LV"),
            Cultures::LtLt => write!(f, "lt-LT"),
            Cultures::NbNo => write!(f, "nb-NO"),
            Cultures::PlPl => write!(f, "pl-PL"),
            Cultures::PtBr => write!(f, "pt-BR"),
            Cultures::PtPt => write!(f, "pt-PT"),
            Cultures::RoRo => write!(f, "ro-RO"),
            Cultures::RuRu => write!(f, "ru-RU"),
            Cultures::SrLatnCs => write!(f, "sr-Latn-CS"),
            Cultures::ZhCn => write!(f, "zh-CN"),
            Cultures::SkSk => write!(f, "sk-SK"),
            Cultures::SlSi => write!(f, "sl-SI"),
            Cultures::EsEs => write!(f, "es-ES"),
            Cultures::SvSe => write!(f, "sv-SE"),
            Cultures::ThTh => write!(f, "th-TH"),
            Cultures::ZhHk => write!(f, "zh-HK"),
            Cultures::ZhTw => write!(f, "zh-TW"),
            Cultures::TrTr => write!(f, "tr-TR"),
            Cultures::UkUa => write!(f, "uk-UA"),
        }
    }
}

impl FromStr for Cultures {
    type Err = Error;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        match s.to_lowercase().trim() {
            "ar-sa" => Ok(Cultures::ArSa),
            "bg-bg" => Ok(Cultures::BgBg),
            "ca-es" => Ok(Cultures::CaEs),
            "hr-hr" => Ok(Cultures::HrHr),
            "cs-cz" => Ok(Cultures::CsCz),
            "da-dk" => Ok(Cultures::DaDk),
            "nl-nl" => Ok(Cultures::NlNl),
            "en-us" => Ok(Cultures::EnUs),
            "et-ee" => Ok(Cultures::EtEe),
            "fi-fi" => Ok(Cultures::FiFi),
            "fr-fr" => Ok(Cultures::FrFr),
            "de-de" => Ok(Cultures::DeDe),
            "el-gr" => Ok(Cultures::ElGr),
            "he-il" => Ok(Cultures::HeIl),
            "hi-in" => Ok(Cultures::HiIn),
            "hu-hu" => Ok(Cultures::HuHu),
            "it-it" => Ok(Cultures::ItIt),
            "ja-jp" => Ok(Cultures::JaJp),
            "kk-kz" => Ok(Cultures::KkKz),
            "ko-kr" => Ok(Cultures::KoKr),
            "lv-lv" => Ok(Cultures::LvLv),
            "lt-lt" => Ok(Cultures::LtLt),
            "nb-no" => Ok(Cultures::NbNo),
            "pl-pl" => Ok(Cultures::PlPl),
            "pt-br" => Ok(Cultures::PtBr),
            "pt-pt" => Ok(Cultures::PtPt),
            "ro-ro" => Ok(Cultures::RoRo),
            "ru-ru" => Ok(Cultures::RuRu),
            "sr-latn-cs" => Ok(Cultures::SrLatnCs),
            "zh-cn" => Ok(Cultures::ZhCn),
            "sk-sk" => Ok(Cultures::SkSk),
            "sl-si" => Ok(Cultures::SlSi),
            "es-es" => Ok(Cultures::EsEs),
            "sv-se" => Ok(Cultures::SvSe),
            "th-th" => Ok(Cultures::ThTh),
            "zh-hk" => Ok(Cultures::ZhHk),
            "zh-tw" => Ok(Cultures::ZhTw),
            "tr-tr" => Ok(Cultures::TrTr),
            "uk-ua" => Ok(Cultures::UkUa),
            e => Err(Error::Generic(format!("Unknown '{e}' culture"))),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use assert_fs::TempDir;
    use std::env;
    use std::fs;

    /// Create a simple project with the provided TOML.
    pub fn setup_project(toml: &str) -> TempDir {
        pub const PERSIST_VAR_NAME: &str = "CARGO_WIX_TEST_PERSIST";

        let temp_dir = TempDir::new().unwrap();
        fs::write(temp_dir.path().join("Cargo.toml"), toml).unwrap();
        fs::create_dir(temp_dir.path().join("src")).unwrap();
        fs::write(temp_dir.path().join("src").join("main.rs"), "fn main() {}").unwrap();

        temp_dir.into_persistent_if(env::var(PERSIST_VAR_NAME).is_ok())
    }

    mod culture {
        use super::*;

        #[test]
        fn from_str_is_correct_for_dash_russian() {
            assert_eq!(Cultures::from_str("ru-ru"), Ok(Cultures::RuRu));
        }

        #[test]
        #[should_panic]
        fn from_str_fails_for_underscore_russian() {
            Cultures::from_str("ru_ru").unwrap();
        }

        #[test]
        fn display_is_correct_for_russian() {
            assert_eq!(format!("{}", Cultures::RuRu), String::from("ru-RU"));
        }

        #[test]
        fn from_str_is_correct_for_lowercase_slovak() {
            assert_eq!(Cultures::from_str("sk-sk"), Ok(Cultures::SkSk));
        }

        #[test]
        fn from_str_is_correct_for_uppercase_slovak() {
            assert_eq!(Cultures::from_str("sk-SK"), Ok(Cultures::SkSk));
        }

        #[test]
        #[should_panic]
        fn from_str_fails_for_underscore_slovak() {
            Cultures::from_str("sk_sk").unwrap();
        }

        #[test]
        fn display_is_correct_for_slovak() {
            assert_eq!(format!("{}", Cultures::SkSk), String::from("sk-SK"));
        }
    }

    mod wix_arch {
        use super::*;

        #[test]
        fn try_from_x86_64_pc_windows_msvc_is_correct() {
            let arch = WixArch::try_from(&Cfg::of("x86_64-pc-windows-msvc").expect("Cfg parsing"))
                .unwrap();
            assert_eq!(arch, WixArch::X64);
        }

        #[test]
        fn try_from_x86_64_pc_windows_gnu_is_correct() {
            let arch =
                WixArch::try_from(&Cfg::of("x86_64-pc-windows-gnu").expect("Cfg parsing")).unwrap();
            assert_eq!(arch, WixArch::X64);
        }

        #[test]
        fn try_from_x86_64_uwp_windows_msvc_is_correct() {
            let arch = WixArch::try_from(&Cfg::of("x86_64-uwp-windows-msvc").expect("Cfg parsing"))
                .unwrap();
            assert_eq!(arch, WixArch::X64);
        }

        #[test]
        fn try_from_x86_64_uwp_windows_gnu_is_correct() {
            let arch = WixArch::try_from(&Cfg::of("x86_64-uwp-windows-gnu").expect("Cfg parsing"))
                .unwrap();
            assert_eq!(arch, WixArch::X64);
        }

        #[test]
        fn try_from_i686_pc_windows_msvc_is_correct() {
            let arch =
                WixArch::try_from(&Cfg::of("i686-pc-windows-msvc").expect("Cfg parsing")).unwrap();
            assert_eq!(arch, WixArch::X86);
        }

        #[test]
        fn try_from_i686_pc_windows_gnu_is_correct() {
            let arch =
                WixArch::try_from(&Cfg::of("i686-pc-windows-gnu").expect("Cfg parsing")).unwrap();
            assert_eq!(arch, WixArch::X86);
        }

        #[test]
        fn try_from_i686_uwp_windows_msvc_is_correct() {
            let arch =
                WixArch::try_from(&Cfg::of("i686-uwp-windows-msvc").expect("Cfg parsing")).unwrap();
            assert_eq!(arch, WixArch::X86);
        }

        #[test]
        fn try_from_i686_uwp_windows_gnu_is_correct() {
            let arch =
                WixArch::try_from(&Cfg::of("i686-uwp-windows-gnu").expect("Cfg parsing")).unwrap();
            assert_eq!(arch, WixArch::X86);
        }

        #[test]
        fn try_from_i586_pc_windows_msvc_is_correct() {
            let arch =
                WixArch::try_from(&Cfg::of("i586-pc-windows-msvc").expect("Cfg parsing")).unwrap();
            assert_eq!(arch, WixArch::X86);
        }

        #[test]
        fn try_from_aarch64_pc_windows_msvc_is_correct() {
            let arch = WixArch::try_from(&Cfg::of("aarch64-pc-windows-msvc").expect("Cfg parsing"))
                .unwrap();
            assert_eq!(arch, WixArch::Arm64);
        }

        #[test]
        fn try_from_aarch64_uwp_windows_msvc_is_correct() {
            let arch =
                WixArch::try_from(&Cfg::of("aarch64-uwp-windows-msvc").expect("Cfg parsing"))
                    .unwrap();
            assert_eq!(arch, WixArch::Arm64);
        }

        #[test]
        fn try_from_thumbv7a_pc_windows_msvc_is_correct() {
            let arch =
                WixArch::try_from(&Cfg::of("thumbv7a-pc-windows-msvc").expect("Cfg parsing"))
                    .unwrap();
            assert_eq!(arch, WixArch::Arm);
        }

        #[test]
        fn try_from_thumbv7a_uwp_windows_msvc_is_correct() {
            let arch =
                WixArch::try_from(&Cfg::of("thumbv7a-uwp-windows-msvc").expect("Cfg parsing"))
                    .unwrap();
            assert_eq!(arch, WixArch::Arm);
        }

        #[test]
        fn from_str_is_correct() {
            let arch = WixArch::from_str("thumbv7a-uwp-windows-msvc").unwrap();
            assert_eq!(arch, WixArch::Arm);
        }
    }
}
