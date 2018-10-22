extern crate sxd_document;
extern crate sxd_xpath;
extern crate tempfile;

use std::fs::File;
use std::io::Read;
use std::path::Path;
use std::process::Command;
use self::sxd_document::parser;
use self::sxd_xpath::{Context, Factory};

#[allow(dead_code)]
pub const TARGET_NAME: &str = "target";

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
/// > cargo init --bin --quiet --vcs none "C:\Users\<username>\AppData\Local\Temp\cargo_wix_text_######"
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
pub fn create_test_package() -> tempfile::TempDir {
    // Use a prefix because the default `.tmp` is an invalid name for a Cargo package.
    //
    // Cannot use dashes. WiX Toolset only allows A-Z, a-z, digits, underscores (_), or periods (.)
    // for attribute IDs.
    let temp_dir = tempfile::Builder::new().prefix("cargo_wix_test_").tempdir().unwrap();
    let cargo_init_status = Command::new("cargo")
        .arg("init")
        .arg("--bin")
        .arg("--quiet")
        .arg("--vcs")
        .arg("none")
        .arg(temp_dir.path())
        .status()
        .expect("Creation of test Cargo package");
    assert!(cargo_init_status.success());
    temp_dir
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
    wxs.read_to_string(&mut wxs_content).expect("Read WiX Source file");
    let wxs_package = parser::parse(&wxs_content).expect("Parsing WiX Source file");
    let wxs_document = wxs_package.as_document();
    let mut context = Context::new();
    context.set_namespace("wix", "http://schemas.microsoft.com/wix/2006/wi");
    let xpath = Factory::new().build(xpath).unwrap().unwrap();
    xpath.evaluate(&context, wxs_document.root()).unwrap().string()
}
