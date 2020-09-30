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

//! The implementation for the `print` command. This command is focused on
//! printing various templates based on a package's manifest (Cargo.toml) or
//! end-user input.

pub mod license;
pub mod wxs;

use crate::Error;
use crate::Result;

use regex::Regex;

use std::fs::File;
use std::io::{self, Write};
use std::path::PathBuf;

use cargo_metadata::Package;

fn destination(output: Option<&PathBuf>) -> Result<Box<dyn Write>> {
    if let Some(ref output) = output {
        trace!("An output path has been explicity specified");
        let f = File::create(output)?;
        Ok(Box::new(f))
    } else {
        trace!(
            "An output path has NOT been explicity specified. Implicitly \
             determine output."
        );
        Ok(Box::new(io::stdout()))
    }
}

fn first_author(package: &Package) -> Result<String> {
    package.authors.first()
        .and_then(|s| {
            // Strip email if it exists.
            let re = Regex::new(r"<(.*?)>").unwrap();
            Some(re.replace_all(s, ""))
        })
        .map(|s| String::from(s.trim()))
        .ok_or(Error::Manifest("authors"))
}

#[cfg(test)]
mod tests {
    use super::*;

    const SINGLE_AUTHOR_MANIFEST: &str = r#"{
            "name": "Example",
            "version": "0.1.0",
            "authors": ["First Last <first.last@example.com>"],

            "id": "",
            "dependencies": [],
            "targets": [],
            "features": {},
            "manifest_path": ""
        }"#;

    const MULTIPLE_AUTHORS_MANIFEST: &str = r#"{
            "name": "Example",
            "version": "0.1.0",
            "authors": ["1 Author <first.last@example.com>", "2 Author <2.author@example.com>", "3 author <3.author@example.com>"],

            "id": "",
            "dependencies": [],
            "targets": [],
            "features": {},
            "manifest_path": ""
        }"#;

    #[test]
    fn first_author_with_single_author_works() {
        let manifest = serde_json::from_str(SINGLE_AUTHOR_MANIFEST)
            .expect("Parsing TOML");
        let actual = first_author(&manifest).unwrap();
        assert_eq!(actual, String::from("First Last"));
    }

    #[test]
    fn first_author_with_multiple_authors_works() {
        let manifest = serde_json::from_str(MULTIPLE_AUTHORS_MANIFEST)
            .expect("Parsing TOML");
        let actual = first_author(&manifest).unwrap();
        assert_eq!(actual, String::from("1 Author"));
    }
}
