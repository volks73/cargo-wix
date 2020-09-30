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

//! The implementation for the `clean` command. This command is focused on
//! cleaning up build output, similar to the `cargo clean` subcommand.

use crate::Error;
use crate::Result;
use crate::CARGO_MANIFEST_FILE;
use crate::WIX;

use std::env;
use std::ffi::OsStr;
use std::fs;
use std::path::{Path, PathBuf};

/// A builder for creating an execution context to clean a package of WiX
/// Toolset-related output.
#[derive(Debug, Clone)]
pub struct Builder<'a> {
    input: Option<&'a str>,
}

impl<'a> Builder<'a> {
    /// Creates a new `Builder` instance.
    pub fn new() -> Self {
        Builder { input: None }
    }

    /// Sets the path to a package's manifest (Cargo.toml) to be cleaned.
    ///
    /// The default is to use the current working directory if a Cargo.toml file
    /// is found.
    pub fn input(&mut self, i: Option<&'a str>) -> &mut Self {
        self.input = i;
        self
    }

    /// Builds an execution context to clean the package of WiX Toolset-related
    /// output.
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

/// A context for removing WiX Toolset-related output from a package.
#[derive(Debug)]
pub struct Execution {
    input: Option<PathBuf>,
}

impl Execution {
    /// Removes WiX Toolset-related output from the package's `target` folder.
    ///
    /// This is similar to the `cargo clean` subcommand.
    pub fn run(self) -> Result<()> {
        debug!("input = {:?}", self.input);
        let manifest = super::manifest(self.input.as_ref())?;
        debug!("target_directory = {:?}", manifest.target_directory);
        let target_wix = self.target_wix(&manifest.target_directory)?;
        debug!("target_wix = {:?}", target_wix);
        if target_wix.exists() {
            trace!("The 'target\\wix' folder exists");
            warn!("Removing the 'target\\wix' folder");
            fs::remove_dir_all(target_wix)?;
        } else {
            trace!("The 'target\\wix' folder does not exist");
            info!("Nothing to clean");
        }
        Ok(())
    }

    fn target_wix(&self, target_directory: &Path) -> Result<PathBuf> {
        Ok(target_directory.join(WIX))
    }
}

impl Default for Execution {
    fn default() -> Self {
        Builder::new().build()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    mod builder {
        use super::*;

        #[test]
        fn input_works() {
            const EXPECTED: &str = "C:\\Cargo.toml";
            let mut actual = Builder::default();
            actual.input(Some(EXPECTED));
            assert_eq!(actual.input, Some(EXPECTED));
        }
    }

    mod execution {
        extern crate assert_fs;

        use super::*;
        use std::fs::File;

        #[test]
        fn target_wix_works() {
            let actual = Execution::default().target_wix(Path::new("target")).unwrap();
            let mut cwd = env::current_dir().expect("Current Working Directory");
            cwd.push("target");
            cwd.push(WIX);
            assert_eq!(actual, cwd);
        }

        #[test]
        fn target_wix_with_nonexistent_manifest_fails() {
            let result = Builder::new()
                .input(Some("C:\\Cargo.toml"))
                .build()
                .target_wix(Path::new("C:\\target"));
            assert!(result.is_err());
        }

        #[test]
        fn target_wix_with_existing_cargo_toml_works() {
            let temp_dir = assert_fs::TempDir::new().unwrap();
            let cargo_toml_path = temp_dir.path().join("Cargo.toml");
            let expected = temp_dir.path().join("target").join(WIX);
            let _non_cargo_toml_handle = File::create(&cargo_toml_path).expect("Create file");
            let actual = Builder::new()
                .input(cargo_toml_path.to_str())
                .build()
                .target_wix(&expected)
                .unwrap();
            assert_eq!(actual, expected);
        }
    }
}
