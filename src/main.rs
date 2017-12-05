extern crate clap;
extern crate toml;

use std::fs::File;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::Command;
use toml::Value;

const WIX_TOOLSET_COMPILER: &str = "candle";
const WIX_TOOLSET_LINKER: &str = "light";
const SIGNTOOL: &str = "signtool";

fn main() {
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
    let mut installer_wxs = PathBuf::from("wix");
    installer_wxs.push("main");
    installer_wxs.set_extension("wxs");
    let mut installer_wixobj = PathBuf::from("target");
    installer_wixobj.push("wix");
    installer_wixobj.push("build");
    installer_wixobj.push("installer");
    installer_wixobj.set_extension("wixobj");
    let mut installer_msi = PathBuf::from("target");
    installer_msi.push("wix");
    installer_msi.push("installer");
    installer_msi.push(&format!("{}-{}-win64", pkg_name, pkg_version));
    installer_msi.set_extension("msi");
    // Compile the installer
    if let Some(status) = Command::new(WIX_TOOLSET_COMPILER)
        .arg("-o")
        .arg(&installer_wixobj)
        .arg(&installer_wxs)
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
        .arg(&installer_wixobj)
        .arg("-out")
        .arg(&installer_msi)
        .status()
        .ok() {
        if !status.success() {
            panic!("Failed to link the installer");
        }
    }
    // Sign the installer
    if let Some(status) = Command::new(SIGNTOOL)
        .arg("sign")
        .arg("/a")
        .arg("/t")
        .arg("http://timestamp.comodoca.com")
        .arg(&installer_msi)
        .status()
        .ok() {
        if !status.success() {
            panic!("Failed to sign the installer");
        }
    }
}
