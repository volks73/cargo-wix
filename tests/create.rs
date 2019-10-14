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
#[macro_use]
extern crate lazy_static;
extern crate predicates;
extern crate toml;
extern crate wix;

mod common;

use assert_fs::prelude::*;
use predicates::prelude::*;

use common::{PACKAGE_NAME, TARGET_NAME};
use std::env;
use std::fs::{self, File};
use std::io::{Read, Write};
use std::path::PathBuf;
use toml::Value;
use wix::create::{Builder, Execution};
use wix::initialize;
use wix::{CARGO_MANIFEST_FILE, WIX};

lazy_static! {
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
    let expected_msi_file = TARGET_WIX_DIR.join(format!("{}-0.1.0-x86_64.msi", PACKAGE_NAME));
    env::set_current_dir(package.path()).unwrap();
    initialize::Execution::default().run().unwrap();
    let result = Execution::default().run();
    env::set_current_dir(original_working_directory).unwrap();
    result.expect("OK result");
    package
        .child(TARGET_WIX_DIR.as_path())
        .assert(predicate::path::exists());
    package
        .child(expected_msi_file)
        .assert(predicate::path::exists());
}

#[test]
fn metadata_works() {
    let original_working_directory = env::current_dir().unwrap();
    let package = common::create_test_package_metadata();
    let expected_msi_file = TARGET_WIX_DIR.join("Metadata-2.1.0-x86_64.msi");
    env::set_current_dir(package.path()).unwrap();
    initialize::Execution::default().run().unwrap();
    let result = Execution::default().run();
    env::set_current_dir(original_working_directory).unwrap();
    result.expect("OK result");
    package
        .child(TARGET_WIX_DIR.as_path())
        .assert(predicate::path::exists());
    package
        .child(expected_msi_file)
        .assert(predicate::path::exists());
}

#[test]
fn output_trailing_forwardslash_works() {
    let output_dir = PathBuf::from(TARGET_NAME).join("output_dir");
    let output_dir_str = format!("{}/", output_dir.to_str().unwrap());
    let original_working_directory = env::current_dir().unwrap();
    let package = common::create_test_package();
    let expected_msi_file = output_dir.join(format!("{}-0.1.0-x86_64.msi", PACKAGE_NAME));
    env::set_current_dir(package.path()).unwrap();
    initialize::Execution::default().run().unwrap();
    let result = Builder::new()
        .output(Some(output_dir_str.as_str()))
        .build()
        .run();
    env::set_current_dir(original_working_directory).unwrap();
    result.expect("OK result");
    package
        .child(TARGET_WIX_DIR.as_path())
        .assert(predicate::path::exists());
    package
        .child(expected_msi_file)
        .assert(predicate::path::exists());
}

#[test]
fn output_trailing_backslash_works() {
    let output_dir = PathBuf::from(TARGET_NAME).join("output_dir");
    let output_dir_str = format!("{}\\", output_dir.to_str().unwrap());
    let original_working_directory = env::current_dir().unwrap();
    let package = common::create_test_package();
    let expected_msi_file = output_dir.join(format!("{}-0.1.0-x86_64.msi", PACKAGE_NAME));
    env::set_current_dir(package.path()).unwrap();
    initialize::Execution::default().run().unwrap();
    let result = Builder::new()
        .output(Some(output_dir_str.as_str()))
        .build()
        .run();
    env::set_current_dir(original_working_directory).unwrap();
    result.expect("OK result");
    package
        .child(TARGET_WIX_DIR.as_path())
        .assert(predicate::path::exists());
    package
        .child(expected_msi_file)
        .assert(predicate::path::exists());
}

#[test]
fn output_existing_dir_works() {
    let output_dir = PathBuf::from("output_dir");
    let original_working_directory = env::current_dir().unwrap();
    let package = common::create_test_package();
    let expected_msi_file = output_dir.join(format!("{}-0.1.0-x86_64.msi", PACKAGE_NAME));
    env::set_current_dir(package.path()).unwrap();
    fs::create_dir(&output_dir).unwrap();
    initialize::Execution::default().run().unwrap();
    let result = Builder::new().output(output_dir.to_str()).build().run();
    env::set_current_dir(original_working_directory).unwrap();
    result.expect("OK result");
    package
        .child(TARGET_WIX_DIR.as_path())
        .assert(predicate::path::exists());
    package
        .child(expected_msi_file)
        .assert(predicate::path::exists());
}

