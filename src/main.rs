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

extern crate ansi_term;
extern crate atty;
#[macro_use] extern crate clap;
#[macro_use] extern crate log;
extern crate loggerv;
extern crate toml;

use ansi_term::Colour;
use clap::{App, Arg, SubCommand};
use std::error::Error as StdError;
use std::fs::File;
use std::fmt;
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use toml::Value;

const CARGO_MANIFEST_FILE: &str = "Cargo.toml";
const SUBCOMMAND_NAME: &str = "wix";
const WIX_TOOLSET_COMPILER: &str = "candle";
const WIX_TOOLSET_LINKER: &str = "light";
const SIGNTOOL: &str = "signtool";

const ERROR_COLOR: Colour = Colour::Fixed(9); // Bright red

/// The template, or example, WiX Source (WXS) file.
static TEMPLATE: &str = include_str!("template.wxs");

#[derive(Debug)]
enum Error {
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
            Error::Sign(..) => 7,
            Error::Toml(..) => 8,
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
            Error::Sign(..) => "Sign",
            Error::Toml(..) => "TOML",
        }
    }

    fn cause(&self) -> Option<&StdError> {
        match *self {
            Error::Io(ref err) => Some(err),
            Error::Toml(ref err) => Some(err),
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

#[derive(Debug, Clone, Copy, PartialEq)]
enum Platform {
    X86,
    X64
}

impl fmt::Display for Platform {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            Platform::X86 => write!(f, "x86"),
            Platform::X64 => write!(f, "x64"),
        }
    }
}

/// Prints the template to stdout
fn print_template() -> Result<(), Error> {
    io::stdout().write(TEMPLATE.as_bytes())?;
    Ok(())
}

