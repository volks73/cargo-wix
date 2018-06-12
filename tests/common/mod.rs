extern crate tempfile;

use std::process::Command;

pub fn create_test_package() -> tempfile::TempDir {
    // Use a prefix because the default `.tmp` is an invalid name for a Cargo package.
    let temp_dir = tempfile::Builder::new().prefix("cargo-wix-test-").tempdir().unwrap();
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