#[test]
fn output_file_without_extension_works() {
    let output_dir = PathBuf::from(TARGET_NAME).join("output_dir");
    let original_working_directory = env::current_dir().unwrap();
    let package = common::create_test_package();
    let output_file = output_dir.join(PACKAGE_NAME);
    let expected_msi_file = output_dir.join(format!("{}.msi", PACKAGE_NAME));
    env::set_current_dir(package.path()).unwrap();
    initialize::Execution::default().run().unwrap();
    let result = Builder::new().output(output_file.to_str()).build().run();
    env::set_current_dir(original_working_directory).unwrap();
    result.expect("OK result");
    package
        .child(TARGET_WIX_DIR.as_path())
        .assert(predicate::path::exists());
    package
        .child(expected_msi_file)
        .assert(predicate::path::exists());
}

#[test]
fn output_file_with_extension_works() {
    let output_dir = PathBuf::from(TARGET_NAME).join("output_dir");
    let original_working_directory = env::current_dir().unwrap();
    let package = common::create_test_package();
    let expected_msi_file = output_dir.join(format!("{}.msi", PACKAGE_NAME));
    env::set_current_dir(package.path()).unwrap();
    initialize::Execution::default().run().unwrap();
    let result = Builder::new()
        .output(expected_msi_file.to_str())
        .build()
        .run();
    env::set_current_dir(original_working_directory).unwrap();
    result.expect("OK result");
    package
        .child(TARGET_WIX_DIR.as_path())
        .assert(predicate::path::exists());
    package
        .child(expected_msi_file)
        .assert(predicate::path::exists());
}

#[test]
fn init_with_package_section_fields_works() {
    let original_working_directory = env::current_dir().unwrap();
    let package = common::create_test_package();
    let expected_msi_file = TARGET_WIX_DIR.join(format!("{}-0.1.0-x86_64.msi", PACKAGE_NAME));
    env::set_current_dir(package.path()).unwrap();
    let package_manifest = package.child("Cargo.toml");
    let mut toml: Value = {
        let mut cargo_toml_handle = File::open(package_manifest.path()).unwrap();
        let mut cargo_toml_content = String::new();
        cargo_toml_handle
            .read_to_string(&mut cargo_toml_content)
            .unwrap();
        toml::from_str(&cargo_toml_content).unwrap()
    };
    {
        toml.get_mut("package")
            .and_then(|p| {
                match p {
                    Value::Table(ref mut t) => {
                        t.insert(
                            String::from("description"),
                            Value::from("This is a description"),
                        );
                        t.insert(
                            String::from("documentation"),
                            Value::from("https://www.example.com/docs"),
                        );
                        t.insert(
                            String::from("homepage"),
                            Value::from("https://www.example.com"),
                        );
                        t.insert(String::from("license"), Value::from("MIT"));
                        t.insert(
                            String::from("repository"),
                            Value::from("https://www.example.com/repo"),
                        );
                    }
                    _ => panic!("The 'package' section is not a table"),
                };
                Some(p)
            })
            .expect("A package section for the Cargo.toml");
        let toml_string = toml.to_string();
        let mut cargo_toml_handle = File::create(package_manifest.path()).unwrap();
        cargo_toml_handle.write_all(toml_string.as_bytes()).unwrap();
    }
    initialize::Execution::default().run().unwrap();
    let result = Execution::default().run();
    env::set_current_dir(original_working_directory).unwrap();
    result.expect("OK result");
    package
        .child(TARGET_WIX_DIR.as_path())
        .assert(predicate::path::exists());
    package
        .child(expected_msi_file)
        .assert(predicate::path::exists());
}

