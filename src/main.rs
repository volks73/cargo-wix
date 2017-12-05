// Copyright (C) 2017 Christopher R. Field.

extern crate ansi_term;
extern crate atty;
#[macro_use] extern crate clap;
#[macro_use] extern crate log;
extern crate loggerv;
extern crate toml;

use ansi_term::Colour;
use clap::{App, Arg, SubCommand};
use std::fs::File;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::process::Command;
use toml::Value;

const SUBCOMMAND_NAME: &str = "wix";
const WIX_TOOLSET_COMPILER: &str = "candle";
const WIX_TOOLSET_LINKER: &str = "light";
const SIGNTOOL: &str = "signtool";

const ERROR_COLOR: Colour = Colour::Fixed(9); // Bright red

fn main() {
    // Based on documentation for the ansi_term crate, Windows 10 supports ANSI escape characters,
    // but it must be enabled first. The ansi_term crate provides a function for enabling ANSI
    // support in Windows, but it is conditionally compiled and only exists for Windows builds. To
    // avoid build errors on non-windows platforms, a cfg guard should be put in place.
    #[cfg(windows)] ansi_term::enable_ansi_support().expect("Enable ANSI support on Windows");
    // TODO: Add verbosity logging
    let matches = App::new(crate_name!())
        .bin_name("cargo")
        .subcommand(
            SubCommand::with_name(SUBCOMMAND_NAME)
                .version(crate_version!())
                .about(crate_description!())
                .author(crate_authors!())
                .arg(Arg::with_name("sign")
                     .help("The Windows installer (msi) will be signed using the SignTool application available in the Windows 10 SDK. The signtool is invoked with the '/a' flag to automatically obtain an appropriate certificate from the Windows certificate manager. The default is to also use the Comodo timestamp server with the '/t' flag.")
                     .short("s")
                     .long("sign"))
                .arg(Arg::with_name("win64")
                     .help("Builds the installer for the x64 platform. The default is to build the installer for the x86 platform.")
                     .long("win64"))
                .arg(Arg::with_name("verbose")
                     .help("Sets the level of verbosity. The higher the level of verbosity, the more information that is printed and logged when the application is executed. This flag can be specified multiple times, where each occurrance increases the level and/or details written for each statement.")
                     .long("verbose")
                     .short("v")
                     .multiple(true))
        ).get_matches();
    let matches = matches.subcommand_matches(SUBCOMMAND_NAME).unwrap();
    if matches.occurrences_of("verbose") > 3 {
        loggerv::Logger::new()
            .line_numbers(true)
            .module_path(true)
    } else {
        loggerv::Logger::new()
            .module_path(false)
    }.verbosity(matches.occurrences_of("verbose"))
    .level(true)
    .init()
    .expect("logger to initiate");
    let platform = if matches.is_present("win64") {
        "x64"
    } else {
        "x86"
    };
    debug!("platform = {:?}", platform);
    let cargo_file_path = Path::new("Cargo.toml");
    debug!("cargo_file_path = {:?}", cargo_file_path);
    let mut cargo_file = File::open(cargo_file_path).expect("Open Cargo.toml file");
    let mut cargo_file_content = String::new();
    cargo_file.read_to_string(&mut cargo_file_content).expect("Read to string");
    let cargo_values = cargo_file_content.parse::<Value>().expect("Parse cargo file contents");
    let pkg_version = cargo_values
        .get("package")
        .and_then(|p| p.as_table())
        .and_then(|t| t.get("version"))
        .and_then(|v| v.as_str())
        .expect("Package version");
    debug!("pkg_version = {:?}", pkg_version);
    let pkg_name = cargo_values
        .get("package")
        .and_then(|p| p.as_table())
        .and_then(|t| t.get("name"))
        .and_then(|n| n.as_str())
        .expect("Package name");
    debug!("pkg_name = {:?}", pkg_name);
    let pkg_description = cargo_values
        .get("package")
        .and_then(|p| p.as_table())
        .and_then(|t| t.get("description"))
        .and_then(|d| d.as_str())
        .expect("Package description");
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
    // Build the binary with the release profile. A release binary has already been built, this
    // will essentially do nothing.
    info!("Building release binary");
    if let Some(status) = Command::new("cargo")
        .arg("build")
        .arg("--release")
        .status()
        .ok() {
        if !status.success() {
            panic!("Failed to build the release executable");
        }
    }
    // Compile the installer
    info!("Compiling installer");
    if let Some(status) = Command::new(WIX_TOOLSET_COMPILER)
        .arg(format!("-dVersion={}", pkg_version))
        .arg(format!("-dPlatform={}", platform))
        .arg(format!("-dProductName={}", pkg_name))
        .arg(format!("-dBinaryName={}", bin_name))
        .arg(format!("-dDescription={}", pkg_description))
        .arg("-o")
        .arg(&main_wixobj)
        .arg(&main_wxs)
        .status()
        .ok() {
        if !status.success() {
            panic!("Failed to compile the installer");
        }
    }
    // Link the installer
    info!("Linking the installer");
    if let Some(status) = Command::new(WIX_TOOLSET_LINKER)
        .arg("-ext")
        .arg("WixUIExtension")
        .arg("-cultures:en-us")
        .arg(&main_wixobj)
        .arg("-out")
        .arg(&main_msi)
        .status()
        .ok() {
        if !status.success() {
            panic!("Failed to link the installer");
        }
    }
    // Sign the installer
    if matches.is_present("sign") {
        info!("Signing the installer");
        if let Some(status) = Command::new(SIGNTOOL)
            .arg("sign")
            .arg("/a")
            .arg("/t")
            .arg("http://timestamp.comodoca.com")
            .arg(&main_msi)
            .status()
            .ok() {
            if !status.success() {
                panic!("Failed to sign the installer");
            }
        }
    }
    // TODO: Wrap execution into a function that returns a Result
    //match result {
        //Ok(_) => {
            //std::process::exit(0);
        //},
        //Err(e) => {
            //let mut tag = format!("Error[{}] ({})", e.code(), e.description());
            //if atty::is(atty::Stream::Stderr) {
                //tag = ERROR_COLOR.paint(tag).to_string()
            //}
            //writeln!(&mut std::io::stderr(), "{}: {}", tag, e)
                //.expect("Writing to stderr");
            //std::process::exit(e.code());
        //}
    //}
}
