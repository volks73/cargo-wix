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
use predicates::prelude::*;

use cargo_wix::initialize::{Builder, Execution};
use common::WIX_NAME;
use std::env;
use std::path::PathBuf;

pub const MAIN_WXS_NAME: &str = "main.wxs";
pub const LICENSE_RTF_NAME: &str = "License.rtf";

lazy_static!{
    static ref WIX: PathBuf = PathBuf::from(WIX_NAME);
    static ref WIX_MAIN_WXS: PathBuf = PathBuf::from(WIX_NAME).join(MAIN_WXS_NAME);
    static ref WIX_LICENSE_RTF: PathBuf = PathBuf::from(WIX_NAME).join(LICENSE_RTF_NAME);
}

#[test]
fn default_execution_works() {
    // Save the current working directory so that we can change back to it at
    // the end of the test. This avoids polluting the `tests` folder for the
    // source code with test artifacts.
    let original_working_directory = env::current_dir().unwrap();
    let package = common::create_test_package();
    env::set_current_dir(package.path()).unwrap();
    let result = Execution::default().run();
    env::set_current_dir(original_working_directory).unwrap();
    assert!(result.is_ok());
    package.child(WIX.as_path()).assert(predicate::path::exists());
    package.child(WIX_MAIN_WXS.as_path()).assert(predicate::path::exists());
    package.child(WIX_LICENSE_RTF.as_path()).assert(predicate::path::missing());
}

#[test]
fn change_description_works() {
    const EXPECTED: &str = "This is a description";
    let original_working_directory = env::current_dir().unwrap();
    let package = common::create_test_package();
    env::set_current_dir(package.path()).unwrap();
    let result = Builder::default().description(Some(EXPECTED)).build().run();
    env::set_current_dir(original_working_directory).unwrap();
    assert!(result.is_ok());
    let actual = common::evaluate_xpath(
        package.child(WIX_MAIN_WXS.as_path()).path(),
        "/wix:Wix/wix:Product/wix:Package/@Description"
    );
    assert_eq!(actual, EXPECTED);
}

#[test]
fn change_help_url_works() {
    const EXPECTED: &str = "http://www.example.com";
    let original_working_directory = env::current_dir().unwrap();
    let package = common::create_test_package();
    env::set_current_dir(package.path()).unwrap();
    let result = Builder::default().help_url(Some(EXPECTED)).build().run();
    env::set_current_dir(original_working_directory).unwrap();
    assert!(result.is_ok());
    let actual = common::evaluate_xpath(
        package.child(WIX_MAIN_WXS.as_path()).path(),
        "/wix:Wix/wix:Product/wix:Property[@Id='ARPHELPLINK']/@Value"
    );
    assert_eq!(actual, EXPECTED);
}

#[test]
fn change_manufacturer_works() {
    const EXPECTED: &str = "Example Manufacturer";
    let original_working_directory = env::current_dir().unwrap();
    let package = common::create_test_package();
    env::set_current_dir(package.path()).unwrap();
    let result = Builder::default().manufacturer(Some(EXPECTED)).build().run();
    env::set_current_dir(original_working_directory).unwrap();
    assert!(result.is_ok());
    let actual = common::evaluate_xpath(
        package.child(WIX_MAIN_WXS.as_path()).path(),
        "/wix:Wix/wix:Product/wix:Package/@Manufacturer"
    );
    assert_eq!(actual, EXPECTED);
}

