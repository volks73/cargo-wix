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

//! The implementation for the `create`, or default, command. The default
//! command, `cargo wix`, is focused on creating, or building, the installer
//! using the WiX Toolset.
//!
//! Generally, this involves locating the WiX Source file (wxs) and passing
//! options and flags to the WiX Toolset's compiler (`candle.exe`) and linker
//! (`light.exe`). By default, it looks for a `wix\main.wxs` file relative to
//! the root of the package's manifest (Cargo.toml). A different WiX Source file
//! can be set with the `input` method using the `Builder` struct.

use crate::Cultures;
use crate::Error;
use crate::Platform;
use crate::Result;
use crate::BINARY_FOLDER_NAME;
use crate::CARGO;
use crate::EXE_FILE_EXTENSION;
use crate::MSI_FILE_EXTENSION;
use crate::TARGET_FOLDER_NAME;
use crate::WIX;
use crate::WIX_COMPILER;
use crate::WIX_LINKER;
use crate::WIX_OBJECT_FILE_EXTENSION;
use crate::WIX_PATH_KEY;
use crate::WIX_SOURCE_FILE_EXTENSION;

use semver::Version;

use std::env;
use std::io::ErrorKind;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::str::FromStr;

use toml::Value;

/// A builder for running the `cargo wix` subcommand.
#[derive(Debug, Clone)]
pub struct Builder<'a> {
    bin_path: Option<&'a str>,
    capture_output: bool,
    culture: Option<&'a str>,
    includes: Option<Vec<&'a str>>,
    input: Option<&'a str>,
    locale: Option<&'a str>,
    name: Option<&'a str>,
    no_build: bool,
    output: Option<&'a str>,
    version: Option<&'a str>,
}

impl<'a> Builder<'a> {
    /// Creates a new `Builder` instance.
    pub fn new() -> Self {
        Builder {
            bin_path: None,
            capture_output: true,
            culture: None,
            includes: None,
            input: None,
            locale: None,
            name: None,
            no_build: false,
            output: None,
            version: None,
        }
    }

    /// Sets the path to the WiX Toolset's `bin` folder.
    ///
    /// The WiX Toolset's `bin` folder should contain the needed `candle.exe`
    /// and `light.exe` applications. The default is to use the WIX system
    /// environment variable that is created during installation of the WiX
    /// Toolset. This will override any value obtained from the environment.
    pub fn bin_path(&mut self, b: Option<&'a str>) -> &mut Self {
        self.bin_path = b;
        self
    }

    /// Enables or disables capturing of the output from the builder (`cargo`),
    /// compiler (`candle`), linker (`light`), and signer (`signtool`).
    ///
    /// The default is to capture all output, i.e. display nothing in the
    /// console but the log statements.
    pub fn capture_output(&mut self, c: bool) -> &mut Self {
        self.capture_output = c;
        self
    }

    /// Sets the culture to use with the linker (light.exe) for building a
    /// localized installer.
    ///
    /// This value will override any defaults and skip looking for a value in
    /// the `[package.metadata.wix]` section of the package's manifest
    /// (Cargo.toml).
    pub fn culture(&mut self, c: Option<&'a str>) -> &mut Self {
        self.culture = c;
        self
    }

    /// Adds multiple WiX Source (wxs) files to the creation of an installer.
    ///
    /// By default, any `.wxs` file located in the project's `wix` folder will
    /// be included in the creation of an installer for the project. This method
    /// adds, or appends, to the list of `.wxs` files. The value is a relative
    /// or absolute path.
    ///
    /// This value will override any default and skip looking for a value in the
    /// `[package.metadata.wix]` section of the package's manifest (Cargo.toml).
    pub fn includes(&mut self, i: Option<Vec<&'a str>>) -> &mut Self {
        self.includes = i;
        self
    }

    /// Sets the path to a package's manifest (Cargo.toml) file.
    ///
    /// A package's manifest is used to create an installer. If no path is
    /// specified, then the current working directory (CWD) is used. An error
    /// will occur if there is no `Cargo.toml` file in the CWD or at the
    /// specified path. Either an absolute or relative path is valid.
    ///
    /// This value will override any default and skip looking for a value in the
    /// `[package.metadata.wix]` section of the package's manifest (Cargo.toml).
    pub fn input(&mut self, i: Option<&'a str>) -> &mut Self {
        self.input = i;
        self
    }

    /// Sets the path to a WiX localization file, `.wxl`, for the linker
    /// (light.exe).
    ///
    /// The [WiX localization file] is an XML file that contains localization
    /// strings.
    ///
    /// This value will override any default and skip looking for a value in the
    /// `[package.metadata.wix]` section of the package's manifest (Cargo.toml).
    ///
    /// [WiX localization file]: http://wixtoolset.org/documentation/manual/v3/howtos/ui_and_localization/make_installer_localizable.html
    pub fn locale(&mut self, l: Option<&'a str>) -> &mut Self {
        self.locale = l;
        self
    }

