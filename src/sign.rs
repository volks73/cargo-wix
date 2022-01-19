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

//! The implementation for the `sign` command. This command focuses on signing
//! installers using the Windows SDK `signtool` application.

use crate::Error;
use crate::Result;
use crate::TimestampServer;
use crate::BINARY_FOLDER_NAME;
use crate::EXE_FILE_EXTENSION;
use crate::MSI_FILE_EXTENSION;
use crate::SIGNTOOL;
use crate::SIGNTOOL_PATH_KEY;
use crate::WIX;

use log::{debug, info, trace};

use std::env;
use std::ffi::OsStr;
use std::fs;
use std::io::ErrorKind;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::str::FromStr;

use cargo_metadata::Package;

/// A builder for creating an execution context to sign an installer.
#[derive(Debug, Clone)]
pub struct Builder<'a> {
    bin_path: Option<&'a str>,
    capture_output: bool,
    description: Option<&'a str>,
    homepage: Option<&'a str>,
    input: Option<&'a str>,
    package: Option<&'a str>,
    product_name: Option<&'a str>,
    timestamp: Option<&'a str>,
}

impl<'a> Builder<'a> {
    /// Creates a new `Builder` instance.
    pub fn new() -> Self {
        Builder {
            bin_path: None,
            capture_output: true,
            description: None,
            homepage: None,
            input: None,
            package: None,
            product_name: None,
            timestamp: None,
        }
    }

    /// Sets the package on which to operate during this build
    pub fn package(&mut self, p: Option<&'a str>) -> &mut Self {
        self.package = p;
        self
    }
    /// Sets the path to the folder containing the `signtool.exe` file.
    ///
    // Normally the `signtool.exe` is installed in the `bin` folder of the
    // Windows SDK installation. The default is to use the `PATH` system
    // environment variable.
    pub fn bin_path(&mut self, b: Option<&'a str>) -> &mut Self {
        self.bin_path = b;
        self
    }

    /// Enables or disables capturing of the output from the `signtool`
    /// application.
    ///
    /// The default is to capture all output, i.e. display nothing in the
    /// console but the log statements.
    pub fn capture_output(&mut self, c: bool) -> &mut Self {
        self.capture_output = c;
        self
    }

    /// Sets the description.
    ///
    /// This override the description obtained from the `description` field in
    /// the package's manifest (Cargo.toml).
    ///
    /// The description is displayed in the ACL dialog.
    pub fn description(&mut self, d: Option<&'a str>) -> &mut Self {
        self.description = d;
        self
    }

    /// Sets the homepage URL that is displayed in the ACL dialog.
    ///
    /// The default is to use the value for the `homepage` field in the
    /// package's manifest (Cargo.toml) if it exists; otherwise, a URL
    /// is _not_ displayed in the ACL dialog.
    pub fn homepage(&mut self, h: Option<&'a str>) -> &mut Self {
        self.homepage = h;
        self
    }

    /// Sets the path to a package's manifest (Cargo.toml).
    pub fn input(&mut self, i: Option<&'a str>) -> &mut Self {
        self.input = i;
        self
    }

    /// Sets the product name.
    ///
    /// The default is to use the value for the `name` field in the package's
    /// manifest (Cargo.toml).
    pub fn product_name(&mut self, p: Option<&'a str>) -> &mut Self {
        self.product_name = p;
        self
    }

    /// Sets the URL for the timestamp server used when signing an installer.
    ///
    /// The default is to _not_ use a timestamp server, even though it is highly
    /// recommended. Use this method to enable signing with the timestamp.
    pub fn timestamp(&mut self, t: Option<&'a str>) -> &mut Self {
        self.timestamp = t;
        self
    }

    /// Creates an execution context for signing a package's installer.
    pub fn build(&mut self) -> Execution {
        Execution {
            bin_path: self.bin_path.map(PathBuf::from),
            capture_output: self.capture_output,
            description: self.description.map(String::from),
            homepage: self.homepage.map(String::from),
            input: self.input.map(PathBuf::from),
            package: self.package.map(String::from),
            product_name: self.product_name.map(String::from),
            timestamp: self.timestamp.map(String::from),
        }
    }
}

impl<'a> Default for Builder<'a> {
    fn default() -> Self {
        Builder::new()
    }
}

/// A context for signing a package's installer.
#[derive(Debug)]
pub struct Execution {
    bin_path: Option<PathBuf>,
    capture_output: bool,
    description: Option<String>,
    homepage: Option<String>,
    input: Option<PathBuf>,
    package: Option<String>,
    product_name: Option<String>,
    timestamp: Option<String>,
}

