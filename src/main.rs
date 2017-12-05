#[macro_use] extern crate clap;
extern crate toml;

use clap::{App, Arg, SubCommand};
use std::fs::File;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::Command;
use toml::Value;

const SUBCOMMAND_NAME: &str = "wix";
const WIX_TOOLSET_COMPILER: &str = "candle";
const WIX_TOOLSET_LINKER: &str = "light";
const SIGNTOOL: &str = "signtool";

fn main() {
    // TODO: Add ansi color support
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
        ).get_matches();
    let matches = matches.subcommand_matches(SUBCOMMAND_NAME).unwrap();
    let cargo_file_path = Path::new("Cargo.toml");
    let mut cargo_file = File::open(cargo_file_path).expect("Open Cargo.toml file");
    let mut cargo_file_content = String::new();
    cargo_file.read_to_string(&mut cargo_file_content).expect("Read to string");
    let pkg_values = cargo_file_content.parse::<Value>().expect("Parse cargo file contents");
    let pkg_version = pkg_values
        .get("package")
        .and_then(|p| p.as_table())
        .and_then(|t| t.get("version"))
        .and_then(|v| v.as_str())
        .expect("Package version");
    let pkg_name = pkg_values
        .get("package")
        .and_then(|p| p.as_table())
        .and_then(|t| t.get("name"))
        .and_then(|n| n.as_str())
        .expect("Package name");
    let mut main_wxs = PathBuf::from("wix");
    main_wxs.push("main");
    main_wxs.set_extension("wxs");
    let mut main_wixobj = PathBuf::from("target");
    main_wixobj.push("wix");
    main_wixobj.push("build");
    main_wixobj.push("main");
    main_wixobj.set_extension("wixobj");
    let mut main_msi = PathBuf::from("target");
    main_msi.push("wix");
    main_msi.push(&format!("{}-{}-win64", pkg_name, pkg_version));
    main_msi.set_extension("msi");
    // Build a release executable
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
    if let Some(status) = Command::new(WIX_TOOLSET_COMPILER)
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
    // TODO: Add error handling
}