#[test]
fn init_with_all_options_works() {
    const LICENSE_FILE: &str = "License_Example.txt";
    const EULA_FILE: &str = "Eula_Example.rtf";
    let original_working_directory = env::current_dir().unwrap();
    let package = common::create_test_package();
    let expected_msi_file = TARGET_WIX_DIR.join(format!("{}-0.1.0-x86_64.msi", PACKAGE_NAME));
    env::set_current_dir(package.path()).unwrap();
    let bin_example_path = package.path().join("bin").join("Example.exe");
    fs::create_dir(bin_example_path.parent().unwrap()).unwrap();
    {
        let _bin_example_handle = File::create(&bin_example_path).unwrap();
    }
    let banner_path = package.path().join("img").join("Banner.bmp");
    fs::create_dir(banner_path.parent().unwrap()).unwrap();
    {
        let _banner_handle = File::create(&banner_path).unwrap();
    }
    let dialog_path = package.path().join("img").join("Dialog.bmp");
    {
        let _dialog_handle = File::create(&dialog_path).unwrap();
    }
    let package_license = package.child(LICENSE_FILE);
    {
        let _license_handle = File::create(package_license.path()).unwrap();
    }
    let package_eula = package.child(EULA_FILE);
    {
        let _eula_handle = File::create(package_eula.path()).unwrap();
    }
    let product_icon_path = package.path().join("img").join("Product.ico");
    {
        let _product_icon_handle = File::create(&product_icon_path).unwrap();
    }
    initialize::Builder::new()
        .banner(banner_path.to_str())
        .binaries(bin_example_path.to_str().map(|b| vec![b]))
        .description(Some("This is a description"))
        .dialog(dialog_path.to_str())
        .eula(package_eula.path().to_str())
        .help_url(Some("http://www.example.com"))
        .license(package_license.path().to_str())
        .manufacturer(Some("Example Manufacturer"))
        .product_icon(product_icon_path.to_str())
        .product_name(Some("Example Product Name"))
        .build()
        .run()
        .unwrap();
    let result = Execution::default().run();
    env::set_current_dir(original_working_directory).unwrap();
    result.expect("OK result");
    package
        .child(TARGET_WIX_DIR.as_path())
        .assert(predicate::path::exists());
    package
        .child(expected_msi_file)
        .assert(predicate::path::exists());
}

#[test]
fn init_with_banner_option_works() {
    let original_working_directory = env::current_dir().unwrap();
    let package = common::create_test_package();
    let expected_msi_file = TARGET_WIX_DIR.join(format!("{}-0.1.0-x86_64.msi", PACKAGE_NAME));
    env::set_current_dir(package.path()).unwrap();
    let banner_path = package.path().join("img").join("Banner.bmp");
    fs::create_dir(banner_path.parent().unwrap()).unwrap();
    {
        let _banner_handle = File::create(&banner_path).unwrap();
    }
    initialize::Builder::new()
        .banner(banner_path.to_str())
        .build()
        .run()
        .unwrap();
    let result = Execution::default().run();
    env::set_current_dir(original_working_directory).unwrap();
    result.expect("OK result");
    package
        .child(TARGET_WIX_DIR.as_path())
        .assert(predicate::path::exists());
    package
        .child(expected_msi_file)
        .assert(predicate::path::exists());
}

#[test]
fn init_with_binaries_option_works() {
    let original_working_directory = env::current_dir().unwrap();
    let package = common::create_test_package();
    let expected_msi_file = TARGET_WIX_DIR.join(format!("{}-0.1.0-x86_64.msi", PACKAGE_NAME));
    env::set_current_dir(package.path()).unwrap();
    let bin_example_path = package.path().join("bin").join("Example.exe");
    fs::create_dir(bin_example_path.parent().unwrap()).unwrap();
    {
        let _bin_example_handle = File::create(&bin_example_path).unwrap();
    }
    initialize::Builder::new()
        .binaries(bin_example_path.to_str().map(|b| vec![b]))
        .build()
        .run()
        .unwrap();
    let result = Execution::default().run();
    env::set_current_dir(original_working_directory).unwrap();
    result.expect("OK result");
    package
        .child(TARGET_WIX_DIR.as_path())
        .assert(predicate::path::exists());
    package
        .child(expected_msi_file)
        .assert(predicate::path::exists());
}

