// Copyright (C) 2020 Brian Cook (aka Coding Badly).
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

//! The implementation for is_bundle_build.  This function probes a compiler output file (wxiobj)
//! for indications of what is being built: product or bundle installer.

use crate::Result;

use encoding_rs_io::DecodeReaderBytes;
use sxd_document::parser;
use sxd_xpath::{Context, Factory};

use std::fs::File;
use std::io::Read;
use std::path::Path;

#[derive(Debug)]
pub enum BuildType {
    Unknown,
    Product,
    Bundle,
}

pub fn get_build_type(wxiobj: &Path) -> Result<BuildType> {
    let file = File::open(wxiobj)?;
    let mut decoder = DecodeReaderBytes::new(file);
    let mut content = String::new();
    decoder.read_to_string(&mut content)?;
    let package = parser::parse(&content)?;
    let document = package.as_document();
    let mut context = Context::new();
    context.set_namespace("wix", "http://schemas.microsoft.com/wix/2006/objects");
    // The assumption is that the following cannot fail because the path is known to be valid at
    // compile-time.
    let xpath = Factory::new().build("/wix:wixObject/wix:section/@type").unwrap().unwrap();
    let value = xpath
        .evaluate(&context, document.root())?
        .string();
    if value == "product" {
        Ok(BuildType::Product)
    }
    else if value == "bundle" {
        Ok(BuildType::Bundle)
    }
    else {
        Ok(BuildType::Unknown)
    }
}
