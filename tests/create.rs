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
extern crate predicates;

mod common;

use assert_fs::prelude::*;
use cargo_wix::create::Execution;
use cargo_wix::initialize;
use predicates::prelude::*;
use std::env;

#[test]
fn default_execution_works() {
    let original_working_directory = env::current_dir().unwrap();
    let package = common::create_test_package();
    let expected_msi_file = format!(
        "target\\wix\\{}-0.1.0-x86_64.msi", package.path().file_name().and_then(|o| o.to_str()).unwrap()
    );
    env::set_current_dir(package.path()).unwrap();
    initialize::Execution::default().run().unwrap();
    let result = Execution::default().run();
    env::set_current_dir(original_working_directory).unwrap();
    assert!(result.is_ok());
    package.child("target\\wix").assert(&predicate::path::exists());
    package.child(expected_msi_file).assert(&predicate::path::exists());
}