    /// Sets the name.
    ///
    /// The default is to use the `name` field under the `[package]` section of
    /// the package's manifest (Cargo.toml). This overrides that value.
    ///
    /// The installer (msi) that is created will be named in the following
    /// format: "name-major.minor.patch-platform.msi", where _name_ is the value
    /// specified with this method or the value from the `name` field under the
    /// `[package]` section, the _major.minor.patch_ is the version number from
    /// the package's manifest `version` field or the value specified at the
    /// command line, and the _platform_ is either "i686" or "x86_64" depending
    /// on the build environment.
    ///
    /// This does __not__ change the name of the executable that is installed.
    /// The name of the executable can be changed by modifying the WiX Source
    /// (wxs) file with a text editor.
    ///
    /// This value will override any default and skip looking for a value in the
    /// `[package.metadata.wix]` section of the package's manifest (Cargo.toml).
    pub fn name(&mut self, p: Option<&'a str>) -> &mut Self {
        self.name = p;
        self
    }

    /// Skips the building of the project with the release profile.
    ///
    /// If `true`, the project will _not_ be built using the release profile,
    /// i.e. the `cargo build --release` command will not be executed. The
    /// default is to build the project before each creation. This is useful if
    /// building the project is more involved or is handled in a separate
    /// process.
    ///
    /// This value will override any default and skip looking for a value in the
    /// `[package.metadata.wix]` section of the package's manifest (Cargo.toml).
    pub fn no_build(&mut self, n: bool) -> &mut Self {
        self.no_build = n;
        self
    }

    /// Sets the output file and destination.
    ///
    /// The default is to create a MSI file with the
    /// `<product-name>-<version>-<arch>.msi` file name and extension in the
    /// `target\wix` folder. Use this method to override the destination and
    /// file name of the Windows installer.
    ///
    /// If the path is to an existing folder or contains a trailing slash
    /// (forward or backward), then the default MSI file name is used, but the
    /// installer will be available at the specified path. When specifying a
    /// file name and path, the `.msi` file is not required. It will be added
    /// automatically.
    ///
    /// This value will override any default and skip looking for a value in the
    /// `[package.metadata.wix]` section of the package's manifest (Cargo.toml).
    pub fn output(&mut self, o: Option<&'a str>) -> &mut Self {
        self.output = o;
        self
    }

    /// Sets the version.
    ///
    /// This overrides the `version` field of the package's manifest
    /// (Cargo.toml). The version should be in the "Major.Minor.Patch" notation.
    ///
    /// This value will override any default and skip looking for a value in the
    /// `[package.metadata.wix]` section of the package's manifest (Cargo.toml).
    pub fn version(&mut self, v: Option<&'a str>) -> &mut Self {
        self.version = v;
        self
    }

    /// Builds a context for creating, or building, the installer.
    pub fn build(&mut self) -> Execution {
        Execution {
            bin_path: self.bin_path.map(PathBuf::from),
            capture_output: self.capture_output,
            culture: self.culture.map(String::from),
            includes: self
                .includes
                .as_ref()
                .map(|v| v.iter().map(&PathBuf::from).collect()),
            input: self.input.map(PathBuf::from),
            locale: self.locale.map(PathBuf::from),
            name: self.name.map(String::from),
            no_build: self.no_build,
            output: self.output.map(String::from),
            version: self.version.map(String::from),
        }
    }
}

impl<'a> Default for Builder<'a> {
    fn default() -> Self {
        Builder::new()
    }
}

/// A context for creating, or building, an installer.
#[derive(Debug)]
pub struct Execution {
    bin_path: Option<PathBuf>,
    capture_output: bool,
    culture: Option<String>,
    includes: Option<Vec<PathBuf>>,
    input: Option<PathBuf>,
    locale: Option<PathBuf>,
    name: Option<String>,
    no_build: bool,
    output: Option<String>,
    version: Option<String>,
}