#[test]
fn init_with_multiple_binaries_works() {
    let original_working_directory = env::current_dir().unwrap();
    let package = common::create_test_package_multiple_binaries();
    let expected_msi_file = TARGET_WIX_DIR.join(format!("{}-0.1.0-x86_64.msi", PACKAGE_NAME));
    env::set_current_dir(package.path()).unwrap();
    initialize::Builder::new().build().run().unwrap();
    let result = Execution::default().run();
    env::set_current_dir(original_working_directory).unwrap();
    result.expect("OK result");
    package
        .child(TARGET_WIX_DIR.as_path())
        .assert(predicate::path::exists());
    package
        .child(expected_msi_file)
        .assert(predicate::path::exists());
}

#[test]
fn init_with_description_option_works() {
    let original_working_directory = env::current_dir().unwrap();
    let package = common::create_test_package();
    let expected_msi_file = TARGET_WIX_DIR.join(format!("{}-0.1.0-x86_64.msi", PACKAGE_NAME));
    env::set_current_dir(package.path()).unwrap();
    initialize::Builder::new()
        .description(Some("This is a description"))
        .build()
        .run()
        .unwrap();
    let result = Execution::default().run();
    env::set_current_dir(original_working_directory).unwrap();
    result.expect("OK result");
    package
        .child(TARGET_WIX_DIR.as_path())
        .assert(predicate::path::exists());
    package
        .child(expected_msi_file)
        .assert(predicate::path::exists());
}

#[test]
fn init_with_dialog_option_works() {
    let original_working_directory = env::current_dir().unwrap();
    let package = common::create_test_package();
    let expected_msi_file = TARGET_WIX_DIR.join(format!("{}-0.1.0-x86_64.msi", PACKAGE_NAME));
    env::set_current_dir(package.path()).unwrap();
    let dialog_path = package.path().join("img").join("Dialog.bmp");
    fs::create_dir(dialog_path.parent().unwrap()).unwrap();
    {
        let _dialog_handle = File::create(&dialog_path).unwrap();
    }
    initialize::Builder::new()
        .dialog(dialog_path.to_str())
        .build()
        .run()
        .unwrap();
    let result = Execution::default().run();
    env::set_current_dir(original_working_directory).unwrap();
    result.expect("OK result");
    package
        .child(TARGET_WIX_DIR.as_path())
        .assert(predicate::path::exists());
    package
        .child(expected_msi_file)
        .assert(predicate::path::exists());
}

#[test]
fn init_with_eula_in_cwd_works() {
    const EULA_FILE: &str = "Eula_Example.rtf";
    let original_working_directory = env::current_dir().unwrap();
    let package = common::create_test_package();
    let expected_msi_file = TARGET_WIX_DIR.join(format!("{}-0.1.0-x86_64.msi", PACKAGE_NAME));
    env::set_current_dir(package.path()).unwrap();
    let package_eula = package.child(EULA_FILE);
    {
        let _eula_handle = File::create(package_eula.path()).unwrap();
    }
    initialize::Builder::new()
        .eula(package_eula.path().to_str())
        .build()
        .run()
        .expect("Initialization");
    let result = Execution::default().run();
    env::set_current_dir(original_working_directory).unwrap();
    result.expect("OK result");
    package
        .child(TARGET_WIX_DIR.as_path())
        .assert(predicate::path::exists());
    package
        .child(expected_msi_file)
        .assert(predicate::path::exists());
}

#[test]
fn init_with_eula_in_docs_works() {
    const EULA_FILE: &str = "Eula_Example.rtf";
    let original_working_directory = env::current_dir().unwrap();
    let package = common::create_test_package();
    let expected_msi_file = TARGET_WIX_DIR.join(format!("{}-0.1.0-x86_64.msi", PACKAGE_NAME));
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
    result.expect("OK result");
    package
        .child(TARGET_WIX_DIR.as_path())
        .assert(predicate::path::exists());
    package
        .child(expected_msi_file)
        .assert(predicate::path::exists());
}

