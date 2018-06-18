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
//! variable, but a WIX system environment variable is created during the installation of the WiX
//! Toolset. The `-B,--bin-path` option can also be used to specify a path (relative or absolute)
//! to the WiX Toolset `bin` folder. The order of precedence (descending) is: `-B,--bin-path`
//! option, WIX environment variable, and then the PATH system environment variable. An error will
//! be displayed if the compiler and/or linker cannot be found.
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
extern crate semver;
extern crate toml;
extern crate uuid;

use std::default::Default;
use std::error::Error as StdError;
use std::env;
use std::fmt;
use std::fs::File;
use std::io::{self, ErrorKind, Read};
use std::path::{Path, PathBuf};
use std::str::FromStr;
use toml::Value;

pub mod clean;
pub mod create;
mod eula;
pub mod initialize;
pub mod print;
pub mod purge;
pub mod sign;

pub const BINARY_FOLDER_NAME: &str = "bin";
pub const CARGO_MANIFEST_FILE: &str = "Cargo.toml";
pub const CARGO: &str = "cargo";
pub const EXE_FILE_EXTENSION: &str = "exe";
pub const MSI_FILE_EXTENSION: &str = "msi";
pub const RTF_FILE_EXTENSION: &str = "rtf";
pub const SIGNTOOL: &str = "signtool";
pub const SIGNTOOL_PATH_KEY: &str = "SIGNTOOL_PATH";
pub const TARGET_FOLDER_NAME: &str = "target";
pub const WIX: &str = "wix";
pub const WIX_COMPILER: &str = "candle";
pub const WIX_LINKER: &str = "light";
pub const WIX_PATH_KEY: &str = "WIX";
pub const WIX_SOURCE_FILE_EXTENSION: &str = "wxs";
pub const WIX_SOURCE_FILE_NAME: &str = "main";

/// A specialized `Result` type for cargo-wix operations.
pub type Result<T> = std::result::Result<T, Error>;

/// The WiX Source (wxs) template.
static WIX_SOURCE_TEMPLATE: &str = include_str!("main.wxs.mustache");

/// The Apache-2.0 Rich Text Format (RTF) license template.
static APACHE2_LICENSE_TEMPLATE: &str = include_str!("Apache-2.0.rtf.mustache");

/// The GPL-3.0 Rich Text Format (RTF) license template.
static GPL3_LICENSE_TEMPLATE: &str = include_str!("GPL-3.0.rtf.mustache");

/// The MIT Rich Text Format (RTF) license template.
static MIT_LICENSE_TEMPLATE: &str = include_str!("MIT.rtf.mustache");

fn manifest(input: Option<&PathBuf>) -> Result<Value> {
    let default_manifest = {
        let mut cwd = env::current_dir()?;
        cwd.push(CARGO_MANIFEST_FILE);
        cwd
    };
    let cargo_file_path = input.unwrap_or(&default_manifest);
    debug!("cargo_file_path = {:?}", cargo_file_path);
    let mut cargo_file = File::open(cargo_file_path)?;
    let mut cargo_file_content = String::new();
    cargo_file.read_to_string(&mut cargo_file_content)?;
    let manifest = cargo_file_content.parse::<Value>()?;
    Ok(manifest)
}

fn description(description: Option<String>, manifest: &Value) -> Option<String> {
    description.or(manifest.get("package")
        .and_then(|p| p.as_table())
        .and_then(|t| t.get("description"))
        .and_then(|d| d.as_str())
        .map(String::from))
}

