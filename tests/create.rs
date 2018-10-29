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

extern crate assert_fs;
extern crate cargo_wix;
#[macro_use] extern crate lazy_static;
extern crate predicates;

mod common;

use assert_fs::prelude::*;
use cargo_wix::create::Execution;
use cargo_wix::initialize;
use cargo_wix::WIX;
use common::TARGET_NAME;
use predicates::prelude::*;
use std::env;
use std::fs::{self, File};
use std::io::Read;
use std::path::PathBuf;

lazy_static!{
    static ref TARGET_WIX_DIR: PathBuf = {
        let mut p = PathBuf::from(TARGET_NAME);
        p.push(WIX);
        p
    };
}

#[test]
fn default_works() {
    let original_working_directory = env::current_dir().unwrap();
    let package = common::create_test_package();
    let expected_msi_file = TARGET_WIX_DIR.join(format!(
        "{}-0.1.0-x86_64.msi", package.path().file_name().and_then(|o| o.to_str()).unwrap()
    ));
    env::set_current_dir(package.path()).unwrap();
    initialize::Execution::default().run().unwrap();
    let result = Execution::default().run();
    env::set_current_dir(original_working_directory).unwrap();
    assert!(result.is_ok());
    package.child(TARGET_WIX_DIR.as_path()).assert(predicate::path::exists());
    package.child(expected_msi_file).assert(predicate::path::exists());
}

#[test]
fn all_initialize_options_works() {
    const LICENSE_FILE: &str = "License_Example.txt";
    const EULA_FILE: &str = "Eula_Example.rtf";
    let original_working_directory = env::current_dir().unwrap();
    let package = common::create_test_package();
    let expected_msi_file = TARGET_WIX_DIR.join(format!(
        "{}-0.1.0-x86_64.msi", package.path().file_name().and_then(|o| o.to_str()).unwrap()
    ));
    env::set_current_dir(package.path()).unwrap();
    let package_license = package.child(LICENSE_FILE);
    let _license_handle = File::create(package_license.path()).unwrap();
    let package_eula = package.child(EULA_FILE);
    let _eula_handle = File::create(package_eula.path()).unwrap();
    initialize::Builder::new()
        .binary_name(Some("Example"))
        .description(Some("This is a description"))
        .eula(package_eula.path().to_str())
        .help_url(Some("http://www.example.com"))
        .license(package_license.path().to_str())
        .manufacturer(Some("Example Manufacturer"))
        .product_name(Some("Example Product Name"))
        .build()
        .run()
        .unwrap();
    let result = Execution::default().run();
    env::set_current_dir(original_working_directory).unwrap();
    assert!(result.is_ok());
    package.child(TARGET_WIX_DIR.as_path()).assert(predicate::path::exists());
    package.child(expected_msi_file).assert(predicate::path::exists());
}

#[test]
fn binary_name_initialize_option_works() {
    let original_working_directory = env::current_dir().unwrap();
    let package = common::create_test_package();
    let expected_msi_file = TARGET_WIX_DIR.join(format!(
        "{}-0.1.0-x86_64.msi", package.path().file_name().and_then(|o| o.to_str()).unwrap()
    ));
    env::set_current_dir(package.path()).unwrap();
    initialize::Builder::new()
        .binary_name(Some("Example"))
        .build()
        .run()
        .unwrap();
    let result = Execution::default().run();
    env::set_current_dir(original_working_directory).unwrap();
    assert!(result.is_ok());
    package.child(TARGET_WIX_DIR.as_path()).assert(predicate::path::exists());
    package.child(expected_msi_file).assert(predicate::path::exists());
}

#[test]
fn description_initialize_option_works() {
    let original_working_directory = env::current_dir().unwrap();
    let package = common::create_test_package();
    let expected_msi_file = TARGET_WIX_DIR.join(format!(
        "{}-0.1.0-x86_64.msi", package.path().file_name().and_then(|o| o.to_str()).unwrap()
    ));
    env::set_current_dir(package.path()).unwrap();
    initialize::Builder::new()
        .description(Some("This is a description"))
        .build()
        .run()
        .unwrap();
    let result = Execution::default().run();
    env::set_current_dir(original_working_directory).unwrap();
    assert!(result.is_ok());
    package.child(TARGET_WIX_DIR.as_path()).assert(predicate::path::exists());
    package.child(expected_msi_file).assert(predicate::path::exists());
}

#[test]
fn eula_in_cwd_works() {
    const EULA_FILE: &str = "Eula_Example.rtf";
    let original_working_directory = env::current_dir().unwrap();
    let package = common::create_test_package();
    let expected_msi_file = TARGET_WIX_DIR.join(format!(
        "{}-0.1.0-x86_64.msi", package.path().file_name().and_then(|o| o.to_str()).unwrap()
    ));
    env::set_current_dir(package.path()).unwrap();
    let package_eula = package.child(EULA_FILE);
    {
        let _eula_handle = File::create(package_eula.path()).unwrap();
    }
    initialize::Builder::new()
        .eula(Some(EULA_FILE))
        .build()
        .run()
        .expect("Initialization");
    let result = Execution::default().run();
    env::set_current_dir(original_working_directory).unwrap();
    assert!(result.is_ok());
    package.child(TARGET_WIX_DIR.as_path()).assert(predicate::path::exists());
    package.child(expected_msi_file).assert(predicate::path::exists());
}

