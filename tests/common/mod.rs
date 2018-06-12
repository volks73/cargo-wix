extern crate tempfile;

use std::process::Command;

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

