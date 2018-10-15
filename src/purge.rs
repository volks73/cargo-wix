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

use CARGO_MANIFEST_FILE;
use clean;
use Error;
use Result;
use std::env;
use std::ffi::OsStr;
use std::fs;
use std::path::PathBuf;
use WIX;

/// A builder for running the subcommand.
#[derive(Debug, Clone)]
pub struct Builder<'a> {
    input: Option<&'a str>,
}

impl<'a> Builder<'a> {
    /// Creates a new `Builder` instance.
    pub fn new() -> Self {
        Builder {
            input: None,
        }
    }

    /// Sets the path to a package's manifest (Cargo.toml) to be purge.
    ///
    /// The default is to use the current working directory if a Cargo.toml file
    /// is found. This method overrides the default.
    pub fn input(&mut self, i: Option<&'a str>) -> &mut Self {
        self.input = i;
        self
    }

    pub fn build(&mut self) -> Execution {
        Execution {
            input: self.input.map(PathBuf::from),
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
    input: Option<PathBuf>,
}

impl Execution {
    pub fn run(self) -> Result<()> {
        debug!("input = {:?}", self.input);
        let mut cleaner = clean::Builder::new();
        cleaner.input(self.input.as_ref().and_then(|p| p.to_str()));
        cleaner.build().run()?;
        let wix = self.wix()?;
        debug!("wix = {:?}", wix);
        if wix.exists() {
            trace!("The 'wix' folder exists");
            warn!("Removing the 'wix' folder");
            fs::remove_dir_all(wix)?;
        } else {
            trace!("The 'wix' folder does not exist");
            info!("Nothing to purge");
        }
        Ok(())
    }

    fn wix(&self) -> Result<PathBuf> {
        if let Some(ref input) = self.input {
            trace!("A Cargo.toml file has been explicity specified");
            if input.exists() && input.is_file() {
                trace!("The input path exists and it is a file");
                if input.file_name() == Some(OsStr::new(CARGO_MANIFEST_FILE)) {
                    trace!("The input file is a Cargo manifest file");
                    Ok(input.parent().map(|p| p.to_path_buf()).and_then(|mut p| {
                        p.push(WIX);
                        Some(p)
                    }).unwrap())
                } else {
                    Err(Error::Generic(format!(
                        "The '{}' path does not appear to be to a '{}' file", 
                        input.display(), 
                        CARGO_MANIFEST_FILE
                    )))
                }
            } else {
                Err(Error::Generic(format!(
                    "The '{}' path does not exist or it is not a file", 
                    input.display()
                )))
            }
        } else {
            trace!("An input path has NOT been explicitly specified, implicitly using the current \
                   working directory");
            let mut cwd = env::current_dir()?;
            cwd.push(WIX);
            Ok(cwd)
        }
    }
}

impl Default for Execution {
    fn default() -> Self {
        Builder::new().build()
    }
}

