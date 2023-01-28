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

mod common;

use assert_fs::prelude::*;
use predicates::prelude::*;

use assert_fs::TempDir;

use lazy_static::lazy_static;

use serial_test::serial;

use std::env;
use std::fs::{self, File};
use std::io::{Read, Write};
use std::path::PathBuf;

use toml::Value;

use wix::initialize::{Builder, Execution};
use wix::{
    CARGO_MANIFEST_FILE, LICENSE_FILE_NAME, RTF_FILE_EXTENSION, WIX, WIX_SOURCE_FILE_EXTENSION,
    WIX_SOURCE_FILE_NAME,
};

use crate::common::{add_license_to_package, init_logging, SUBPACKAGE1_NAME, SUBPACKAGE2_NAME};

lazy_static! {
    static ref MAIN_WXS: String = WIX_SOURCE_FILE_NAME.to_owned() + "." + WIX_SOURCE_FILE_EXTENSION;
    static ref LICENSE_RTF: String = LICENSE_FILE_NAME.to_owned() + "." + RTF_FILE_EXTENSION;
    static ref WIX_PATH: PathBuf = PathBuf::from(WIX);
    static ref MAIN_WXS_PATH: PathBuf = PathBuf::from(WIX).join(MAIN_WXS.as_str());
    static ref LICENSE_RTF_PATH: PathBuf = PathBuf::from(WIX).join(LICENSE_RTF.as_str());
}

#[test]
#[serial]
fn default_works() {
    // Save the current working directory so that we can change back to it at
    // the end of the test. This avoids polluting the `tests` folder for the
    // source code with test artifacts.
    let original_working_directory = env::current_dir().unwrap();
    let package = common::create_test_package();
    env::set_current_dir(package.path()).unwrap();
    let result = Execution::default().run();
    env::set_current_dir(original_working_directory).unwrap();
    println!("{result:?}");
    assert!(result.is_ok());
    package
        .child(WIX_PATH.as_path())
        .assert(predicate::path::exists());
    package
        .child(MAIN_WXS_PATH.as_path())
        .assert(predicate::path::exists());
    package
        .child(LICENSE_RTF_PATH.as_path())
        .assert(predicate::path::missing());
}

#[test]
#[serial]
fn description_works() {
    const EXPECTED: &str = "This is a description";
    let original_working_directory = env::current_dir().unwrap();
    let package = common::create_test_package();
    env::set_current_dir(package.path()).unwrap();
    let result = Builder::default().description(Some(EXPECTED)).build().run();
    env::set_current_dir(original_working_directory).unwrap();
    assert!(result.is_ok());
    let actual = common::evaluate_xpath(
        package.child(MAIN_WXS_PATH.as_path()).path(),
        "/wix:Wix/wix:Product/wix:Package/@Description",
    );
    assert_eq!(actual, EXPECTED);
}

#[test]
#[serial]
fn help_url_works() {
    const EXPECTED: &str = "http://www.example.com";
    let original_working_directory = env::current_dir().unwrap();
    let package = common::create_test_package();
    env::set_current_dir(package.path()).unwrap();
    let result = Builder::default().help_url(Some(EXPECTED)).build().run();
    env::set_current_dir(original_working_directory).unwrap();
    assert!(result.is_ok());
    let actual = common::evaluate_xpath(
        package.child(MAIN_WXS_PATH.as_path()).path(),
        "/wix:Wix/wix:Product/wix:Property[@Id='ARPHELPLINK']/@Value",
    );
    assert_eq!(actual, EXPECTED);
}

#[test]
#[serial]
fn manufacturer_works() {
    const EXPECTED: &str = "Example Manufacturer";
    let original_working_directory = env::current_dir().unwrap();
    let package = common::create_test_package();
    env::set_current_dir(package.path()).unwrap();
    let result = Builder::default()
        .manufacturer(Some(EXPECTED))
        .build()
        .run();
    env::set_current_dir(original_working_directory).unwrap();
    assert!(result.is_ok());
    let actual = common::evaluate_xpath(
        package.child(MAIN_WXS_PATH.as_path()).path(),
        "/wix:Wix/wix:Product/wix:Package/@Manufacturer",
    );
    assert_eq!(actual, EXPECTED);
}