/// Runs the subcommand to build the release binary, compile, link, and possibly sign the installer
/// (msi).
fn run(platform: Platform, sign: bool, capture_output: bool) -> Result<(), Error> {
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
    let pkg_author = cargo_values
        .get("package")
        .and_then(|p| p.as_table())
        .and_then(|t| t.get("authors"))
        .and_then(|a| a.as_array())
        .and_then(|a| a.get(0)) // For now, just use the first author
        .and_then(|f| f.as_str())
        .ok_or(Error::Manifest(String::from("authors")))?;
    debug!("pkg_description = {:?}", pkg_description);
    let bin_name = cargo_values
        .get("bin")
        .and_then(|b| b.as_table())
        .and_then(|t| t.get("name"))
        .and_then(|n| n.as_str())
        .unwrap_or(pkg_name);
    debug!("bin_name = {:?}", bin_name);
    let mut main_wxs = PathBuf::from("wix");
    main_wxs.push("main");
    main_wxs.set_extension("wxs");
    debug!("main_wxs = {:?}", main_wxs);
    let mut main_wixobj = PathBuf::from("target");
    main_wixobj.push("wix");
    main_wixobj.push("build");
    main_wixobj.push("main");
    main_wixobj.set_extension("wixobj");
    debug!("main_wixobj = {:?}", main_wixobj);
    let mut main_msi = PathBuf::from("target");
    main_msi.push("wix");
    // Do NOT use the `set_extension` method for the MSI path. Since the pkg_version is in X.X.X
    // format, the `set_extension` method will replace the Patch version number and
    // architecture/platform with `msi`.  Instead, just include the extension in the formatted
    // name.
    main_msi.push(&format!("{}-{}-{}.msi", pkg_name, pkg_version, platform));
    debug!("main_msi = {:?}", main_msi);
    // Build the binary with the release profile. If a release binary has already been built, then
    // this will essentially do nothing.
    info!("Building release binary");
    if let Some(status) = {
        let mut builder = Command::new("cargo");
        if capture_output {
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
        let mut compiler = Command::new(WIX_TOOLSET_COMPILER);
        if capture_output {
            compiler.stdout(Stdio::null());
            compiler.stderr(Stdio::null());
        } 
        compiler.arg(format!("-dVersion={}", pkg_version))
            .arg(format!("-dPlatform={}", platform))
            .arg(format!("-dProductName={}", pkg_name))
            .arg(format!("-dBinaryName={}", bin_name))
            .arg(format!("-dDescription={}", pkg_description))
            .arg(format!("-dAuthor={}", pkg_author))
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
        let mut linker = Command::new(WIX_TOOLSET_LINKER);
        if capture_output {    
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
    if sign {
        info!("Signing the installer");
        if let Some(status) = {
            let mut signer = Command::new(SIGNTOOL);
            if capture_output {
                signer.stdout(Stdio::null());
                signer.stderr(Stdio::null());
            }
            signer.arg("sign")
                .arg("/a")
                .arg("/t")
                .arg("http://timestamp.comodoca.com")
                .arg(&main_msi)
                .status()
        }.ok() {
            if !status.success() {
                // TODO: Add better error message
                return Err(Error::Sign(String::from("Failed to sign the installer")));
            }
        }
    }
    Ok(())
}

fn main() {
    // Based on documentation for the ansi_term crate, Windows 10 supports ANSI escape characters,
    // but it must be enabled first. The ansi_term crate provides a function for enabling ANSI
    // support in Windows, but it is conditionally compiled and only exists for Windows builds. To
    // avoid build errors on non-windows platforms, a cfg guard should be put in place.
    #[cfg(windows)] ansi_term::enable_ansi_support().expect("Enable ANSI support on Windows");
    let matches = App::new(crate_name!())
        .bin_name("cargo")
        .subcommand(
            SubCommand::with_name(SUBCOMMAND_NAME)
                .version(crate_version!())
                .about(crate_description!())
                .author(crate_authors!())
                .arg(Arg::with_name("nocapture")
                     .help("By default, this subcommand captures, or hides, all output from the builder, compiler, linker, and signer for the binary and Windows installer, respectively. Use this flag to show the output.")
                     .long("nocapture"))
                .arg(Arg::with_name("print-template")
                     .help("Prints a template WiX Source (wxs) file to use with this subcommand to stdout. The template provided with this subcommand uses xml preprocessor varaibles to set values based on fields in the rust project's manifest file (Cargo.toml). Only the '{{replace-with-a-guid}}' placeholders within the template need to be modified with unique GUIDs by hand. Redirection can be used to save the contents to 'main.wxs' and then placed in the 'wix' subfolder.")
                     .long("print-template"))
                .arg(Arg::with_name("sign")
                     .help("The Windows installer (msi) will be signed using the SignTool application available in the Windows 10 SDK. The signtool is invoked with the '/a' flag to automatically obtain an appropriate certificate from the Windows certificate manager. The default is to also use the Comodo timestamp server with the '/t' flag.")
                     .short("s")
                     .long("sign"))
                .arg(Arg::with_name("verbose")
                     .help("Sets the level of verbosity. The higher the level of verbosity, the more information that is printed and logged when the application is executed. This flag can be specified multiple times, where each occurrance increases the level and/or details written for each statement.")
                     .long("verbose")
                     .short("v")
                     .multiple(true))
                .arg(Arg::with_name("x64")
                     .help("Builds the installer for the x64 platform. The default is to build the installer for the x86 platform.")
                     .long("x64"))
        ).get_matches();
    let matches = matches.subcommand_matches(SUBCOMMAND_NAME).unwrap();
    let verbosity = matches.occurrences_of("verbose");
    let capture_output = !matches.is_present("nocapture");
    if verbosity > 3 {
        loggerv::Logger::new()
            .line_numbers(true)
            .module_path(true)
    } else {
        loggerv::Logger::new()
            .module_path(false)
    }.verbosity(verbosity)
    .level(true)
    .init()
    .expect("logger to initiate");
    let platform = if matches.is_present("x64") {
        Platform::X64
    } else {
        Platform::X86
    };
    debug!("platform = {:?}", platform);
    let result = if matches.is_present("print-template") {
        print_template()
    } else {
        run(platform, matches.is_present("sign"), capture_output) 
    };
    match result {
        Ok(_) => {
            std::process::exit(0);
        },
        Err(e) => {
            let mut tag = format!("Error[{}] ({})", e.code(), e.description());
            if atty::is(atty::Stream::Stderr) {
                tag = ERROR_COLOR.paint(tag).to_string()
            }
            writeln!(&mut std::io::stderr(), "{}: {}", tag, e)
                .expect("Writing to stderr");
            std::process::exit(e.code());
        }
    }
}