impl Execution {
    /// Signs a package's installer.
    pub fn run(self) -> Result<()> {
        info!("Signing the installer");
        debug!("bin_path = {:?}", self.bin_path);
        debug!("capture_output = {:?}", self.capture_output);
        debug!("description = {:?}", self.description);
        debug!("homepage = {:?}", self.homepage);
        debug!("input = {:?}", self.input);
        debug!("package = {:?}", self.package);
        debug!("product_name = {:?}", self.product_name);
        debug!("timestamp = {:?}", self.timestamp);
        let manifest = super::manifest(self.input.as_ref())?;
        debug!("target_directory = {:?}", manifest.target_directory);
        let package = super::package(&manifest, self.package.as_deref())?;
        let product_name = super::product_name(self.product_name.as_ref(), &package);
        let description = if let Some(d) = super::description(self.description.clone(), &package) {
            trace!("A description was provided either at the command line or in the package's manifest (Cargo.toml).");
            format!("{} - {}", product_name, d)
        } else {
            trace!("A description was not provided at the command line or in the package's manifest (Cargo.toml).");
            product_name
        };
        debug!("description = {:?}", description);
        let msi = self.msi(&manifest.target_directory)?;
        let mut signer = self.signer()?;
        debug!("signer = {:?}", signer);
        if self.capture_output {
            trace!("Capturing the {} output", SIGNTOOL);
            signer.stdout(Stdio::null());
            signer.stderr(Stdio::null());
        }
        signer.arg("sign").arg("/a").arg("/fd").arg("certHash").arg("/d").arg(description);
        if let Some(h) = self.homepage(&package) {
            trace!("Using the '{}' URL for the expanded description", h);
            signer.arg("/du").arg(h);
        }
        if let Some(t) = self.timestamp {
            let server = TimestampServer::from_str(&t)?;
            trace!(
                "Using the '{}' timestamp server to sign the installer",
                server
            );
            signer.arg("/t");
            signer.arg(server.url());
        }
        let status = signer.arg(&msi).status().map_err(|err| {
            if err.kind() == ErrorKind::NotFound {
                Error::Generic(format!(
                    "The {0} application could not be found. Please check the Windows 10 SDK \
                     (https://developer.microsoft.com/en-us/windows/downloads/windows-10-sdk) is \
                     installed and you are using the x64 or x86 Native Build Tools prompt so the \
                     {0} application is available.",
                    SIGNTOOL
                ))
            } else {
                err.into()
            }
        })?;
        if !status.success() {
            return Err(Error::Command(
                SIGNTOOL,
                status.code().unwrap_or(100),
                self.capture_output,
            ));
        }
        Ok(())
    }

    fn homepage(&self, manifest: &Package) -> Option<String> {
        self.homepage
            .as_ref()
            .map(String::from)
            .or_else(|| manifest.homepage.clone())
    }

    fn msi(&self, target_directory: &Path) -> Result<PathBuf> {
        if let Some(ref i) = self.input {
            trace!("The path to an installer to sign has been explicitly set");
            let msi = PathBuf::from(i);
            if msi.exists() {
                trace!("The installer exists");
                Ok(msi)
            } else {
                Err(Error::Generic(format!(
                    "The '{}' path does not exist for the installer",
                    msi.display()
                )))
            }
        } else {
            trace!("The path to an installer has not been explicitly set");
            let cwd = target_directory.join(WIX);
            for entry in fs::read_dir(cwd)? {
                let entry = entry?;
                let path = entry.path();
                if path.extension() == Some(OsStr::new(MSI_FILE_EXTENSION)) {
                    return Ok(path);
                }
            }
            Err(Error::Generic(format!(
                "Could not find an installer ({}) to sign",
                MSI_FILE_EXTENSION
            )))
        }
    }