#[test]
#[serial]
fn product_name_works() {
    const EXPECTED: &str = "Example Product Name";
    let original_working_directory = env::current_dir().unwrap();
    let package = common::create_test_package();
    env::set_current_dir(package.path()).unwrap();
    let result = Builder::default()
        .product_name(Some(EXPECTED))
        .build()
        .run();
    env::set_current_dir(original_working_directory).unwrap();
    assert!(result.is_ok());
    assert_eq!(
        common::evaluate_xpath(
            package.child(MAIN_WXS_PATH.as_path()).path(),
            "/wix:Wix/wix:Product/@Name"
        ),
        EXPECTED
    );
    assert_eq!(
        common::evaluate_xpath(
            package.child(MAIN_WXS_PATH.as_path()).path(),
            "/wix:Wix/wix:Product/wix:Property[@Id='DiskPrompt']/@Value"
        ),
        EXPECTED.to_string() + " Installation"
    );
    assert_eq!(
        common::evaluate_xpath(
            package.child(MAIN_WXS_PATH.as_path()).path(),
            "//*/wix:Directory[@Id='APPLICATIONFOLDER']/@Name"
        ),
        EXPECTED
    );
}

#[test]
#[serial]
fn binaries_works() {
    const EXPECTED: &str = "bin\\Example.exe";
    let original_working_directory = env::current_dir().unwrap();
    let package = common::create_test_package();
    env::set_current_dir(package.path()).unwrap();
    let result = Builder::default()
        .binaries(Some(vec![EXPECTED]))
        .build()
        .run();
    env::set_current_dir(original_working_directory).unwrap();
    assert!(result.is_ok());
    assert_eq!(
        common::evaluate_xpath(
            package.child(MAIN_WXS_PATH.as_path()).path(),
            "//*/wix:File[@Id='exe0']/@Name"
        ),
        "Example.exe"
    );
    assert_eq!(
        common::evaluate_xpath(
            package.child(MAIN_WXS_PATH.as_path()).path(),
            "//*/wix:File[@Id='exe0']/@Source"
        ),
        EXPECTED
    );
}

#[test]
fn input_works() {
    let package = common::create_test_package();
    Builder::default()
        .input(package.child(CARGO_MANIFEST_FILE).path().to_str())
        .build()
        .run()
        .expect("OK result");
    package.child(WIX).assert(predicate::path::exists());
    package
        .child(MAIN_WXS_PATH.as_path())
        .assert(predicate::path::exists());
    package
        .child(LICENSE_RTF_PATH.as_path())
        .assert(predicate::path::missing());
}

#[test]
#[serial]
fn output_works() {
    let original_working_directory = env::current_dir().unwrap();
    let package = common::create_test_package();
    let output = TempDir::new().unwrap();
    env::set_current_dir(package.path()).unwrap();
    let result = Builder::default()
        .output(output.path().to_str())
        .build()
        .run();
    env::set_current_dir(original_working_directory).unwrap();
    assert!(result.is_ok());
    output
        .child(MAIN_WXS.as_str())
        .assert(predicate::path::exists());
}

