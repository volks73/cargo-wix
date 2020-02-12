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

use encoding_rs_io::DecodeReaderBytes;
use xml::reader::{EventReader, XmlEvent};

use std::fs::File;
use std::io::BufReader;
use std::path::Path;

#[derive(Debug)]
pub enum BundleBuildStatus {
    Unknown,
    No,
    Yes,
}

pub fn is_bundle_build(wxiobj: &Path) -> BundleBuildStatus {
    if let Ok(file) = File::open(wxiobj) {
        let file = BufReader::new(file);
        let decoder = DecodeReaderBytes::new(file);
        let parser = EventReader::new(decoder);

        // For each XML element read until the first StartDocument with the name "section".
        for ref parsed in parser {
            if let Ok(ref event) = parsed {
                if let XmlEvent::StartElement{ref name, ref attributes, ..} = event {
                    if name.local_name == "section" {
                        // For each attribute scan for the "type"
                        for ref attribute in attributes {
                             if attribute.name.local_name == "type" {
                                if attribute.value == "product" {
                                    return BundleBuildStatus::No;
                                }
                                else if attribute.value == "bundle" {
                                    return BundleBuildStatus::Yes;
                                }
                             }
                        }
                        break;
                    }
                }
            }
        }
    }
    BundleBuildStatus::Unknown
}