    fn signer(&self) -> Result<Command> {
        if let Some(mut path) = self.bin_path.as_ref().map(|s| {
            let mut p = PathBuf::from(s);
            trace!(
                "Using the '{}' path to the Windows SDK '{}' folder for the signer",
                p.display(),
                BINARY_FOLDER_NAME
            );
            p.push(SIGNTOOL);
            p.set_extension(EXE_FILE_EXTENSION);
            p
        }) {
            if !path.exists() {
                path.pop(); // Remove the 'signtool' application from the path
                Err(Error::Generic(format!(
                    "The signer application ('{}') does not exist at the '{}' path specified via \
                    the '-S, --sign-path' command line argument. Please check the path is correct and \
                    the signer application exists at the path.",
                    SIGNTOOL,
                    path.display()
                )))
            } else {
                Ok(Command::new(path))
            }
        } else if let Some(mut path) = env::var_os(SIGNTOOL_PATH_KEY).map(|s| {
            let mut p = PathBuf::from(s);
            trace!(
                "Using the '{}' path to the Windows SDK '{}' folder for the signer",
                p.display(),
                BINARY_FOLDER_NAME
            );
            p.push(SIGNTOOL);
            p.set_extension(EXE_FILE_EXTENSION);
            p
        }) {
            if !path.exists() {
                path.pop(); // Remove the `signtool` application from the path
                Err(Error::Generic(format!(
                    "The signer application ('{}') does not exist at the '{}' path specified \
                     via the {} environment variable. Please check the path is correct and the \
                     signer application exists at the path.",
                    SIGNTOOL,
                    path.display(),
                    SIGNTOOL_PATH_KEY
                )))
            } else {
                Ok(Command::new(path))
            }
        } else {
            Ok(Command::new(SIGNTOOL))
        }
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
        fn bin_path_works() {
            const EXPECTED: &str = "C:\\signtool.exe";
            let mut actual = Builder::new();
            actual.bin_path(Some(EXPECTED));
            assert_eq!(actual.bin_path, Some(EXPECTED));
        }

        #[test]
        fn capture_output_works() {
            let mut actual = Builder::new();
            actual.capture_output(false);
            assert!(!actual.capture_output);
        }

        #[test]
        fn description_works() {
            const EXPECTED: &str = "This is a description";
            let mut actual = Builder::new();
            actual.description(Some(EXPECTED));
            assert_eq!(actual.description, Some(EXPECTED));
        }

        #[test]
        fn homepage_works() {
            const EXPECTED: &str = "http://www.example.com";
            let mut actual = Builder::new();
            actual.homepage(Some(EXPECTED));
            assert_eq!(actual.homepage, Some(EXPECTED));
        }

        #[test]
        fn input_works() {
            const EXPECTED: &str = "C:\\Example";
            let mut actual = Builder::new();
            actual.input(Some(EXPECTED));
            assert_eq!(actual.input, Some(EXPECTED));
        }

        #[test]
        fn product_name_works() {
            const EXPECTED: &str = "Example";
            let mut actual = Builder::new();
            actual.product_name(Some(EXPECTED));
            assert_eq!(actual.product_name, Some(EXPECTED));
        }

        #[test]
        fn timestamp_works() {
            const EXPECTED: &str = "http://www.example.com";
            let mut actual = Builder::new();
            actual.timestamp(Some(EXPECTED));
            assert_eq!(actual.timestamp, Some(EXPECTED));
        }
    }

    mod execution {
        extern crate assert_fs;

        use std::fs::File;

        use super::*;
        use crate::tests::setup_project;

        const MIN_MANIFEST: &str = r#"[package]
            name = "Example"
            version = "0.1.0"
            authors = ["First Last <first.last@example.com>"]
        "#;

        const HOMEPAGE_MANIFEST: &str = r#"[package]
            name = "Example"
            version = "0.1.0"
            authors = ["First Last <first.last@example.com>"]
            homepage = "http://www.example.com"
        "#;

        #[test]
        fn homepage_without_homepage_field_works() {
            let project = setup_project(MIN_MANIFEST);
            let manifest = crate::manifest(Some(&project.path().join("Cargo.toml"))).unwrap();
            let package = crate::package(&manifest, None).unwrap();

            let actual = Execution::default().homepage(&package);
            assert!(actual.is_none());
        }

        #[test]
        fn homepage_with_homepage_field_works() {
            let project = setup_project(HOMEPAGE_MANIFEST);
            let manifest = crate::manifest(Some(&project.path().join("Cargo.toml"))).unwrap();
            let package = crate::package(&manifest, None).unwrap();

            let actual = Execution::default().homepage(&package);
            assert_eq!(actual, Some(String::from("http://www.example.com")));
        }

        #[test]
        fn homepage_with_override_works() {
            const EXPECTED: &str = "http://www.another.com";

            let project = setup_project(HOMEPAGE_MANIFEST);
            let manifest = crate::manifest(Some(&project.path().join("Cargo.toml"))).unwrap();
            let package = crate::package(&manifest, None).unwrap();

            let actual = Builder::new()
                .homepage(Some(EXPECTED))
                .build()
                .homepage(&package);
            assert_eq!(actual, Some(String::from(EXPECTED)));
        }

        #[test]
        fn msi_with_nonexistent_installer_fails() {
            let result = Execution::default().msi(Path::new("target"));
            assert!(result.is_err());
        }

        #[test]
        fn msi_with_existing_file_works() {
            let temp_dir = assert_fs::TempDir::new().unwrap();
            let msi_path = temp_dir.path().join("Example.msi");
            let _msi_handle = File::create(&msi_path).expect("Create file");
            let actual = Builder::new()
                .input(msi_path.to_str())
                .build()
                .msi(Path::new("target"))
                .unwrap();
            assert_eq!(actual, msi_path);
        }

        #[test]
        fn signer_works() {
            let result = Execution::default().signer();
            assert!(result.is_ok());
        }

        #[test]
        fn signer_with_nonexisting_path_fails() {
            let result = Builder::new()
                .bin_path(Some("Example.exe"))
                .build()
                .signer();
            assert!(result.is_err());
        }

        #[test]
        fn signer_with_nonexistent_environment_path_fails() {
            env::set_var(SIGNTOOL_PATH_KEY, "Example");
            let result = Execution::default().signer();
            env::remove_var(SIGNTOOL_PATH_KEY);
            assert!(result.is_err());
        }
    }
}