#[test]
#[serial]
fn input_with_output_works() {
    let package = common::create_test_package();
    let package_manifest = package.child(CARGO_MANIFEST_FILE);
    let output = package.path().join("assets").join("windows");
    fs::create_dir(output.parent().unwrap()).unwrap();
    fs::create_dir(&output).unwrap();
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
            .map(|p| {
                match p {
                    Value::Table(ref mut t) => {
                        t.insert(String::from("license"), Value::from("MIT"))
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
    Builder::default()
        .input(package.child(CARGO_MANIFEST_FILE).path().to_str())
        .output(output.to_str())
        .build()
        .run()
        .expect("OK result");
    assert!(output.join(MAIN_WXS.as_str()).exists());
    assert!(output.join(LICENSE_RTF.as_str()).exists());
}

#[test]
#[serial]
fn license_with_txt_file_works() {
    const EXPECTED: &str = "License_Example.txt";
    let original_working_directory = env::current_dir().unwrap();
    let package = common::create_test_package();
    let package_license = package.child(EXPECTED);
    env::set_current_dir(package.path()).unwrap();
    let _license_handle = File::create(package_license.path()).unwrap();
    let result = Builder::default()
        .license(package_license.path().to_str())
        .build()
        .run();
    env::set_current_dir(original_working_directory).unwrap();
    assert!(result.is_ok());
    assert_eq!(
        common::evaluate_xpath(
            package.child(MAIN_WXS_PATH.as_path()).path(),
            "//*/wix:File[@Id='LicenseFile']/@Name"
        ),
        EXPECTED
    );
    assert_eq!(
        common::evaluate_xpath(
            package.child(MAIN_WXS_PATH.as_path()).path(),
            "//*/wix:File[@Id='LicenseFile']/@Source"
        ),
        package_license.path().to_str().unwrap()
    );
}

#[test]
#[serial]
fn license_with_rtf_file_works() {
    const EXPECTED: &str = "License_Example.rtf";
    let original_working_directory = env::current_dir().unwrap();
    let package = common::create_test_package();
    let package_license = package.child(EXPECTED);
    env::set_current_dir(package.path()).unwrap();
    let _license_handle = File::create(package_license.path()).unwrap();
    let result = Builder::default()
        .license(package_license.path().to_str())
        .build()
        .run();
    env::set_current_dir(original_working_directory).unwrap();
    assert!(result.is_ok());
    assert_eq!(
        common::evaluate_xpath(
            package.child(MAIN_WXS_PATH.as_path()).path(),
            "//*/wix:File[@Id='LicenseFile']/@Name"
        ),
        EXPECTED
    );
    assert_eq!(
        common::evaluate_xpath(
            package.child(MAIN_WXS_PATH.as_path()).path(),
            "//*/wix:File[@Id='LicenseFile']/@Source"
        ),
        package_license.path().to_str().unwrap()
    );
    assert_eq!(
        common::evaluate_xpath(
            package.child(MAIN_WXS_PATH.as_path()).path(),
            "//*/wix:WixVariable[@Id='WixUILicenseRtf']/@Value"
        ),
        package_license.path().to_str().unwrap()
    );
}

#[test]
#[serial]
fn eula_works() {
    const EXPECTED: &str = "EULA_Example.rtf";
    let original_working_directory = env::current_dir().unwrap();
    let package = common::create_test_package();
    let package_eula = package.child(EXPECTED);
    env::set_current_dir(package.path()).unwrap();
    let _eula_handle = File::create(package_eula.path()).unwrap();
    let result = Builder::default()
        .eula(package_eula.path().to_str())
        .build()
        .run();
    env::set_current_dir(original_working_directory).unwrap();
    assert!(result.is_ok());
    assert_eq!(
        common::evaluate_xpath(
            package.child(MAIN_WXS_PATH.as_path()).path(),
            "//*/wix:WixVariable[@Id='WixUILicenseRtf']/@Value"
        ),
        package_eula.path().to_str().unwrap()
    );
}

#[test]
#[serial]
fn mit_license_id_works() {
    let original_working_directory = env::current_dir().unwrap();
    let package = common::create_test_package();
    env::set_current_dir(package.path()).unwrap();
    let package_manifest = package.child(CARGO_MANIFEST_FILE);
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
            .map(|p| {
                match p {
                    Value::Table(ref mut t) => {
                        t.insert(String::from("license"), Value::from("MIT"))
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
    let result = Execution::default().run();
    env::set_current_dir(original_working_directory).unwrap();
    assert!(result.is_ok());
    package
        .child(WIX_PATH.as_path())
        .assert(predicate::path::exists());
    package
        .child(MAIN_WXS_PATH.as_path())
        .assert(predicate::path::exists());
    package
        .child(LICENSE_RTF_PATH.as_path())
        .assert(predicate::path::exists());
    assert_eq!(
        common::evaluate_xpath(
            package.child(MAIN_WXS_PATH.as_path()).path(),
            "//*/wix:File[@Id='LicenseFile']/@Name"
        ),
        LICENSE_RTF.to_owned()
    );
    assert_eq!(
        common::evaluate_xpath(
            package.child(MAIN_WXS_PATH.as_path()).path(),
            "//*/wix:File[@Id='LicenseFile']/@Source"
        ),
        LICENSE_RTF_PATH.display().to_string()
    );
    assert_eq!(
        common::evaluate_xpath(
            package.child(MAIN_WXS_PATH.as_path()).path(),
            "//*/wix:WixVariable[@Id='WixUILicenseRtf']/@Value"
        ),
        LICENSE_RTF_PATH.display().to_string()
    );
}

#[test]
#[serial]
fn apache2_license_id_works() {
    let original_working_directory = env::current_dir().unwrap();
    let package = common::create_test_package();
    env::set_current_dir(package.path()).unwrap();
    let package_manifest = package.child(CARGO_MANIFEST_FILE);
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
            .map(|p| {
                match p {
                    Value::Table(ref mut t) => {
                        t.insert(String::from("license"), Value::from("Apache-2.0"))
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
    let result = Execution::default().run();
    env::set_current_dir(original_working_directory).unwrap();
    assert!(result.is_ok());
    package
        .child(WIX_PATH.as_path())
        .assert(predicate::path::exists());
    package
        .child(MAIN_WXS_PATH.as_path())
        .assert(predicate::path::exists());
    package
        .child(LICENSE_RTF_PATH.as_path())
        .assert(predicate::path::exists());
    assert_eq!(
        common::evaluate_xpath(
            package.child(MAIN_WXS_PATH.as_path()).path(),
            "//*/wix:File[@Id='LicenseFile']/@Name"
        ),
        LICENSE_RTF.to_owned()
    );
    assert_eq!(
        common::evaluate_xpath(
            package.child(MAIN_WXS_PATH.as_path()).path(),
            "//*/wix:File[@Id='LicenseFile']/@Source"
        ),
        LICENSE_RTF_PATH.display().to_string()
    );
    assert_eq!(
        common::evaluate_xpath(
            package.child(MAIN_WXS_PATH.as_path()).path(),
            "//*/wix:WixVariable[@Id='WixUILicenseRtf']/@Value"
        ),
        LICENSE_RTF_PATH.display().to_string()
    );
}

#[test]
#[serial]
fn gpl3_license_id_works() {
    let original_working_directory = env::current_dir().unwrap();
    let package = common::create_test_package();
    env::set_current_dir(package.path()).unwrap();
    let package_manifest = package.child(CARGO_MANIFEST_FILE);
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
            .map(|p| {
                match p {
                    Value::Table(ref mut t) => {
                        t.insert(String::from("license"), Value::from("GPL-3.0"))
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
    let result = Execution::default().run();
    env::set_current_dir(original_working_directory).unwrap();
    assert!(result.is_ok());
    package
        .child(WIX_PATH.as_path())
        .assert(predicate::path::exists());
    package
        .child(MAIN_WXS_PATH.as_path())
        .assert(predicate::path::exists());
    package
        .child(LICENSE_RTF_PATH.as_path())
        .assert(predicate::path::exists());
    assert_eq!(
        common::evaluate_xpath(
            package.child(MAIN_WXS_PATH.as_path()).path(),
            "//*/wix:File[@Id='LicenseFile']/@Name"
        ),
        LICENSE_RTF.to_owned()
    );
    assert_eq!(
        common::evaluate_xpath(
            package.child(MAIN_WXS_PATH.as_path()).path(),
            "//*/wix:File[@Id='LicenseFile']/@Source"
        ),
        LICENSE_RTF_PATH.display().to_string()
    );
    assert_eq!(
        common::evaluate_xpath(
            package.child(MAIN_WXS_PATH.as_path()).path(),
            "//*/wix:WixVariable[@Id='WixUILicenseRtf']/@Value"
        ),
        LICENSE_RTF_PATH.display().to_string()
    );
}

#[test]
#[serial]
fn license_file_field_with_rtf_file_works() {
    const EXPECTED: &str = "License_Example.rtf";
    let original_working_directory = env::current_dir().unwrap();
    let package = common::create_test_package();
    let package_license = package.child(EXPECTED);
    env::set_current_dir(package.path()).unwrap();
    let _license_handle = File::create(package_license.path()).unwrap();
    let package_manifest = package.child(CARGO_MANIFEST_FILE);
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
            .map(|p| {
                match p {
                    Value::Table(ref mut t) => t.insert(
                        String::from("license-file"),
                        Value::from(package_license.path().to_str().unwrap()),
                    ),
                    _ => panic!("The 'package' section is not a table"),
                };
                Some(p)
            })
            .expect("A package section for the Cargo.toml");
        let toml_string = toml.to_string();
        let mut cargo_toml_handle = File::create(package_manifest.path()).unwrap();
        cargo_toml_handle.write_all(toml_string.as_bytes()).unwrap();
    }
    let result = Execution::default().run();
    env::set_current_dir(original_working_directory).unwrap();
    assert!(result.is_ok());
    assert_eq!(
        common::evaluate_xpath(
            package.child(MAIN_WXS_PATH.as_path()).path(),
            "//*/wix:File[@Id='LicenseFile']/@Name"
        ),
        EXPECTED
    );
    assert_eq!(
        common::evaluate_xpath(
            package.child(MAIN_WXS_PATH.as_path()).path(),
            "//*/wix:File[@Id='LicenseFile']/@Source"
        ),
        package_license.path().to_str().unwrap()
    );
    assert_eq!(
        common::evaluate_xpath(
            package.child(MAIN_WXS_PATH.as_path()).path(),
            "//*/wix:WixVariable[@Id='WixUILicenseRtf']/@Value"
        ),
        package_license.path().to_str().unwrap()
    );
}

#[test]
#[serial]
fn license_file_field_with_txt_file_works() {
    const EXPECTED: &str = "License_Example.txt";
    let original_working_directory = env::current_dir().unwrap();
    let package = common::create_test_package();
    let package_license = package.child(EXPECTED);
    env::set_current_dir(package.path()).unwrap();
    let _license_handle = File::create(package_license.path()).unwrap();
    let package_manifest = package.child(CARGO_MANIFEST_FILE);
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
            .map(|p| {
                match p {
                    Value::Table(ref mut t) => t.insert(
                        String::from("license-file"),
                        Value::from(package_license.path().to_str().unwrap()),
                    ),
                    _ => panic!("The 'package' section is not a table"),
                };
                Some(p)
            })
            .expect("A package section for the Cargo.toml");
        let toml_string = toml.to_string();
        let mut cargo_toml_handle = File::create(package_manifest.path()).unwrap();
        cargo_toml_handle.write_all(toml_string.as_bytes()).unwrap();
    }
    let result = Execution::default().run();
    env::set_current_dir(original_working_directory).unwrap();
    assert!(result.is_ok());
    assert_eq!(
        common::evaluate_xpath(
            package.child(MAIN_WXS_PATH.as_path()).path(),
            "//*/wix:File[@Id='LicenseFile']/@Name"
        ),
        EXPECTED
    );
    assert_eq!(
        common::evaluate_xpath(
            package.child(MAIN_WXS_PATH.as_path()).path(),
            "//*/wix:File[@Id='LicenseFile']/@Source"
        ),
        package_license.path().to_str().unwrap()
    );
}

#[test]
#[serial]
fn banner_works() {
    const EXPECTED: &str = "img\\Banner.bmp";
    let original_working_directory = env::current_dir().unwrap();
    let package = common::create_test_package();
    let package_banner = package.child(EXPECTED);
    env::set_current_dir(package.path()).unwrap();
    fs::create_dir("img").unwrap();
    let _banner_handle = File::create(package_banner.path()).unwrap();
    let result = Builder::default()
        .banner(package_banner.path().to_str())
        .build()
        .run();
    env::set_current_dir(original_working_directory).unwrap();
    assert!(result.is_ok());
    assert_eq!(
        common::evaluate_xpath(
            package.child(MAIN_WXS_PATH.as_path()).path(),
            "//*/wix:WixVariable[@Id='WixUIBannerBmp']/@Value"
        ),
        package_banner.path().to_str().unwrap()
    );
}

#[test]
#[serial]
fn dialog_works() {
    const EXPECTED: &str = "img\\Dialog.bmp";
    let original_working_directory = env::current_dir().unwrap();
    let package = common::create_test_package();
    let package_dialog = package.child(EXPECTED);
    env::set_current_dir(package.path()).unwrap();
    fs::create_dir("img").unwrap();
    let _dialog_handle = File::create(package_dialog.path()).unwrap();
    let result = Builder::default()
        .dialog(package_dialog.path().to_str())
        .build()
        .run();
    env::set_current_dir(original_working_directory).unwrap();
    assert!(result.is_ok());
    assert_eq!(
        common::evaluate_xpath(
            package.child(MAIN_WXS_PATH.as_path()).path(),
            "//*/wix:WixVariable[@Id='WixUIDialogBmp']/@Value"
        ),
        package_dialog.path().to_str().unwrap()
    );
}

#[test]
#[serial]
fn product_icon_works() {
    const EXPECTED: &str = "img\\Product.ico";
    let original_working_directory = env::current_dir().unwrap();
    let package = common::create_test_package();
    let package_product_icon = package.child(EXPECTED);
    env::set_current_dir(package.path()).unwrap();
    fs::create_dir("img").unwrap();
    let _product_icon_handle = File::create(package_product_icon.path()).unwrap();
    let result = Builder::default()
        .product_icon(package_product_icon.path().to_str())
        .build()
        .run();
    env::set_current_dir(original_working_directory).unwrap();
    assert!(result.is_ok());
    assert_eq!(
        common::evaluate_xpath(
            package.child(MAIN_WXS_PATH.as_path()).path(),
            "//*/wix:Icon[@Id='ProductICO']/@SourceFile"
        ),
        package_product_icon.path().to_str().unwrap()
    );
}

#[test]
#[serial]
fn multiple_binaries_works() {
    const EXPECTED_NAME_1: &str = "main1";
    const EXPECTED_SOURCE_1: &str = "$(var.CargoTargetBinDir)\\main1.exe";
    const EXPECTED_NAME_2: &str = "main2";
    const EXPECTED_SOURCE_2: &str = "$(var.CargoTargetBinDir)\\main2.exe";
    const EXPECTED_NAME_3: &str = "main3";
    const EXPECTED_SOURCE_3: &str = "$(var.CargoTargetBinDir)\\main3.exe";
    let original_working_directory = env::current_dir().unwrap();
    let package = common::create_test_package_multiple_binaries();
    env::set_current_dir(package.path()).unwrap();
    let result = Builder::default().build().run();
    env::set_current_dir(original_working_directory).unwrap();
    assert!(result.is_ok());
    assert_eq!(
        common::evaluate_xpath(
            package.child(MAIN_WXS_PATH.as_path()).path(),
            "//*/wix:File[@Id='exe0']/@Name"
        ),
        format!("{EXPECTED_NAME_1}.exe")
    );
    assert_eq!(
        common::evaluate_xpath(
            package.child(MAIN_WXS_PATH.as_path()).path(),
            "//*/wix:File[@Id='exe0']/@Source"
        ),
        EXPECTED_SOURCE_1
    );
    assert_eq!(
        common::evaluate_xpath(
            package.child(MAIN_WXS_PATH.as_path()).path(),
            "//*/wix:File[@Id='exe1']/@Name"
        ),
        format!("{EXPECTED_NAME_2}.exe")
    );
    assert_eq!(
        common::evaluate_xpath(
            package.child(MAIN_WXS_PATH.as_path()).path(),
            "//*/wix:File[@Id='exe1']/@Source"
        ),
        EXPECTED_SOURCE_2
    );
    assert_eq!(
        common::evaluate_xpath(
            package.child(MAIN_WXS_PATH.as_path()).path(),
            "//*/wix:File[@Id='exe2']/@Name"
        ),
        format!("{EXPECTED_NAME_3}.exe")
    );
    assert_eq!(
        common::evaluate_xpath(
            package.child(MAIN_WXS_PATH.as_path()).path(),
            "//*/wix:File[@Id='exe2']/@Source"
        ),
        EXPECTED_SOURCE_3
    );
}

#[test]
#[serial]
fn workspace_no_package_fails() {
    init_logging();
    let original_working_directory = env::current_dir().unwrap();
    let package = common::create_test_workspace();
    env::set_current_dir(package.path()).unwrap();
    let result = Builder::default().build().run();
    env::set_current_dir(original_working_directory).unwrap();
    assert!(result.is_err());
}

#[test]
#[serial]
fn workspace_package_works() {
    init_logging();
    let original_working_directory = env::current_dir().unwrap();
    let package = common::create_test_workspace();
    env::set_current_dir(package.path()).unwrap();
    let result = Builder::default()
        .package(Some(SUBPACKAGE1_NAME))
        .build()
        .run();
    env::set_current_dir(original_working_directory).unwrap();
    assert!(result.is_ok());
    package
        .child(SUBPACKAGE1_NAME)
        .child(WIX_PATH.as_path())
        .assert(predicate::path::exists());
    package
        .child(SUBPACKAGE1_NAME)
        .child(MAIN_WXS_PATH.as_path())
        .assert(predicate::path::exists());
    package
        .child(SUBPACKAGE1_NAME)
        .child(LICENSE_RTF_PATH.as_path())
        .assert(predicate::path::missing());
}

#[test]
#[serial]
fn workspace_package_with_license_works() {
    init_logging();
    let original_working_directory = env::current_dir().unwrap();
    let package = common::create_test_workspace();
    add_license_to_package(&package.path().join(SUBPACKAGE1_NAME), "GPL-3.0");
    add_license_to_package(&package.path().join(SUBPACKAGE2_NAME), "GPL-3.0");

    env::set_current_dir(package.path()).unwrap();
    let result = Builder::default()
        .package(Some(SUBPACKAGE1_NAME))
        .license(Some("license"))
        .build()
        .run();
    env::set_current_dir(original_working_directory).unwrap();
    assert!(result.is_ok());
}
