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
use Template;
use toml::Value;

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

    /// Sets the copyright holder in the license dialog of the Windows installer (msi).
    pub fn copyright_holder(&mut self, h: Option<&'a str>) -> &mut Self {
        self.copyright_holder = h;
        self
    }

    /// Sets the copyright year in the license dialog of the Windows installer (msi).
    pub fn copyright_year(&mut self, y: Option<&'a str>) -> &mut Self {
        self.copyright_year = y;
        self
    }

    /// Sets the path to a package's manifest (Cargo.toml) to be used to generate a WiX Source
    /// (wxs) file from the embedded template.
    ///
    /// A `wix` and `wix\main.wxs` file will be created in the same directory as the package's
    /// manifest. The default is to use the package's manifest in the current working directory.
    pub fn input(&mut self, i: Option<&'a str>) -> &mut Self {
        self.input = i;
        self
    }
    
    /// Sets the destination for creating all of the output from initialization. 
    ///
    /// The default is to create all initialization output in the current working directory.
    pub fn output(&mut self, o: Option<&'a str>) -> &mut Self {
        self.output = o;
        self
    }

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

#[derive(Debug)]
pub struct Execution {
    copyright_holder: Option<String>,
    copyright_year: Option<String>,
    input: Option<PathBuf>,
    output: Option<PathBuf>,
}

impl Execution {
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

