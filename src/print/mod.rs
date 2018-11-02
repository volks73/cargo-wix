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

pub mod license;
pub mod wxs;

use Error;
use regex::Regex;
use Result;
use std::fs::File;
use std::fmt;
use std::io::{self, Write};
use std::path::PathBuf;
use std::str::FromStr;
use toml::Value;

/// The WiX Source (wxs) template.
static WIX_SOURCE_TEMPLATE: &str = include_str!("main.wxs.mustache");

/// The Apache-2.0 Rich Text Format (RTF) license template.
static APACHE2_LICENSE_TEMPLATE: &str = include_str!("Apache-2.0.rtf.mustache");

/// The GPL-3.0 Rich Text Format (RTF) license template.
static GPL3_LICENSE_TEMPLATE: &str = include_str!("GPL-3.0.rtf.mustache");

/// The MIT Rich Text Format (RTF) license template.
static MIT_LICENSE_TEMPLATE: &str = include_str!("MIT.rtf.mustache");

/// The different templates that can be printed using the `--print-template` option.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Template {
    Apache2,
    Gpl3,
    Mit,
    Wxs,
}

impl Template {
    /// Gets the ID for the template.
    ///
    /// In the case of a license template, the ID is the SPDX ID which is also used for the
    /// `license` field in the package's manifest (Cargo.toml). This is also the same value used
    /// with the `--print-template` option.
    pub fn id(&self) -> &str {
        match *self {
            Template::Apache2 => "Apache-2.0",
            Template::Gpl3 => "GPL-3.0",
            Template::Mit => "MIT",
            Template::Wxs => "WXS",
        }
    }

    /// Gets the possible string representations of each variant.
    pub fn possible_values() -> Vec<String> {
        vec![
            Template::Apache2.id().to_owned(), 
            Template::Apache2.id().to_lowercase(), 
            Template::Gpl3.id().to_owned(), 
            Template::Gpl3.id().to_lowercase(), 
            Template::Mit.id().to_owned(), 
            Template::Mit.id().to_lowercase(), 
            Template::Wxs.id().to_owned(),
            Template::Wxs.id().to_lowercase(),
        ]
    }

    /// Gets the IDs of all supported licenses.
    pub fn license_ids() -> Vec<String> {
        vec![
            Template::Apache2.id().to_owned(),
            Template::Gpl3.id().to_owned(),
            Template::Mit.id().to_owned(),
        ]
    }

    /// Gets the embedded contents of the template as a string.
    pub fn to_str(&self) -> &str {
        match *self {
            Template::Apache2 => APACHE2_LICENSE_TEMPLATE,
            Template::Gpl3 => GPL3_LICENSE_TEMPLATE,
            Template::Mit => MIT_LICENSE_TEMPLATE,
            Template::Wxs => WIX_SOURCE_TEMPLATE,
        }
    }
}

impl fmt::Display for Template {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.id())
    }
}

impl FromStr for Template {
    type Err = Error;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        match s.to_lowercase().trim() {
            "apache-2.0" => Ok(Template::Apache2),
            "gpl-3.0" => Ok(Template::Gpl3),
            "mit" => Ok(Template::Mit),
            "wxs" => Ok(Template::Wxs),
            _ => Err(Error::Generic(format!("Cannot convert from '{}' to a Template variant", s))),
        }
    }
}

fn destination(output: Option<&PathBuf>) -> Result<Box<Write>> {
    if let Some(ref output) = output {
        trace!("An output path has been explicity specified");
        let f = File::create(output)?;
        Ok(Box::new(f))
    } else {
        trace!("An output path has NOT been explicity specified. Implicitly \
                determine output.");
        Ok(Box::new(io::stdout()))
    }
}

fn first_author(manifest: &Value) -> Result<String> {
    manifest.get("package")
        .and_then(|p| p.as_table())
        .and_then(|t| t.get("authors"))
        .and_then(|a| a.as_array())
        .and_then(|a| a.get(0)) 
        .and_then(|f| f.as_str())
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

    const SINGLE_AUTHOR_MANIFEST: &str = r#"[package]
            name = "Example"
            version = "0.1.0"
            authors = ["First Last <first.last@example.com>"]
        "#;

    const MULTIPLE_AUTHORS_MANIFEST: &str = r#"[package]
            name = "Example"
            version = "0.1.0"
            authors = ["1 Author <first.last@example.com>", "2 Author <2.author@example.com>", "3 author <3.author@example.com>"]
        "#;

    #[test]
    fn first_author_with_single_author_works() {
        let manifest = SINGLE_AUTHOR_MANIFEST.parse::<Value>().expect("Parsing TOML");
        let actual = first_author(&manifest).unwrap();
        assert_eq!(actual, String::from("First Last"));
    }

    #[test]
    fn first_author_with_multiple_authors_works() {
        let manifest = MULTIPLE_AUTHORS_MANIFEST.parse::<Value>().expect("Parsing TOML");
        let actual = first_author(&manifest).unwrap();
        assert_eq!(actual, String::from("1 Author"));
    }
}


