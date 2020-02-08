extern crate assert_fs;
extern crate sxd_document;
extern crate sxd_xpath;

use assert_fs::prelude::*;

use self::sxd_document::parser;
use self::sxd_xpath::{Context, Factory};
use assert_fs::TempDir;
use std::fs;
use std::fs::{File, OpenOptions};
use std::io::{Read, Write};
use std::path::Path;
use std::process::Command;

#[allow(dead_code)]
pub const TARGET_NAME: &str = "target";

// Cannot use dashes. WiX Toolset only allows A-Z, a-z, digits, underscores (_), or periods (.)
// for attribute IDs.
#[allow(dead_code)]
pub const PACKAGE_NAME: &str = "cargowixtest";

/// Create a new cargo project/package for a binary project in a temporary
/// directory.
///
/// This provides a unique, isolated Cargo project/package for testing. A
/// temporary directory is created. Then, a cargo project is initialized within
/// the temporary directory. The package/project is initialized without any
/// verison control system (vcs). The command that is ultimately executed to
/// create the cargo project in the temporary directory is:
///
/// ```
/// > cargo init --bin --quiet --vcs none --name cargowixtest "C:\Users\<username>\AppData\Local\Temp\cargo_wix_text_######"
/// ```
///
/// where `<username>` is replaced with the current logged in user for the
/// Windows Operating System (OS) and `######` is a hash ID that guarentees the
/// folder is unique.
///
/// # Panics
///
/// This will panic if a temporary directory fails to be created or if cargo
/// fails to create the project/package.
#[allow(dead_code)]
pub fn create_test_package() -> TempDir {
    let temp_dir = TempDir::new().unwrap();
    let cargo_init_status = Command::new("cargo")
        .arg("init")
        .arg("--bin")
        .arg("--quiet")
        .arg("--vcs")
        .arg("none")
        .arg("--name")
        .arg(PACKAGE_NAME)
        .arg(temp_dir.path())
        .status()
        .expect("Creation of test Cargo package");
    assert!(cargo_init_status.success());
    temp_dir
}

/// Create a new cargo project/package for a project with multiple binaries in a
/// temporary directory. See the [create_test_package] function for more
/// information.
///
/// Following creation of the project, the manifest file (Cargo.toml) is
/// modified to include multiple `[[bin]]` sections for multiple binaries. The
/// original `main.rs` file that is created for the first binary is copied for
/// each of the other binaries. A total of three (3) binaries will be created
/// and added to the manifest file.
///
/// [create_test_package]: fn.create_test_package.html
///
/// # Panics
///
/// This will panic if a temporary directory fails to be created or if cargo
/// fails to create the project/package.
///
/// It will also panic if it cannot modify the manifest file (Cargo.toml) or the
/// project layout for multiple binaries.
#[allow(dead_code)]
pub fn create_test_package_multiple_binaries() -> TempDir {
    let package = create_test_package();
    let package_manifest = package.child("Cargo.toml");
    let package_src = package.child("src");
    {
        let mut cargo_toml_handle = OpenOptions::new()
            .read(true)
            .append(true)
            .open(package_manifest.path())
            .unwrap();
        cargo_toml_handle
            .write_all(
                r#"[[bin]]
name = "main1"
path = "src/main1.rs"

[[bin]]
name = "main2"
path = "src/main2.rs"

[[bin]]
name = "main3"
path = "src/main3.rs"
"#
                .as_bytes(),
            )
            .unwrap();
    }
    let package_original_main = package_src.child("main.rs");
    fs::copy(
        package_original_main.path(),
        package_src.child("main1.rs").path(),
    )
    .unwrap();
    fs::copy(
        package_original_main.path(),
        package_src.child("main2.rs").path(),
    )
    .unwrap();
    fs::copy(
        package_original_main.path(),
        package_src.child("main3.rs").path(),
    )
    .unwrap();
    fs::remove_file(package_original_main.path()).unwrap();
    package
}

/// Create a new cargo project/package for a project with a
/// `[package.metadata.wix]` section.
///
/// Following creation of the project, the manifest file (Cargo.toml) is
/// modified to include a `[package.metadata.wix]` section.
///
/// # Panics
///
/// This will panic if a temporary directory fails to be created or if cargo
/// fails to create the project/package.
///
/// It will also panic if it cannot modify the manifest file (Cargo.toml) or the
/// project layout for multiple binaries.
#[allow(dead_code)]
pub fn create_test_package_metadata() -> TempDir {
    let package = create_test_package();
    let package_manifest = package.child("Cargo.toml");
    let mut cargo_toml_handle = OpenOptions::new()
        .read(true)
        .append(true)
        .open(package_manifest.path())
        .unwrap();
    cargo_toml_handle
        .write_all(
            r#"[package.metadata.wix]
name = "Metadata"
version = "2.1.0"
inputs = ["wix\\main.wxs"]
"#
            .as_bytes(),
        )
        .unwrap();
    package
}

/// Create a new cargo project/package for a project with multiple WXS files.
///
/// # Panics
///
/// This will panic if a temporary directory fails to be created or if cargo
/// fails to create the project/package.
///
/// It will also panic if it cannot modify the manifest file (Cargo.toml) or the
/// project layout for multiple binaries.
///
/// This function will panic if the `wix` sub-folder could not be created.
#[allow(dead_code)]
pub fn create_test_package_multiple_wxs_sources() -> TempDir {
    let one_wxs = include_str!("one.wxs");
    let two_wxs = include_str!("two.wxs");
    let package = create_test_package();
    let mut misc_dir = package.path().join("misc");
    fs::create_dir(&misc_dir).unwrap();
    misc_dir.push("one.wxs");
    let mut one_wxs_handle = File::create(&misc_dir).unwrap();
    one_wxs_handle.write_all(one_wxs.as_bytes()).unwrap();
    misc_dir.pop();
    misc_dir.push("two.wxs");
    let mut two_wxs_handle = File::create(&misc_dir).unwrap();
    two_wxs_handle.write_all(two_wxs.as_bytes()).unwrap();
    package
}

/// Evaluates an XPath expression for a WiX Source file.
///
/// This registers the WiX XML namespace with the `wix` prefix. So, XPath
/// expressions should use `/wix:Wix/` as the start and prefix all element/node
/// names with the `wix:` prefix. Note, attributes should _not_ have the `wix:`
/// prefix.
///
/// All values are currently returned as strings.
#[allow(dead_code)]
pub fn evaluate_xpath(wxs: &Path, xpath: &str) -> String {
    let mut wxs = File::open(wxs).expect("Open Wix Source file");
    let mut wxs_content = String::new();
    wxs.read_to_string(&mut wxs_content)
        .expect("Read WiX Source file");
    let wxs_package = parser::parse(&wxs_content).expect("Parsing WiX Source file");
    let wxs_document = wxs_package.as_document();
    let mut context = Context::new();
    context.set_namespace("wix", "http://schemas.microsoft.com/wix/2006/wi");
    let xpath = Factory::new().build(xpath).unwrap().unwrap();
    xpath
        .evaluate(&context, wxs_document.root())
        .unwrap()
        .string()
}