impl Execution {
    /// Creates, or builds, an installer within a built context.
    #[allow(clippy::cognitive_complexity)]
    pub fn run(self) -> Result<()> {
        debug!("bin_path = {:?}", self.bin_path);
        debug!("capture_output = {:?}", self.capture_output);
        debug!("culture = {:?}", self.culture);
        debug!("includes = {:?}", self.includes);
        debug!("input = {:?}", self.input);
        debug!("locale = {:?}", self.locale);
        debug!("name = {:?}", self.name);
        debug!("no_build = {:?}", self.no_build);
        debug!("output = {:?}", self.output);
        debug!("version = {:?}", self.version);
        let manifest = super::manifest(self.input.as_ref())?;
        let name = self.name(&manifest)?;
        debug!("name = {:?}", name);
        let version = self.version(&manifest)?;
        debug!("version = {:?}", version);
        let culture = self.culture(&manifest)?;
        debug!("culture = {:?}", culture);
        let locale = self.locale(&manifest)?;
        debug!("locale = {:?}", locale);
        let platform = self.platform();
        debug!("platform = {:?}", platform);
        let wxs_sources = self.wxs_sources(&manifest)?;
        debug!("wxs_sources = {:?}", wxs_sources);
        let wixobj_destination = self.wixobj_destination()?;
        debug!("wixobj_destination = {:?}", wixobj_destination);
        let msi_destination = self.msi_destination(&name, &version, platform, &manifest)?;
        debug!("msi_destination = {:?}", msi_destination);
        if self.no_build(&manifest) {
            warn!("Skipped building the release binary");
        } else {
            // Build the binary with the release profile. If a release binary
            // has already been built, then this will essentially do nothing.
            info!("Building the release binary");
            let mut builder = Command::new(CARGO);
            debug!("builder = {:?}", builder);
            if self.capture_output {
                trace!("Capturing the '{}' output", CARGO);
                builder.stdout(Stdio::null());
                builder.stderr(Stdio::null());
            }
            let status = builder.arg("build").arg("--release").status()?;
            if !status.success() {
                return Err(Error::Command(
                    CARGO,
                    status.code().unwrap_or(100),
                    self.capture_output,
                ));
            }
        }
        // Compile the installer
        info!("Compiling the installer");
        let mut compiler = self.compiler()?;
        debug!("compiler = {:?}", compiler);
        if self.capture_output {
            trace!("Capturing the '{}' output", WIX_COMPILER);
            compiler.stdout(Stdio::null());
            compiler.stderr(Stdio::null());
        }
        compiler
            .arg(format!("-dVersion={}", version))
            .arg(format!("-dPlatform={}", platform))
            .arg("-ext")
            .arg("WixUtilExtension")
            .arg("-o")
            .arg(&wixobj_destination)
            .args(&wxs_sources);
        debug!("command = {:?}", compiler);
        let status = compiler.status().map_err(|err| {
            if err.kind() == ErrorKind::NotFound {
                Error::Generic(format!(
                    "The compiler application ({}) could not be found in the PATH environment \
                    variable. Please check the WiX Toolset (http://wixtoolset.org/) is \
                    installed and check the WiX Toolset's '{}' folder has been added to the PATH \
                    system environment variable, the {} system environment variable exists, or use \
                    the '-b,--bin-path' command line argument.",
                    WIX_COMPILER, BINARY_FOLDER_NAME, WIX_PATH_KEY
                ))
            } else {
                err.into()
            }
        })?;
        if !status.success() {
            return Err(Error::Command(
                WIX_COMPILER,
                status.code().unwrap_or(100),
                self.capture_output,
            ));
        }
        // Link the installer
        info!("Linking the installer");
        let mut linker = self.linker()?;
        debug!("linker = {:?}", linker);
        let wixobj_sources = self.wixobj_sources(&wixobj_destination)?;
        debug!("wixobj_sources = {:?}", wixobj_sources);
        if self.capture_output {
            trace!("Capturing the '{}' output", WIX_LINKER);
            linker.stdout(Stdio::null());
            linker.stderr(Stdio::null());
        }
        if let Some(l) = locale {
            trace!("Using the a WiX localization file");
            linker.arg("-loc").arg(l);
        }
        linker
            .arg("-spdb")
            .arg("-ext")
            .arg("WixUIExtension")
            .arg("-ext")
            .arg("WixUtilExtension")
            .arg(format!("-cultures:{}", culture))
            .arg("-out")
            .arg(&msi_destination)
            .args(&wixobj_sources);
        debug!("command = {:?}", linker);
        let status = linker.status().map_err(|err| {
            if err.kind() == ErrorKind::NotFound {
                Error::Generic(format!(
                    "The linker application ({}) could not be found in the PATH environment \
                     variable. Please check the WiX Toolset (http://wixtoolset.org/) is \
                     installed and check the WiX Toolset's '{}' folder has been added to the PATH \
                     environment variable, the {} system environment variable exists, or use the \
                     '-b,--bin-path' command line argument.",
                    WIX_LINKER, BINARY_FOLDER_NAME, WIX_PATH_KEY
                ))
            } else {
                err.into()
            }
        })?;
        if !status.success() {
            return Err(Error::Command(
                WIX_LINKER,
                status.code().unwrap_or(100),
                self.capture_output,
            ));
        }
        Ok(())
    }