#[test]
fn init_with_help_url_option_works() {
    let original_working_directory = env::current_dir().unwrap();
    let package = common::create_test_package();
    let expected_msi_file = TARGET_WIX_DIR.join(format!("{}-0.1.0-x86_64.msi", PACKAGE_NAME));
    env::set_current_dir(package.path()).unwrap();
    initialize::Builder::new()
        .help_url(Some("http://www.example.com"))
        .build()
        .run()
        .unwrap();
    let result = Execution::default().run();
    env::set_current_dir(original_working_directory).unwrap();
    result.expect("OK result");
    package
        .child(TARGET_WIX_DIR.as_path())
        .assert(predicate::path::exists());
    package
        .child(expected_msi_file)
        .assert(predicate::path::exists());
}

#[test]
fn init_with_license_in_cwd_works() {
    const LICENSE_FILE: &str = "License_Example.txt";
    let original_working_directory = env::current_dir().unwrap();
    let package = common::create_test_package();
    let expected_msi_file = TARGET_WIX_DIR.join(format!("{}-0.1.0-x86_64.msi", PACKAGE_NAME));
    env::set_current_dir(package.path()).unwrap();
    let package_license = package.child(LICENSE_FILE);
    {
        let _license_handle = File::create(package_license.path()).unwrap();
    }
    initialize::Builder::new()
        .license(package_license.path().to_str())
        .build()
        .run()
        .unwrap();
    let result = Execution::default().run();
    env::set_current_dir(original_working_directory).unwrap();
    result.expect("OK result");
    package
        .child(TARGET_WIX_DIR.as_path())
        .assert(predicate::path::exists());
    package
        .child(expected_msi_file)
        .assert(predicate::path::exists());
}

#[test]
fn init_with_license_in_docs_works() {
    const EULA_FILE: &str = "License_Example.txt";
    let original_working_directory = env::current_dir().unwrap();
    let package = common::create_test_package();
    let expected_msi_file = TARGET_WIX_DIR.join(format!("{}-0.1.0-x86_64.msi", PACKAGE_NAME));
    env::set_current_dir(package.path()).unwrap();
    let package_docs = package.child("docs");
    fs::create_dir(package_docs.path()).unwrap();
    let package_license = package_docs.path().join(EULA_FILE);
    {
        let _license_handle = File::create(&package_license).unwrap();
    }
    initialize::Builder::new()
        .license(package_license.to_str())
        .build()
        .run()
        .expect("Initialization");
    let result = Execution::default().run();
    env::set_current_dir(original_working_directory).unwrap();
    result.expect("OK result");
    package
        .child(TARGET_WIX_DIR.as_path())
        .assert(predicate::path::exists());
    package
        .child(expected_msi_file)
        .assert(predicate::path::exists());
}

#[test]
fn init_with_manufacturer_option_works() {
    let original_working_directory = env::current_dir().unwrap();
    let package = common::create_test_package();
    let expected_msi_file = TARGET_WIX_DIR.join(format!("{}-0.1.0-x86_64.msi", PACKAGE_NAME));
    env::set_current_dir(package.path()).unwrap();
    initialize::Builder::new()
        .manufacturer(Some("Example Manufacturer"))
        .build()
        .run()
        .unwrap();
    let result = Execution::default().run();
    env::set_current_dir(original_working_directory).unwrap();
    result.expect("OK result");
    package
        .child(TARGET_WIX_DIR.as_path())
        .assert(predicate::path::exists());
    package
        .child(expected_msi_file)
        .assert(predicate::path::exists());
}

