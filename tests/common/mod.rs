extern crate tempfile;

use std::process::Command;

#[allow(dead_code)]
pub const WIX_NAME: &str = "wix";
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