    fn compiler(&self) -> Result<Command> {
        if let Some(mut path) = self.bin_path.as_ref().map(|s| {
            let mut p = PathBuf::from(s);
            trace!(
                "Using the '{}' path to the WiX Toolset's '{}' folder for the compiler",
                p.display(),
                BINARY_FOLDER_NAME
            );
            p.push(WIX_COMPILER);
            p.set_extension(EXE_FILE_EXTENSION);
            p
        }) {
            if !path.exists() {
                path.pop(); // Remove the `candle` application from the path
                Err(Error::Generic(format!(
                    "The compiler application ('{}') does not exist at the '{}' path specified via \
                    the '-b,--bin-path' command line argument. Please check the path is correct and \
                    the compiler application exists at the path.",
                    WIX_COMPILER,
                    path.display()
                )))
            } else {
                Ok(Command::new(path))
            }
        } else if let Some(mut path) = env::var_os(WIX_PATH_KEY).map(|s| {
            let mut p = PathBuf::from(s);
            trace!(
                "Using the '{}' path to the WiX Toolset's '{}' folder for the compiler",
                p.display(),
                BINARY_FOLDER_NAME
            );
            p.push(BINARY_FOLDER_NAME);
            p.push(WIX_COMPILER);
            p.set_extension(EXE_FILE_EXTENSION);
            p
        }) {
            if !path.exists() {
                path.pop(); // Remove the `candle` application from the path
                Err(Error::Generic(format!(
                    "The compiler application ('{}') does not exist at the '{}' path specified \
                     via the {} environment variable. Please check the path is correct and the \
                     compiler application exists at the path.",
                    WIX_COMPILER,
                    path.display(),
                    WIX_PATH_KEY
                )))
            } else {
                Ok(Command::new(path))
            }
        } else {
            Ok(Command::new(WIX_COMPILER))
        }
    }

    fn no_build(&self, manifest: &Value) -> bool {
        if self.no_build {
            true
        } else if let Some(pkg_meta_wix_no_build) = manifest
            .get("package")
            .and_then(|p| p.as_table())
            .and_then(|t| t.get("metadata"))
            .and_then(|m| m.as_table())
            .and_then(|t| t.get("wix"))
            .and_then(|w| w.as_table())
            .and_then(|t| t.get("no-build"))
            .and_then(|c| c.as_bool())
        {
            pkg_meta_wix_no_build
        } else {
            false
        }
    }

    fn culture(&self, manifest: &Value) -> Result<Cultures> {
        if let Some(culture) = &self.culture {
            Cultures::from_str(culture)
        } else if let Some(pkg_meta_wix_culture) = manifest
            .get("package")
            .and_then(|p| p.as_table())
            .and_then(|t| t.get("metadata"))
            .and_then(|m| m.as_table())
            .and_then(|t| t.get("wix"))
            .and_then(|w| w.as_table())
            .and_then(|t| t.get("culture"))
            .and_then(|c| c.as_str())
        {
            Cultures::from_str(pkg_meta_wix_culture)
        } else {
            Ok(Cultures::EnUs)
        }
    }

    fn locale(&self, manifest: &Value) -> Result<Option<PathBuf>> {
        if let Some(locale) = self.locale.as_ref().map(PathBuf::from) {
            if locale.exists() {
                Ok(Some(locale))
            } else {
                Err(Error::Generic(format!(
                    "The '{}' WiX localization file could not be found, or it does not exist. \
                     Please check the path is correct and the file exists.",
                    locale.display()
                )))
            }
        } else if let Some(pkg_meta_wix_locale) = manifest
            .get("package")
            .and_then(|p| p.as_table())
            .and_then(|t| t.get("metadata"))
            .and_then(|m| m.as_table())
            .and_then(|t| t.get("wix"))
            .and_then(|w| w.as_table())
            .and_then(|t| t.get("locale"))
            .and_then(|l| l.as_str())
            .map(PathBuf::from)
        {
            Ok(Some(pkg_meta_wix_locale))
        } else {
            Ok(None)
        }
    }

    fn linker(&self) -> Result<Command> {
        if let Some(mut path) = self.bin_path.as_ref().map(|s| {
            let mut p = PathBuf::from(s);
            trace!(
                "Using the '{}' path to the WiX Toolset '{}' folder for the linker",
                p.display(),
                BINARY_FOLDER_NAME
            );
            p.push(WIX_LINKER);
            p.set_extension(EXE_FILE_EXTENSION);
            p
        }) {
            if !path.exists() {
                path.pop(); // Remove the 'light' application from the path
                Err(Error::Generic(format!(
                    "The linker application ('{}') does not exist at the '{}' path specified via \
                     the '-b,--bin-path' command line argument. Please check the path is correct \
                     and the linker application exists at the path.",
                    WIX_LINKER,
                    path.display()
                )))
            } else {
                Ok(Command::new(path))
            }
        } else if let Some(mut path) = env::var_os(WIX_PATH_KEY).map(|s| {
            let mut p = PathBuf::from(s);
            trace!(
                "Using the '{}' path to the WiX Toolset's '{}' folder for the linker",
                p.display(),
                BINARY_FOLDER_NAME
            );
            p.push(BINARY_FOLDER_NAME);
            p.push(WIX_LINKER);
            p.set_extension(EXE_FILE_EXTENSION);
            p
        }) {
            if !path.exists() {
                path.pop(); // Remove the `candle` application from the path
                Err(Error::Generic(format!(
                    "The linker application ('{}') does not exist at the '{}' path specified \
                     via the {} environment variable. Please check the path is correct and the \
                     linker application exists at the path.",
                    WIX_LINKER,
                    path.display(),
                    WIX_PATH_KEY
                )))
            } else {
                Ok(Command::new(path))
            }
        } else {
            Ok(Command::new(WIX_LINKER))
        }
    }