#[test]
fn init_with_product_icon_option_works() {
    let original_working_directory = env::current_dir().unwrap();
    let package = common::create_test_package();
    let expected_msi_file = TARGET_WIX_DIR.join(format!("{}-0.1.0-x86_64.msi", PACKAGE_NAME));
    env::set_current_dir(package.path()).unwrap();
    let product_icon_path = package.path().join("img").join("Product.ico");
    fs::create_dir(product_icon_path.parent().unwrap()).unwrap();
    {
        let _product_icon_handle = File::create(&product_icon_path).unwrap();
    }
    initialize::Builder::new()
        .product_icon(product_icon_path.to_str())
        .build()
        .run()
        .unwrap();
    let result = Execution::default().run();
    env::set_current_dir(original_working_directory).unwrap();
    result.expect("OK result");
    package
        .child(TARGET_WIX_DIR.as_path())
        .assert(predicate::path::exists());
    package
        .child(expected_msi_file)
        .assert(predicate::path::exists());
}

#[test]
fn init_with_product_name_option_works() {
    let original_working_directory = env::current_dir().unwrap();
    let package = common::create_test_package();
    let expected_msi_file = TARGET_WIX_DIR.join(format!("{}-0.1.0-x86_64.msi", PACKAGE_NAME));
    env::set_current_dir(package.path()).unwrap();
    initialize::Builder::new()
        .product_name(Some("Example Product Name"))
        .build()
        .run()
        .unwrap();
    let mut wxs_handle =
        File::open(package.child(PathBuf::from(WIX).join("main.wxs")).path()).unwrap();
    let mut wxs_content = String::new();
    wxs_handle.read_to_string(&mut wxs_content).unwrap();
    println!("{}", wxs_content);
    let result = Execution::default().run();
    env::set_current_dir(original_working_directory).unwrap();
    result.expect("OK result");
    package
        .child(TARGET_WIX_DIR.as_path())
        .assert(predicate::path::exists());
    package
        .child(expected_msi_file)
        .assert(predicate::path::exists());
}

#[test]
fn input_works() {
    let original_working_directory = env::current_dir().unwrap();
    let package = common::create_test_package();
    let package_manifest = package.child(CARGO_MANIFEST_FILE);
    let output = package.path().join("assets").join("windows");
    fs::create_dir(output.parent().unwrap()).unwrap();
    fs::create_dir(&output).unwrap();
    let expected_msi_file = TARGET_WIX_DIR.join(format!("{}-0.1.0-x86_64.msi", PACKAGE_NAME));
    let mut toml: Value = {
        let mut cargo_toml_handle = File::open(package_manifest.path()).unwrap();
        let mut cargo_toml_content = String::new();
        cargo_toml_handle
            .read_to_string(&mut cargo_toml_content)
            .unwrap();
        toml::from_str(&cargo_toml_content).unwrap()
    };
    {
        toml.get_mut("package")
            .and_then(|p| {
                match p {
                    Value::Table(ref mut t) => {
                        t.insert(
                            String::from("description"),
                            Value::from("This is a description"),
                        );
                        t.insert(
                            String::from("documentation"),
                            Value::from("https://www.example.com/docs"),
                        );
                        t.insert(
                            String::from("homepage"),
                            Value::from("https://www.example.com"),
                        );
                        t.insert(String::from("license"), Value::from("MIT"));
                        t.insert(
                            String::from("repository"),
                            Value::from("https://www.example.com/repo"),
                        );
                    }
                    _ => panic!("The 'package' section is not a table"),
                };
                Some(p)
            })
            .expect("A package section for the Cargo.toml");
        let toml_string = toml.to_string();
        let mut cargo_toml_handle = File::create(package_manifest.path()).unwrap();
        cargo_toml_handle.write_all(toml_string.as_bytes()).unwrap();
    }
    env::set_current_dir(package.path()).unwrap();
    initialize::Builder::default()
        .output(output.to_str())
        .build()
        .run()
        .unwrap();
    let result = Builder::default()
        .input(output.join("main.wxs").to_str())
        .build()
        .run();
    env::set_current_dir(original_working_directory).unwrap();
    result.expect("OK result");
    package
        .child(TARGET_WIX_DIR.as_path())
        .assert(predicate::path::exists());
    package
        .child(expected_msi_file)
        .assert(predicate::path::exists());
}
