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
use std::path::PathBuf;

lazy_static!{
    static ref TARGET_WIX_DIR: PathBuf = {
        let mut p = PathBuf::from(TARGET_NAME);
        p.push(WIX);
        p
    };
}

#[test]
fn default_execution_works() {
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