    fn platform(&self) -> Platform {
        if cfg!(target_arch = "x86_64") {
            Platform::X64
        } else {
            Platform::X86
        }
    }

    fn name(&self, manifest: &Value) -> Result<String> {
        if let Some(ref p) = self.name {
            Ok(p.to_owned())
        } else if let Some(pkg_meta_wix_name) = manifest
            .get("package")
            .and_then(|p| p.as_table())
            .and_then(|t| t.get("metadata"))
            .and_then(|m| m.as_table())
            .and_then(|t| t.get("wix"))
            .and_then(|w| w.as_table())
            .and_then(|t| t.get("name"))
            .and_then(|n| n.as_str())
            .map(String::from)
        {
            Ok(pkg_meta_wix_name)
        } else {
            manifest
                .get("package")
                .and_then(|p| p.as_table())
                .and_then(|t| t.get("name"))
                .and_then(|n| n.as_str())
                .map(String::from)
                .ok_or(Error::Manifest("name"))
        }
    }

    fn msi_destination(
        &self,
        name: &str,
        version: &Version,
        platform: Platform,
        manifest: &Value,
    ) -> Result<PathBuf> {
        let filename = &format!(
            "{}-{}-{}.{}",
            name,
            version,
            platform.arch(),
            MSI_FILE_EXTENSION
        );
        if let Some(ref path_str) = self.output {
            trace!("Using the explicitly specified output path for the MSI destination");
            let path = Path::new(path_str);
            if path_str.ends_with('/') || path_str.ends_with('\\') || path.is_dir() {
                Ok(path.join(filename))
            } else {
                Ok(path.to_owned())
            }
        } else if let Some(pkg_meta_wix_output) = manifest
            .get("package")
            .and_then(|p| p.as_table())
            .and_then(|t| t.get("metadata"))
            .and_then(|m| m.as_table())
            .and_then(|t| t.get("wix"))
            .and_then(|w| w.as_table())
            .and_then(|t| t.get("output"))
            .and_then(|o| o.as_str())
        {
            trace!("Using the output path in the package's metadata for the MSI destination");
            let path = Path::new(pkg_meta_wix_output);
            if pkg_meta_wix_output.ends_with('/')
                || pkg_meta_wix_output.ends_with('\\')
                || path.is_dir()
            {
                Ok(path.join(filename))
            } else {
                Ok(path.to_owned())
            }
        } else {
            trace!("Using the current working directory and default output path for the MSI destination");
            if let Some(manifest_path) = &self.input {
                trace!("Using the package's manifest (Cargo.toml) file path to specify \
                        the MSI destination");
                // Remove the `Cargo.toml` file from the path
                manifest_path.parent().ok_or(Error::Generic(format!(
                    "The '{}' path for the package's manifest file is invalid", manifest_path.display()
                ))).map(|d| {
                    let mut dst = PathBuf::from(d);
                    dst.push(TARGET_FOLDER_NAME);
                    dst
                })
            } else {
                trace!("Using the current working directory (CWD) to build the WiX object files destination");
                Ok(PathBuf::from(TARGET_FOLDER_NAME).join(WIX).join(filename))
            }
        }
    }

    fn wixobj_destination(&self) -> Result<PathBuf> {
        let mut dst = if let Some(manifest_path) = &self.input {
            trace!("Using the package's manifest (Cargo.toml) file path to build \
                the Wix object files destination");
            // Remove the `Cargo.toml` file from the path
            manifest_path.parent().ok_or(Error::Generic(format!(
                "The '{}' path for the package's manifest file is invalid", manifest_path.display()
            ))).map(|d| {
                let mut dst = PathBuf::from(d);
                dst.push(TARGET_FOLDER_NAME);
                dst
            })
        } else {
            trace!("Using the current working directory (CWD) to build the WiX object files destination");
            Ok(PathBuf::from(TARGET_FOLDER_NAME))
        }?;
        // A trailing slash is needed; otherwise, candle tries to dump the
        // object files to a `target\wix` file instead of dumping the object
        // files in the `target\wix\` folder for the `-out` option. The trailing
        // slash must be done "manually" as a string instead of using the
        // PathBuf API because the PathBuf `push` and/or `join` methods treat a
        // single slash (forward or backward) without a prefix as the root `C:\`
        // or `/` and deletes the full path. This is noted in the documentation
        // for PathBuf, but it was unexpected and kind of annoying because I am
        // not sure how to add a trailing slash in a cross-platform way with
        // PathBuf, not that cargo-wix needs to be cross-platform.
        dst.push(format!("{}\\", WIX));
        Ok(dst)
    }

