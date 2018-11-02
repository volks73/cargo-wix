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

use chrono::{Datelike, Utc};
use Error;
use manifest;
use mustache::{self, MapBuilder};
use Result;
use std::path::PathBuf;
use super::Template;
use toml::Value;

/// A builder for creating an execution context to print a license.
#[derive(Debug, Clone)]
pub struct Builder<'a> {
    copyright_year: Option<&'a str>,
    copyright_holder: Option<&'a str>,
    input: Option<&'a str>,
    output: Option<&'a str>,
}

impl<'a> Builder<'a> {
    /// Creates a new `Builder` instance.
    pub fn new() -> Self {
        Builder {
            copyright_year: None,
            copyright_holder: None,
            input: None,
            output: None,
        }
    }

    /// Sets the copyright holder for the generated license.
    ///
    /// If the license template does not use a copyright holder, then this value
    /// is ignored.
    ///
    /// The default is to use the first author from the `authors` field of the
    /// package's manifest (Cargo.toml).
    pub fn copyright_holder(&mut self, h: Option<&'a str>) -> &mut Self {
        self.copyright_holder = h;
        self
    }

    /// Sets the copyright year for the generated license.
    ///
    /// If the license template does not use a copyright year, then this value
    /// is ignored.
    ///
    /// The default is to use this year when generating the license.
    pub fn copyright_year(&mut self, y: Option<&'a str>) -> &mut Self {
        self.copyright_year = y;
        self
    }

    /// Sets the path to a package's manifest (Cargo.toml) to be used to
    /// generate license in the Rich Text Format (RTF).
    ///
    /// By default, the license will be printed to `STDOUT` unless the
    /// [`output`] method is used.
    ///
    /// [`output`]: #output
    pub fn input(&mut self, i: Option<&'a str>) -> &mut Self {
        self.input = i;
        self
    }

    /// Sets the destination.
    ///
    /// The default is to print all output to `STDOUT`. This method can be used
    /// to specify that the generated license be written, or "printed", to a
    /// file instead of `STDOUT`.
    pub fn output(&mut self, o: Option<&'a str>) -> &mut Self {
        self.output = o;
        self
    }

    /// Builds an execution context based on the configuration.
    pub fn build(&self) -> Execution {
        Execution {
            copyright_holder: self.copyright_holder.map(String::from),
            copyright_year: self.copyright_year.map(String::from),
            input: self.input.map(PathBuf::from),
            output: self.output.map(PathBuf::from),
        }
    }
}

impl<'a> Default for Builder<'a> {
    fn default() -> Self {
        Builder::new()
    }
}

/// A context for printing a license.
#[derive(Debug)]
pub struct Execution {
    copyright_holder: Option<String>,
    copyright_year: Option<String>,
    input: Option<PathBuf>,
    output: Option<PathBuf>,
}

impl Execution {
    /// Prints a license based on the built context.
    pub fn run(self, template: Template) -> Result<()> {
        debug!("copyright_holder = {:?}", self.copyright_holder);
        debug!("copyright_year = {:?}", self.copyright_year);
        debug!("input = {:?}", self.input);
        debug!("output = {:?}", self.output);
        let manifest = manifest(self.input.as_ref())?;
        let mut destination = super::destination(self.output.as_ref())?;
        let template = mustache::compile_str(template.to_str())?;
        let data = MapBuilder::new()
            .insert_str("copyright-year", self.copyright_year())
            .insert_str("copyright-holder", self.copyright_holder(&manifest)?)
            .build();
        template.render_data(&mut destination, &data).map_err(Error::from)
    }

    fn copyright_holder(&self, manifest: &Value) -> Result<String> {
        if let Some(ref h) = self.copyright_holder {
            Ok(h.to_owned())
        } else {
            super::first_author(&manifest)
        }
    }

    fn copyright_year(&self) -> String {
        self.copyright_year.clone()
            .map(String::from)
            .unwrap_or(Utc::now().year().to_string())
    }
}

impl Default for Execution {
    fn default() -> Self {
        Builder::new().build()
    }
}

#[cfg(test)]
mod tests{
    use super::*;

    mod builder {
        use super::*;

        #[test]
        fn copyright_holder_works() {
            const EXPECTED: &str = "Example";
            let mut actual = Builder::new();
            actual.copyright_holder(Some(EXPECTED));
            assert_eq!(actual.copyright_holder, Some(EXPECTED));
        }

        #[test]
        fn copyright_year_works() {
            const EXPECTED: &str = "1982";
            let mut actual = Builder::new();
            actual.copyright_year(Some(EXPECTED));
            assert_eq!(actual.copyright_year, Some(EXPECTED));
        }

        #[test]
        fn input_works() {
            const EXPECTED: &str = "Example.wxs";
            let mut actual = Builder::new();
            actual.input(Some(EXPECTED));
            assert_eq!(actual.input, Some(EXPECTED));
        }

        #[test]
        fn output_works() {
            const EXPECTED: &str = "C:\\Example\\output";
            let mut actual = Builder::new();
            actual.output(Some(EXPECTED));
            assert_eq!(actual.output, Some(EXPECTED));
        }
    }

    mod execution {
        use super::*;

        const MIN_MANIFEST: &str = r#"[package]
            name = "Example"
            version = "0.1.0"
            authors = ["First Last <first.last@example.com>"]
        "#;

        #[test]
        fn copyright_holder_works() {
            let manifest = MIN_MANIFEST.parse::<Value>().expect("Parsing TOML");
            let actual = Execution::default().copyright_holder(&manifest).unwrap();
            assert_eq!(actual, String::from("First Last"));
        }

        #[test]
        fn copyright_holder_with_override_works() {
            const EXPECTED: &str = "Dr. Example";
            let manifest = MIN_MANIFEST.parse::<Value>().expect("Parsing TOML");
            let actual = Builder::new()
                .copyright_holder(Some(EXPECTED))
                .build()
                .copyright_holder(&manifest)
                .unwrap();
            assert_eq!(actual, String::from(EXPECTED));
        }

        #[test]
        fn copyright_year_works() {
            let actual = Execution::default().copyright_year();
            assert_eq!(actual, Utc::now().year().to_string());
        }

        #[test]
        fn copyright_year_with_override_works() {
            const EXPECTED: &str = "1982";
            let actual = Builder::new()
                .copyright_year(Some(EXPECTED))
                .build()
                .copyright_year();
            assert_eq!(actual, String::from(EXPECTED));
        }
    }
}