#[test]
fn eula_in_docs_works() {
    const EULA_FILE: &str = "Eula_Example.rtf";
    let original_working_directory = env::current_dir().unwrap();
    let package = common::create_test_package();
    let expected_msi_file = TARGET_WIX_DIR.join(format!(
        "{}-0.1.0-x86_64.msi", package.path().file_name().and_then(|o| o.to_str()).unwrap()
    ));
    env::set_current_dir(package.path()).unwrap();
    let package_docs = package.child("docs");
    fs::create_dir(package_docs.path()).unwrap();
    let package_eula = package_docs.path().join(EULA_FILE);
    {
        let _eula_handle = File::create(&package_eula).unwrap();
    }
    initialize::Builder::new()
        .eula(package_eula.to_str())
        .build()
        .run()
        .expect("Initialization");
    let result = Execution::default().run();
    env::set_current_dir(original_working_directory).unwrap();
    assert!(result.is_ok());
    package.child(TARGET_WIX_DIR.as_path()).assert(predicate::path::exists());
    package.child(expected_msi_file).assert(predicate::path::exists());
}

#[test]
fn help_url_initialize_option_works() {
    let original_working_directory = env::current_dir().unwrap();
    let package = common::create_test_package();
    let expected_msi_file = TARGET_WIX_DIR.join(format!(
        "{}-0.1.0-x86_64.msi", package.path().file_name().and_then(|o| o.to_str()).unwrap()
    ));
    env::set_current_dir(package.path()).unwrap();
    initialize::Builder::new()
        .help_url(Some("http://www.example.com"))
        .build()
        .run()
        .unwrap();
    let result = Execution::default().run();
    env::set_current_dir(original_working_directory).unwrap();
    assert!(result.is_ok());
    package.child(TARGET_WIX_DIR.as_path()).assert(predicate::path::exists());
    package.child(expected_msi_file).assert(predicate::path::exists());
}

#[test]
fn license_initialize_option_works() {
    const LICENSE_FILE: &str = "License_Example.txt";
    let original_working_directory = env::current_dir().unwrap();
    let package = common::create_test_package();
    let expected_msi_file = TARGET_WIX_DIR.join(format!(
        "{}-0.1.0-x86_64.msi", package.path().file_name().and_then(|o| o.to_str()).unwrap()
    ));
    env::set_current_dir(package.path()).unwrap();
    let package_license = package.child(LICENSE_FILE);
    let _license_handle = File::create(package_license.path()).unwrap();
    initialize::Builder::new()
        .license(package_license.path().to_str())
        .build()
        .run()
        .unwrap();
    let result = Execution::default().run();
    env::set_current_dir(original_working_directory).unwrap();
    assert!(result.is_ok());
    package.child(TARGET_WIX_DIR.as_path()).assert(predicate::path::exists());
    package.child(expected_msi_file).assert(predicate::path::exists());
}

#[test]
fn manufacturer_initialize_option_works() {
    let original_working_directory = env::current_dir().unwrap();
    let package = common::create_test_package();
    let expected_msi_file = TARGET_WIX_DIR.join(format!(
        "{}-0.1.0-x86_64.msi", package.path().file_name().and_then(|o| o.to_str()).unwrap()
    ));
    env::set_current_dir(package.path()).unwrap();
    initialize::Builder::new()
        .manufacturer(Some("Example Manufacturer"))
        .build()
        .run()
        .unwrap();
    let result = Execution::default().run();
    env::set_current_dir(original_working_directory).unwrap();
    assert!(result.is_ok());
    package.child(TARGET_WIX_DIR.as_path()).assert(predicate::path::exists());
    package.child(expected_msi_file).assert(predicate::path::exists());
}

#[test]
fn product_name_initialize_option_works() {
    let original_working_directory = env::current_dir().unwrap();
    let package = common::create_test_package();
    let expected_msi_file = TARGET_WIX_DIR.join(format!(
        "{}-0.1.0-x86_64.msi", package.path().file_name().and_then(|o| o.to_str()).unwrap()
    ));
    env::set_current_dir(package.path()).unwrap();
    initialize::Builder::new()
        .product_name(Some("Example Product Name"))
        .build()
        .run()
        .unwrap();
    let result = Execution::default().run();
    env::set_current_dir(original_working_directory).unwrap();
    assert!(result.is_ok());
    package.child(TARGET_WIX_DIR.as_path()).assert(predicate::path::exists());
    package.child(expected_msi_file).assert(predicate::path::exists());
}