    fn wixobj_sources(&self, wixobj_dst: &Path) -> Result<Vec<PathBuf>> {
        let wixobj_sources: Vec<PathBuf> = std::fs::read_dir(wixobj_dst)?
            .filter(|r| r.is_ok())
            .map(|r| r.unwrap().path())
            .filter(|p| p.extension().and_then(|s| s.to_str()) == Some(WIX_OBJECT_FILE_EXTENSION))
            .collect();
        if wixobj_sources.is_empty() {
            Err(Error::Generic(String::from("No WiX object files found.")))
        } else {
            Ok(wixobj_sources)
        }
    }

    fn wxs_sources(&self, manifest: &Value) -> Result<Vec<PathBuf>> {
        let project_wix_dir = if let Some(manifest_path) = &self.input {
            trace!("Using the package's manifest (Cargo.toml) file path to obtain all WXS files");
            manifest_path.parent().ok_or(Error::Generic(format!(
                "The '{}' path for the package's manifest file is invalid", manifest_path.display()
            ))).map(|d| {
                let mut dst = PathBuf::from(d);
                dst.push(WIX);
                dst
            })
        } else {
            trace!("Using the current working directory (CWD) to obtain all WXS files");
            Ok(PathBuf::from(WIX))
        }?;
        let mut wix_sources: Vec<PathBuf> = std::fs::read_dir(project_wix_dir)?
            .filter(|r| r.is_ok())
            .map(|r| r.unwrap().path())
            .filter(|p| {
                p.extension().and_then(|s| s.to_str()) == Some(WIX_SOURCE_FILE_EXTENSION)
            })
            .collect();
        if let Some(paths) = self.includes.as_ref() {
            for p in paths {
                if p.exists() {
                    if p.is_dir() {
                        return Err(Error::Generic(format!(
                            "The '{}' path is not a file. Please check the path and ensure it is to \
                            a WiX Source (wxs) file.",
                            p.display()
                        )));
                    } else {
                        trace!("Using the '{}' WiX source file", p.display());
                    }
                } else {
                    return Err(Error::Generic(format!(
                        "The '{0}' file does not exist. Consider using the 'cargo \
                        wix print WXS > {0}' command to create it.",
                        p.display()
                    )));
                }
            }
            wix_sources.extend(paths.clone());
        } else if let Some(pkg_meta_wix_sources) = manifest
            .get("package")
            .and_then(|p| p.as_table())
            .and_then(|t| t.get("metadata"))
            .and_then(|m| m.as_table())
            .and_then(|t| t.get("wix"))
            .and_then(|w| w.as_table())
            .and_then(|t| t.get("include"))
            .and_then(|i| i.as_array())
            .map(|a| {
                a.iter()
                    .map(|s| s.as_str().map(PathBuf::from).unwrap())
                    .collect::<Vec<PathBuf>>()
            })
        {
            for pkg_meta_wix_source in &pkg_meta_wix_sources {
                if pkg_meta_wix_source.exists() {
                    if pkg_meta_wix_source.is_dir() {
                        return Err(Error::Generic(format!(
                            "The '{}' path is not a file. Please check the path and \
                            ensure it is to a WiX Source (wxs) file in the \
                            'package.metadata.wix' section of the package's manifest \
                            (Cargo.toml).",
                            pkg_meta_wix_source.display()
                        )));
                    } else {
                        trace!(
                            "Using the '{}' WiX source file from the \
                            'package.metadata.wix' section in the package's manifest.",
                            pkg_meta_wix_source.display()
                        );
                    }
                } else {
                    return Err(Error::Generic(format!(
                        "The '{0}' file does not exist. \
                        Consider using the 'cargo wix print WXS > {0} command to create \
                        it.",
                        pkg_meta_wix_source.display()
                    )));
                }
            }
            wix_sources.extend(pkg_meta_wix_sources);
        }
        if wix_sources.is_empty() {
            Err(Error::Generic(String::from(
                "There are no WXS files to create an installer",
            )))
        } else {
            Ok(wix_sources)
        }
    }