fn product_name(product_name: Option<&String>, manifest: &Value) -> Result<String> {
    if let Some(p) = product_name {
        Ok(p.to_owned())
    } else {
        manifest.get("package")
            .and_then(|p| p.as_table())
            .and_then(|t| t.get("name"))
            .and_then(|n| n.as_str())
            .map(String::from)
            .ok_or(Error::Manifest("name"))
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
    Manifest(&'static str),
    /// An error occurred with rendering the template using the mustache renderer.
    Mustache(mustache::Error),
    /// Parsing of the `Cargo.toml` manifest failed.
    Toml(toml::de::Error),
    /// Parsing error for a version string or field.
    Version(semver::SemVerError),
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
            Error::Version(..) => 7,
        }
    }

    pub fn already_exists(p: &Path) -> Self {
        io::Error::new(ErrorKind::AlreadyExists, p.display().to_string()).into()
    }

    pub fn not_found(p: &Path) -> Self {
        io::Error::new(ErrorKind::NotFound, p.display().to_string()).into()
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
            Error::Version(..) => "Version",
        }
    }

    fn cause(&self) -> Option<&StdError> {
        match *self {
            Error::Io(ref err) => Some(err),
            Error::Mustache(ref err) => Some(err),
            Error::Toml(ref err) => Some(err),
            Error::Version(ref err) => Some(err),
            _ => None
        }
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            Error::Command(ref command, ref code) => 
                write!(f, "The '{}' application failed with exit code = {}. Consider using the \
                       '--nocapture' flag to obtain more information.", command, code),
            Error::Generic(ref msg) => msg.fmt(f),
            Error::Io(ref err) => match err.kind() {
                ErrorKind::AlreadyExists => if let Some(path) = err.get_ref() {
                    write!(f, "The '{}' file already exists. Use the '--force' flag to overwrite the contents.", path)
                } else {
                    err.fmt(f)
                },
                ErrorKind::NotFound => if let Some(path) = err.get_ref() {
                    write!(f, "The '{}' path does not exist", path)
                } else {
                    err.fmt(f)
                }
                _ => err.fmt(f),
            },
            Error::Manifest(ref var) => 
                write!(f, "No '{}' field found in the package's manifest (Cargo.toml)", var),
            Error::Mustache(ref err) => err.fmt(f),
            Error::Toml(ref err) => err.fmt(f),
            Error::Version(ref err) => err.fmt(f),
        }
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

impl From<toml::de::Error> for Error {
    fn from(err: toml::de::Error) -> Self {
        Error::Toml(err)
    }
}

impl From<semver::SemVerError> for Error {
    fn from(err: semver::SemVerError) -> Self {
        Error::Version(err)
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

/// The various culture codes for localization.
///
/// These are taken from the table in the [WixUI
/// localization](http://wixtoolset.org/documentation/manual/v3/wixui/wixui_localization.html)
/// documentation.
#[derive(Clone, Debug)]
pub enum Cultures {
    ArSa,
    BgBg,
    CaEs,
    HrHr,
    CsCz,
    DaDk,
    NlNl,
    EnUs,
    EtEe,
    FiFi,
    FrFr,
    DeDe,
    ElGr,
    HeIl,
    HiIn,
    HuHu,
    ItIt,
    JaJp,
    KkKz,
    KoKr,
    LvLv,
    LtLt,
    NbNo,
    PlPl,
    PtBr,
    PtPt,
    RoRo,
    RuRu,
    SrLatnCs,
    ZhCn,
    SkSk,
    SlSi,
    EsEs,
    SvSe,
    ThTh,
    ZhHk,
    ZhTw,
    TrTr,
    UkUa,
}

impl Cultures {
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
            Cultures::RuRu => write!(f, "ru_RU"),
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
            "ru_ru" => Ok(Cultures::RuRu),
            "sr-Latn-CS" => Ok(Cultures::SrLatnCs),
            "zh-CN" => Ok(Cultures::ZhCn),
            "sk-SK" => Ok(Cultures::SkSk),
            "sl-SI" => Ok(Cultures::SlSi),
            "es-ES" => Ok(Cultures::EsEs),
            "sv-SE" => Ok(Cultures::SvSe),
            "th-TH" => Ok(Cultures::ThTh),
            "zh-HK" => Ok(Cultures::ZhHk),
            "zh-TW" => Ok(Cultures::ZhTw),
            "tr-TR" => Ok(Cultures::TrTr),
            "uk-UA" => Ok(Cultures::UkUa),
            e @ _ => Err(Error::Generic(format!("Unknown '{}' culture", e))),
        }
    }
}

impl Default for Cultures {
    fn default() -> Self {
        Cultures::EnUs
    }
}

