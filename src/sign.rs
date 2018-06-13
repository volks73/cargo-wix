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

use BINARY_FOLDER_NAME;
use Error;
use EXE_FILE_EXTENSION;
use MSI_FILE_EXTENSION;
use Result;
use SIGNTOOL;
use SIGNTOOL_PATH_KEY;
use std::env;
use std::ffi::OsStr;
use std::fs;
use std::io::ErrorKind;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::str::FromStr;
use TARGET_FOLDER_NAME;
use TimestampServer;
use toml::Value;
use WIX;

/// A builder for running the subcommand.
#[derive(Debug, Clone)]
pub struct Builder<'a> {
    bin_path: Option<&'a str>,
    capture_output: bool,
    description: Option<&'a str>,
    homepage: Option<&'a str>,
    input: Option<&'a str>,
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
            product_name: None,
            timestamp: None,
        }
    }

    /// Sets the path to the folder containing the `signtool.exe` file.
    ///
    /// Normally the `signtool.exe` is installed in the `bin` folder of the Windows SDK
    /// installation. THe default is to use the PATH system environment variable. This will
    /// override any value obtained from the environment.
    pub fn bin_path(&mut self, b: Option<&'a str>) -> &mut Self {
        self.bin_path = b;
        self
    }

    /// Enables or disables capturing of the output from the builder (`cargo`), compiler
    /// (`candle`), linker (`light`), and signer (`signtool`).
    ///
    /// The default is to capture all output, i.e. display nothing in the console but the log
    /// statements.
    pub fn capture_output(&mut self, c: bool) -> &mut Self {
        self.capture_output = c;
        self
    }

    /// Sets the description.
    ///
    /// This override the description determined from the `description` field in the package's
    /// manifest (Cargo.toml).
    pub fn description(&mut self, d: Option<&'a str>) -> &mut Self {
        self.description = d;
        self
    }
    
    /// Sets the homepage URL that is displayed in the ACL dialog.
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
    /// This override the product name determined from the `name` field in the package's
    /// manifest (Cargo.toml).
    pub fn product_name(&mut self, p: Option<&'a str>) -> &mut Self {
        self.product_name = p;
        self
    }

    /// Sets the URL for the timestamp server used when signing an installer.
    ///
    /// The default is to _not_ use a timestamp server, even though it is highly recommended. Use
    /// this method to enable signing with the timestamp.
    pub fn timestamp(&mut self, t: Option<&'a str>) -> &mut Self {
        self.timestamp = t;
        self
    }

    pub fn build(&mut self) -> Execution {
        Execution {
            bin_path: self.bin_path.map(PathBuf::from),
            capture_output: self.capture_output,
            description: self.description.map(String::from),
            homepage: self.homepage.map(String::from),
            input: self.input.map(PathBuf::from),
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

#[derive(Debug)]
pub struct Execution {
    bin_path: Option<PathBuf>,
    capture_output: bool,
    description: Option<String>,
    homepage: Option<String>,
    input: Option<PathBuf>,
    product_name: Option<String>,
    timestamp: Option<String>,
}

impl Execution {
    pub fn run(self) -> Result<()> {
        info!("Signing the installer");
        debug!("bin_path = {:?}", self.bin_path);
        debug!("capture_output = {:?}", self.capture_output);
        debug!("description = {:?}", self.description);
        debug!("homepage = {:?}", self.homepage);
        debug!("input = {:?}", self.input);
        debug!("product_name = {:?}", self.product_name);
        debug!("timestamp = {:?}", self.timestamp);
        let manifest = super::manifest(self.input.as_ref())?;
        let product_name = super::product_name(self.product_name.as_ref(), &manifest)?;
        let description = if let Some(d) = super::description(self.description.clone(), &manifest) {
            trace!("A description was provided either at the command line or in the package's manifest (Cargo.toml).");
            format!("{} - {}", product_name, d)
        } else {
            trace!("A description was not provided at the command line or in the package's manifest (Cargo.toml).");
            product_name
        };
        debug!("description = {:?}", description);
        let msi = self.msi()?;
        let mut signer = self.signer()?;
        debug!("signer = {:?}", signer);
        if self.capture_output {
            trace!("Capturing the {} output", SIGNTOOL);
            signer.stdout(Stdio::null());
            signer.stderr(Stdio::null());
        }
        signer.arg("sign")
            .arg("/a")
            .arg("/d")
            .arg(description);
        if let Some(h) = self.homepage(&manifest) {
            trace!("Using the '{}' URL for the expanded description", h);
            signer.arg("/du").arg(h);
        }
        if let Some(t) = self.timestamp {
            let server = TimestampServer::from_str(&t)?;
            trace!("Using the '{}' timestamp server to sign the installer", server); 
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
            return Err(Error::Command(SIGNTOOL, status.code().unwrap_or(100)));
        }    
        Ok(())
    }

    fn homepage(&self, manifest: &Value) -> Option<String> {
        self.homepage.clone().or(manifest.get("package")
            .and_then(|p| p.as_table())
            .and_then(|t| t.get("homepage"))
            .and_then(|d| d.as_str())
            .map(|s| String::from(s))
        )
    }

    fn msi(&self) -> Result<PathBuf> {
        if let Some(ref i) = self.input {
            trace!("The path to an installer to sign has been explicitly set");
            let mut msi = PathBuf::from(i);
            if msi.exists() {
                trace!("The installer exists");
                Ok(msi)
            } else {
                Err(Error::Generic(format!(
                    "The '{}' path does not exist for the installer", msi.display()
                )))
            }
        } else {
            trace!("The path to an installer has not been explicitly set");
            let mut cwd = env::current_dir()?;
            cwd.push(TARGET_FOLDER_NAME);
            cwd.push(WIX);
            for entry in fs::read_dir(cwd)? {
                let entry = entry?;
                let path = entry.path();
                if path.extension() == Some(OsStr::new(MSI_FILE_EXTENSION)) {
                    return Ok(path);
                }
            }
            Err(Error::Generic(format!(
                "Could not find an installer ({}) to sign", MSI_FILE_EXTENSION
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
        } else {
            if let Some(mut path) = env::var_os(SIGNTOOL_PATH_KEY).map(|s| {
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
}

impl Default for Execution {
    fn default() -> Self {
        Builder::new().build()
    }
}