    fn version(&self, manifest: &Value) -> Result<Version> {
        if let Some(ref v) = self.version {
            Version::parse(v).map_err(Error::from)
        } else if let Some(pkg_meta_wix_version) = manifest
            .get("package")
            .and_then(|p| p.as_table())
            .and_then(|t| t.get("metadata"))
            .and_then(|m| m.as_table())
            .and_then(|t| t.get("wix"))
            .and_then(|w| w.as_table())
            .and_then(|t| t.get("version"))
            .and_then(|v| v.as_str())
        {
            Version::parse(pkg_meta_wix_version).map_err(Error::from)
        } else {
            manifest
                .get("package")
                .and_then(|p| p.as_table())
                .and_then(|t| t.get("version"))
                .and_then(|v| v.as_str())
                .ok_or(Error::Manifest("version"))
                .and_then(|s| Version::parse(s).map_err(Error::from))
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
        fn defaults_are_correct() {
            let actual = Builder::new();
            assert!(actual.bin_path.is_none());
            assert!(actual.capture_output);
            assert!(actual.culture.is_none());
            assert!(actual.includes.is_none());
            assert!(actual.input.is_none());
            assert!(actual.locale.is_none());
            assert!(actual.name.is_none());
            assert!(!actual.no_build);
            assert!(actual.output.is_none());
            assert!(actual.version.is_none());
        }

        #[test]
        fn bin_path_works() {
            const EXPECTED: &str = "C:\\Wix Toolset\\bin";
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
        fn culture_works() {
            const EXPECTED: &str = "FrFr";
            let mut actual = Builder::new();
            actual.culture(Some(EXPECTED));
            assert_eq!(actual.culture, Some(EXPECTED));
        }

        #[test]
        fn includes_works() {
            const EXPECTED: &str = "C:\\tmp\\hello_world\\wix\\main.wxs";
            let mut actual = Builder::new();
            actual.includes(Some(vec![EXPECTED]));
            assert_eq!(actual.includes, Some(vec![EXPECTED]));
        }

        #[test]
        fn input_works() {
            const EXPECTED: &str = "C:\\tmp\\hello_world\\Cargo.toml";
            let mut actual = Builder::new();
            actual.input(Some(EXPECTED));
            assert_eq!(actual.input, Some(EXPECTED));
        }

        #[test]
        fn locale_works() {
            const EXPECTED: &str = "C:\\tmp\\hello_world\\wix\\main.wxl";
            let mut actual = Builder::new();
            actual.locale(Some(EXPECTED));
            assert_eq!(actual.locale, Some(EXPECTED));
        }

        #[test]
        fn name_works() {
            const EXPECTED: &str = "Name";
            let mut actual = Builder::new();
            actual.name(Some(EXPECTED));
            assert_eq!(actual.name, Some(EXPECTED));
        }

        #[test]
        fn no_build_works() {
            let mut actual = Builder::new();
            actual.no_build(true);
            assert!(actual.no_build);
        }

        #[test]
        fn output_works() {
            const EXPECTED: &str = "C:\\tmp\\hello_world\\output";
            let mut actual = Builder::new();
            actual.output(Some(EXPECTED));
            assert_eq!(actual.output, Some(EXPECTED));
        }

        #[test]
        fn version_works() {
            const EXPECTED: &str = "1.2.3";
            let mut actual = Builder::new();
            actual.version(Some(EXPECTED));
            assert_eq!(actual.version, Some(EXPECTED));
        }

        #[test]
        fn build_with_defaults_works() {
            let mut b = Builder::new();
            let default_execution = b.build();
            assert!(default_execution.bin_path.is_none());
            assert!(default_execution.capture_output);
            assert!(default_execution.culture.is_none());
            assert!(default_execution.includes.is_none());
            assert!(default_execution.input.is_none());
            assert!(default_execution.locale.is_none());
            assert!(default_execution.name.is_none());
            assert!(!default_execution.no_build);
            assert!(default_execution.output.is_none());
            assert!(default_execution.version.is_none());
        }

        #[test]
        fn build_with_all_works() {
            const EXPECTED_BIN_PATH: &str = "C:\\Wix Toolset\\bin";
            const EXPECTED_CULTURE: &str = "FrFr";
            const EXPECTED_INCLUDES: &str = "C:\\tmp\\hello_world\\wix\\main.wxs";
            const EXPECTED_INPUT: &str = "C:\\tmp\\hello_world\\Cargo.toml";
            const EXPECTED_LOCALE: &str = "C:\\tmp\\hello_world\\wix\\main.wxl";
            const EXPECTED_NAME: &str = "Name";
            const EXPECTED_OUTPUT: &str = "C:\\tmp\\hello_world\\output";
            const EXPECTED_VERSION: &str = "1.2.3";
            let mut b = Builder::new();
            b.bin_path(Some(EXPECTED_BIN_PATH));
            b.capture_output(false);
            b.culture(Some(EXPECTED_CULTURE));
            b.includes(Some(vec![EXPECTED_INCLUDES]));
            b.input(Some(EXPECTED_INPUT));
            b.locale(Some(EXPECTED_LOCALE));
            b.name(Some(EXPECTED_NAME));
            b.no_build(true);
            b.output(Some(EXPECTED_OUTPUT));
            b.version(Some(EXPECTED_VERSION));
            let execution = b.build();
            assert_eq!(
                execution.bin_path,
                Some(EXPECTED_BIN_PATH).map(PathBuf::from)
            );
            assert!(!execution.capture_output);
            assert_eq!(execution.culture, Some(EXPECTED_CULTURE).map(String::from));
            assert_eq!(execution.includes, Some(vec![PathBuf::from(EXPECTED_INCLUDES)]));
            assert_eq!(execution.input, Some(PathBuf::from(EXPECTED_INPUT)));
            assert_eq!(execution.locale, Some(EXPECTED_LOCALE).map(PathBuf::from));
            assert_eq!(execution.name, Some(EXPECTED_NAME).map(String::from));
            assert!(execution.no_build);
            assert_eq!(execution.output, Some(EXPECTED_OUTPUT).map(String::from));
            assert_eq!(execution.version, Some(EXPECTED_VERSION).map(String::from));
        }
    }

    mod execution {
        use super::*;

        #[test]
        fn version_metadata_works() {
            const PKG_META_WIX: &str = r#"
                [package]
                version = "0.1.0"

                [package.metadata.wix]
                version = "2.1.0"
            "#;
            let execution = Execution::default();
            let version = execution
                .version(&PKG_META_WIX.parse::<Value>().unwrap())
                .unwrap();
            assert_eq!(version, Version::parse("2.1.0").unwrap());
        }

        #[test]
        fn name_metadata_works() {
            const PKG_META_WIX: &str = r#"
                [package]
                name = "example"

                [package.metadata.wix]
                name = "Metadata"
            "#;
            let execution = Execution::default();
            let name = execution
                .name(&PKG_META_WIX.parse::<Value>().unwrap())
                .unwrap();
            assert_eq!(name, "Metadata".to_owned());
        }

        #[test]
        fn no_build_metadata_works() {
            const PKG_META_WIX: &str = r#"
                [package.metadata.wix]
                no-build = true
            "#;
            let execution = Execution::default();
            let no_build = execution.no_build(&PKG_META_WIX.parse::<Value>().unwrap());
            assert!(no_build);
        }

        #[test]
        fn culture_metadata_works() {
            const PKG_META_WIX: &str = r#"
                [package.metadata.wix]
                culture = "Fr-Fr"
            "#;
            let execution = Execution::default();
            let culture = execution
                .culture(&PKG_META_WIX.parse::<Value>().unwrap())
                .unwrap();
            assert_eq!(culture, Cultures::FrFr);
        }

        #[test]
        fn locale_metadata_works() {
            const PKG_META_WIX: &str = r#"
                [package.metadata.wix]
                locale = "wix/French.wxl"
            "#;
            let execution = Execution::default();
            let locale = execution
                .locale(&PKG_META_WIX.parse::<Value>().unwrap())
                .unwrap();
            assert_eq!(locale, Some(PathBuf::from("wix/French.wxl")));
        }

        #[test]
        fn output_metadata_works() {
            const PKG_META_WIX: &str = r#"
                [package.metadata.wix]
                output = "target/wix/test.msi"
            "#;
            let execution = Execution::default();
            let output = execution.destination_msi(
                "Different",
                &"2.1.0".parse::<Version>().unwrap(),
                Platform::X64,
                &PKG_META_WIX.parse::<Value>().unwrap(),
            );
            assert_eq!(output, PathBuf::from("target/wix/test.msi"));
        }

        #[test]
        fn include_metadata_works() {
            const PKG_META_WIX: &str = r#"
                [package.metadata.wix]
                include = ["Cargo.toml"]
            "#;
            let execution = Execution::default();
            let sources = execution
                .wxs_sources(&PKG_META_WIX.parse::<Value>().unwrap())
                .unwrap();
            assert_eq!(sources, vec![PathBuf::from("Cargo.toml")]);
        }

        const EMPTY_PKG_META_WIX: &str = r#"[package.metadata.wix]"#;

        #[test]
        fn culture_works() {
            let execution = Execution::default();
            let culture = execution
                .culture(&EMPTY_PKG_META_WIX.parse::<Value>().unwrap())
                .unwrap();
            assert_eq!(culture, Cultures::EnUs);
        }

        #[test]
        fn locale_works() {
            let execution = Execution::default();
            let locale = execution
                .locale(&EMPTY_PKG_META_WIX.parse::<Value>().unwrap())
                .unwrap();
            assert!(locale.is_none());
        }

        #[test]
        fn no_build_works() {
            let execution = Execution::default();
            let no_build = execution.no_build(&EMPTY_PKG_META_WIX.parse::<Value>().unwrap());
            assert!(!no_build);
        }

        #[test]
        fn compiler_is_correct_with_defaults() {
            let expected = Command::new(
                env::var_os(WIX_PATH_KEY)
                    .map(|s| {
                        let mut p = PathBuf::from(s);
                        p.push(BINARY_FOLDER_NAME);
                        p.push(WIX_COMPILER);
                        p.set_extension(EXE_FILE_EXTENSION);
                        p
                    })
                    .unwrap(),
            );
            let e = Execution::default();
            let actual = e.compiler().unwrap();
            assert_eq!(format!("{:?}", actual), format!("{:?}", expected));
        }

        #[test]
        fn wixobj_destination_works() {
            let execution = Execution::default();
            assert_eq!(
                execution.wixobj_destination(),
                PathBuf::from("target\\wix\\")
            )
        }
    }
}
