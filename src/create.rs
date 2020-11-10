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
use crate::WIX;
use crate::WIX_COMPILER;
use crate::WIX_LINKER;
use crate::WIX_OBJECT_FILE_EXTENSION;
use crate::WIX_PATH_KEY;
use crate::WIX_SOURCE_FILE_EXTENSION;

use semver::Version;

use std::convert::TryFrom;
use std::env;
use std::ffi::OsString;
use std::fmt;
use std::io::{ErrorKind, Read};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::str::FromStr;

use cargo_metadata::Package;
use serde_json::Value;

/// A builder for running the `cargo wix` subcommand.
#[derive(Debug, Clone)]
pub struct Builder<'a> {
    bin_path: Option<&'a str>,
    capture_output: bool,
    compiler_args: Option<Vec<&'a str>>,
    culture: Option<&'a str>,
    debug_build: bool,
    debug_name: bool,
    includes: Option<Vec<&'a str>>,
    input: Option<&'a str>,
    linker_args: Option<Vec<&'a str>>,
    locale: Option<&'a str>,
    name: Option<&'a str>,
    no_build: bool,
    output: Option<&'a str>,
    package: Option<&'a str>,
    target: Option<&'a str>,
    version: Option<&'a str>,
}

impl<'a> Builder<'a> {
    /// Creates a new `Builder` instance.
    pub fn new() -> Self {
        Builder {
            bin_path: None,
            capture_output: true,
            compiler_args: None,
            culture: None,
            debug_build: false,
            debug_name: false,
            includes: None,
            input: None,
            linker_args: None,
            locale: None,
            name: None,
            no_build: false,
            output: None,
            package: None,
            target: None,
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

    /// Adds an argument to the compiler command.
    ///
    /// This "passes" the argument directly to the WiX compiler (candle.exe).
    /// See the help documentation for the WiX compiler for information about
    /// valid options and flags.
    pub fn compiler_args(&mut self, c: Option<Vec<&'a str>>) -> &mut Self {
        self.compiler_args = c;
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

    /// Builds the package with the Debug profile instead of the Release profile.
    ///
    /// See the [Cargo book] for more information about release profiles. The
    /// default is to use the Release profile when creating the installer. This
    /// value is ignored if the `no_build` method is set to `true`.
    ///
    /// [Cargo book]: https://doc.rust-lang.org/book/ch14-01-release-profiles.html
    pub fn debug_build(&mut self, d: bool) -> &mut Self {
        self.debug_build = d;
        self
    }

    /// Appends `-debug` to the file stem for the installer (msi).
    ///
    /// If `true`, then `-debug` is added as suffix to the file stem (string
    /// before the dot and file extension) for the installer's file name. For
    /// example, if `true`, then file name would be
    /// `example-0.1.0-x86_64-debug.msi`. The default is to _not_ append the
    /// `-debug` because the Release profile is the default.
    ///
    /// Generally, this should be used in combination with the `debug_build`
    /// method to indicate the installer is for a debugging variant of the
    /// installed binary.
    pub fn debug_name(&mut self, d: bool) -> &mut Self {
        self.debug_name = d;
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

    /// Adds an argument to the linker command.
    ///
    /// This "passes" the argument directly to the WiX linker (light.exe). See
    /// the help documentation for the WiX compiler for information about valid
    /// options and flags.
    pub fn linker_args(&mut self, l: Option<Vec<&'a str>>) -> &mut Self {
        self.linker_args = l;
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

    /// Sets the package.
    ///
    /// If the project is organized using a workspace, this selects the package
    /// by name to create an installer. If a workspace is not used, then this
    /// has no effect.
    pub fn package(&mut self, p: Option<&'a str>) -> &mut Self {
        self.package = p;
        self
    }

    /// Sets the target architecture.
    pub fn target(&mut self, v: Option<&'a str>) -> &mut Self {
        self.target = v;
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
            compiler_args: self
                .compiler_args
                .as_ref()
                .map(|c| c.iter().map(|s| (*s).to_string()).collect()),
            culture: self.culture.map(String::from),
            debug_build: self.debug_build,
            debug_name: self.debug_name,
            includes: self
                .includes
                .as_ref()
                .map(|v| v.iter().map(&PathBuf::from).collect()),
            input: self.input.map(PathBuf::from),
            linker_args: self
                .linker_args
                .as_ref()
                .map(|l| l.iter().map(|s| (*s).to_string()).collect()),
            locale: self.locale.map(PathBuf::from),
            name: self.name.map(String::from),
            no_build: self.no_build,
            output: self.output.map(String::from),
            package: self.package.map(String::from),
            version: self.version.map(String::from),
            target: self.target.map(String::from),
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
    compiler_args: Option<Vec<String>>,
    culture: Option<String>,
    debug_build: bool,
    debug_name: bool,
    includes: Option<Vec<PathBuf>>,
    input: Option<PathBuf>,
    linker_args: Option<Vec<String>>,
    locale: Option<PathBuf>,
    name: Option<String>,
    no_build: bool,
    output: Option<String>,
    package: Option<String>,
    target: Option<String>,
    version: Option<String>,
}

impl Execution {
    /// Creates, or builds, an installer within a built context.
    #[allow(clippy::cognitive_complexity)]
    pub fn run(self) -> Result<()> {
        debug!("self.bin_path = {:?}", self.bin_path);
        debug!("self.capture_output = {:?}", self.capture_output);
        debug!("self.compiler_args = {:?}", self.compiler_args);
        debug!("self.culture = {:?}", self.culture);
        debug!("self.debug_build = {:?}", self.debug_build);
        debug!("self.debug_name = {:?}", self.debug_name);
        debug!("self.includes = {:?}", self.includes);
        debug!("self.input = {:?}", self.input);
        debug!("self.linker_args = {:?}", self.linker_args);
        debug!("self.locale = {:?}", self.locale);
        debug!("self.name = {:?}", self.name);
        debug!("self.no_build = {:?}", self.no_build);
        debug!("self.output = {:?}", self.output);
        debug!("self.package = {:?}", self.package);
        debug!("self.target = {:?}", self.target);
        debug!("self.version = {:?}", self.version);
        let manifest_path = super::cargo_toml_file(self.input.as_ref())?;
        debug!("manifest_path = {:?}", manifest_path);
        let manifest = super::manifest(self.input.as_ref())?;
        debug!("target_directory = {:?}", manifest.target_directory);
        let package = super::package(&manifest, self.package.as_deref())?;
        debug!("package = {:?}", package);
        let metadata = package.metadata.clone();
        debug!("metadata = {:?}", metadata);
        let name = self.name(&package)?;
        debug!("name = {:?}", name);
        let version = self.version(&package)?;
        debug!("version = {:?}", version);
        let compiler_args = self.compiler_args(&metadata);
        debug!("compiler_args = {:?}", compiler_args);
        let culture = self.culture(&metadata)?;
        debug!("culture = {:?}", culture);
        let linker_args = self.linker_args(&metadata);
        debug!("linker_args = {:?}", linker_args);
        let locale = self.locale(&metadata)?;
        debug!("locale = {:?}", locale);
        let platform = self.platform()?;
        debug!("platform = {:?}", platform);
        let debug_build = self.debug_build(&metadata);
        debug!("debug_build = {:?}", debug_build);
        let profile = if debug_build { "debug" } else { "release" };
        debug!("profile = {:?}", profile);
        let debug_name = self.debug_name(&metadata);
        debug!("debug_name = {:?}", debug_name);
        let wxs_sources = self.wxs_sources(&package)?;
        debug!("wxs_sources = {:?}", wxs_sources);
        let wixobj_destination = self.wixobj_destination(&manifest.target_directory)?;
        debug!("wixobj_destination = {:?}", wixobj_destination);
        let no_build = self.no_build(&metadata);
        debug!("no_build = {:?}", no_build);
        if no_build {
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
            builder.arg("build");
            if !debug_build {
                builder.arg("--release");
            }
            if let Some(target) = &self.target {
                builder.arg(format!("--target={}", target));
            }
            builder.arg("--manifest-path").arg(&manifest_path);
            debug!("command = {:?}", builder);
            let status = builder.status()?;
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
        compiler.arg(format!("-dProfile={}", profile));
        compiler
            .arg(format!("-dVersion={}", version))
            .arg(format!("-dPlatform={}", platform.wix_arch()?))
            .arg({
                let mut s = OsString::from("-dCargoTargetDir=");
                s.push(&manifest.target_directory);
                s
            })
            .arg({
                let mut s = OsString::from("-dCargoTargetBinDir=");
                let mut bin_path = manifest.target_directory.clone();
                if let Some(target) = &self.target {
                    bin_path.push(target);
                }
                bin_path.push(profile);
                s.push(&bin_path);
                s
            })
            .arg(format!("-dCargoTarget={}", platform.target()));
        compiler
            .arg("-ext")
            .arg("WixUtilExtension")
            .arg("-o")
            .arg(&wixobj_destination);
        if let Some(args) = &compiler_args {
            trace!("Appending compiler arguments");
            compiler.args(args);
        }
        compiler.args(&wxs_sources);
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
        let wixobj_sources = self.wixobj_sources(&wixobj_destination)?;
        debug!("wixobj_sources = {:?}", wixobj_sources);
        let installer_kind = InstallerKind::try_from(
            wixobj_sources
                .iter()
                .map(WixObjKind::try_from)
                .collect::<Result<Vec<WixObjKind>>>()?,
        )?;
        debug!("installer_kind = {:?}", installer_kind);
        let installer_destination = self.installer_destination(
            &name,
            &version,
            platform,
            debug_name,
            &installer_kind,
            &package,
            &manifest.target_directory,
        )?;
        debug!("installer_destination = {:?}", installer_destination);
        // Link the installer
        info!("Linking the installer");
        let mut linker = self.linker()?;
        debug!("linker = {:?}", linker);
        let base_path = manifest_path.parent().ok_or_else(|| {
            Error::Generic(String::from("The base path for the linker is invalid"))
        })?;
        debug!("base_path = {:?}", base_path);
        if self.capture_output {
            trace!("Capturing the '{}' output", WIX_LINKER);
            linker.stdout(Stdio::null());
            linker.stderr(Stdio::null());
        }
        linker
            .arg("-spdb")
            .arg("-ext")
            .arg("WixUIExtension")
            .arg("-ext")
            .arg("WixUtilExtension")
            .arg(format!("-cultures:{}", culture))
            .arg("-out")
            .arg(&installer_destination)
            .arg("-b")
            .arg(&base_path);
        if let Some(l) = locale {
            trace!("Using the a WiX localization file");
            linker.arg("-loc").arg(l);
        }
        if let InstallerKind::Exe = installer_kind {
            trace!("Adding the WixBalExtension for the bundle-based installer");
            linker.arg("-ext").arg("WixBalExtension");
        }
        if let Some(args) = &linker_args {
            trace!("Appending linker arguments");
            linker.args(args);
        }
        linker.args(&wixobj_sources);
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

    fn debug_build(&self, metadata: &Value) -> bool {
        if self.debug_build {
            true
        } else if let Some(pkg_meta_wix_debug_build) = metadata
            .get("wix")
            .and_then(|w| w.as_object())
            .and_then(|t| t.get("dbg-build"))
            .and_then(|c| c.as_bool())
        {
            pkg_meta_wix_debug_build
        } else {
            false
        }
    }

    fn debug_name(&self, metadata: &Value) -> bool {
        if self.debug_name {
            true
        } else if let Some(pkg_meta_wix_debug_name) = metadata
            .get("wix")
            .and_then(|w| w.as_object())
            .and_then(|t| t.get("dbg-name"))
            .and_then(|c| c.as_bool())
        {
            pkg_meta_wix_debug_name
        } else {
            false
        }
    }

    fn no_build(&self, metadata: &Value) -> bool {
        if self.no_build {
            true
        } else if let Some(pkg_meta_wix_no_build) = metadata
            .get("wix")
            .and_then(|w| w.as_object())
            .and_then(|t| t.get("no-build"))
            .and_then(|c| c.as_bool())
        {
            pkg_meta_wix_no_build
        } else {
            false
        }
    }

    fn compiler_args(&self, metadata: &Value) -> Option<Vec<String>> {
        self.compiler_args.to_owned().or_else(|| {
            metadata
                .get("wix")
                .and_then(|w| w.as_object())
                .and_then(|t| t.get("compiler-args"))
                .and_then(|i| i.as_array())
                .map(|a| {
                    a.iter()
                        .map(|s| s.as_str().map(String::from).unwrap())
                        .collect::<Vec<String>>()
                })
        })
    }

    fn linker_args(&self, metadata: &Value) -> Option<Vec<String>> {
        self.linker_args.to_owned().or_else(|| {
            metadata
                .get("wix")
                .and_then(|w| w.as_object())
                .and_then(|t| t.get("linker-args"))
                .and_then(|i| i.as_array())
                .map(|a| {
                    a.iter()
                        .map(|s| s.as_str().map(String::from).unwrap())
                        .collect::<Vec<String>>()
                })
        })
    }

    fn culture(&self, metadata: &Value) -> Result<Cultures> {
        if let Some(culture) = &self.culture {
            Cultures::from_str(culture)
        } else if let Some(pkg_meta_wix_culture) = metadata
            .get("wix")
            .and_then(|w| w.as_object())
            .and_then(|t| t.get("culture"))
            .and_then(|c| c.as_str())
        {
            Cultures::from_str(pkg_meta_wix_culture)
        } else {
            Ok(Cultures::EnUs)
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn installer_destination(
        &self,
        name: &str,
        version: &Version,
        platform: Platform,
        debug_name: bool,
        installer_kind: &InstallerKind,
        package: &Package,
        target_directory: &Path,
    ) -> Result<PathBuf> {
        let filename = if debug_name {
            format!(
                "{}-{}-{}-debug.{}",
                name,
                version,
                platform.arch(),
                installer_kind
            )
        } else {
            format!(
                "{}-{}-{}.{}",
                name,
                version,
                platform.arch(),
                installer_kind
            )
        };
        if let Some(ref path_str) = self.output {
            trace!("Using the explicitly specified output path for the MSI destination");
            let path = Path::new(path_str);
            if path_str.ends_with('/') || path_str.ends_with('\\') || path.is_dir() {
                Ok(path.join(filename))
            } else {
                Ok(path.to_owned())
            }
        } else if let Some(pkg_meta_wix_output) = package
            .metadata
            .get("wix")
            .and_then(|w| w.as_object())
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
            trace!("Using the package's manifest (Cargo.toml) file path to specify the MSI destination");
            Ok(target_directory.join(WIX).join(filename))
        }
    }

    fn locale(&self, metadata: &Value) -> Result<Option<PathBuf>> {
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
        } else if let Some(pkg_meta_wix_locale) = metadata
            .get("wix")
            .and_then(|w| w.as_object())
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

    fn platform(&self) -> Result<Platform> {
        Ok(Platform::new(&self.target()?))
    }

    fn name(&self, package: &Package) -> Result<String> {
        if let Some(ref p) = self.name {
            Ok(p.to_owned())
        } else if let Some(pkg_meta_wix_name) = package
            .metadata
            .get("wix")
            .and_then(|w| w.as_object())
            .and_then(|t| t.get("name"))
            .and_then(|n| n.as_str())
            .map(String::from)
        {
            Ok(pkg_meta_wix_name)
        } else {
            Ok(package.name.clone())
        }
    }

    /// Gets the target used to compile the binary. This will either return the
    /// target passed as an argument, or find it from rustc's HOST triple.
    // NOTE: This does not support default-target. Ideally we would use cargo
    // --unit-graph to figure this out without having to second-guess the
    // compiler. Unfortunately, cargo --unit-graph is unstable.
    fn target(&self) -> Result<String> {
        match &self.target {
            Some(v) => Ok(v.clone()),
            None => get_host_triple(),
        }
    }

    fn wixobj_destination(&self, target_directory: &Path) -> Result<PathBuf> {
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
        let dst = target_directory.join(WIX).join("");
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

    fn wxs_sources(&self, package: &Package) -> Result<Vec<PathBuf>> {
        let project_wix_dir = package
            .manifest_path
            .parent()
            .ok_or_else(|| {
                Error::Generic(format!(
                    "The '{}' path for the package's manifest file is invalid",
                    package.manifest_path.display()
                ))
            })
            .map(|d| PathBuf::from(d).join(WIX))?;
        let mut wix_sources = {
            if project_wix_dir.exists() {
                std::fs::read_dir(project_wix_dir)?
                    .filter(|r| r.is_ok())
                    .map(|r| r.unwrap().path())
                    .filter(|p| {
                        p.extension().and_then(|s| s.to_str()) == Some(WIX_SOURCE_FILE_EXTENSION)
                    })
                    .collect()
            } else {
                Vec::new()
            }
        };
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
        } else if let Some(pkg_meta_wix_sources) = package
            .metadata
            .get("wix")
            .and_then(|w| w.as_object())
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
                             'package.metadata.wix' section in the package's \
                             manifest.",
                            pkg_meta_wix_source.display()
                        );
                    }
                } else {
                    return Err(Error::Generic(format!(
                        "The '{0}' file does not exist. Consider using the \
                         'cargo wix print WXS > {0} command to create it.",
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

    fn version(&self, package: &Package) -> Result<Version> {
        if let Some(ref v) = self.version {
            Version::parse(v).map_err(Error::from)
        } else if let Some(pkg_meta_wix_version) = package
            .metadata
            .get("wix")
            .and_then(|w| w.as_object())
            .and_then(|t| t.get("version"))
            .and_then(|v| v.as_str())
        {
            Version::parse(pkg_meta_wix_version).map_err(Error::from)
        } else {
            Ok(package.version.clone())
        }
    }
}

/// Gets the compiler's host triple, used to know what the default target is.
fn get_host_triple() -> Result<String> {
    let output = Command::new("rustc")
        .args(&["--version", "--verbose"])
        .output()?;
    for line in output.stdout.split(|b| *b == b'\n') {
        let mut line_elt = line.splitn(2, |b| *b == b':');
        let first = line_elt.next();
        let second = line_elt.next();
        if let (Some(b"host"), Some(host_triple)) = (first, second) {
            let s = String::from_utf8(host_triple.to_vec()).map_err(|_| {
                Error::Generic(
                    "Failed to parse output of rustc --verbose --version: invalid UTF8".to_string(),
                )
            });
            return Ok(s?.trim().to_string());
        }
    }
    Err(Error::Generic(
        "Failed to parse output of rustc --verbose --version".to_string(),
    ))
}

impl Default for Execution {
    fn default() -> Self {
        Builder::new().build()
    }
}

/// The kind of WiX Object (wixobj) file.
#[derive(Debug, PartialEq)]
pub enum WixObjKind {
    /// A WiX Object (wixobj) file that ultimately links back to a WiX Source
    /// (wxs) file with a [`bundle`] tag.
    ///
    /// [`bundle`]: https://wixtoolset.org/documentation/manual/v3/xsd/wix/bundle.html
    Bundle,
    /// A WiX Object (wixobj) file that ultimately links back to a WiX Source
    /// (wxs) file with a [`fragment`] tag.
    ///
    /// [`fragment`]: https://wixtoolset.org/documentation/manual/v3/xsd/wix/fragment.html
    Fragment,
    /// A WiX Object (wixobj) file that ultimately links back to a WiX Source
    /// (wxs) file with a [`product`] tag.
    ///
    /// [`product`]: https://wixtoolset.org/documentation/manual/v3/xsd/wix/product.html
    Product,
}

impl WixObjKind {
    /// Determines if the WiX Object (wixobj) file kind is a [`bundle`].
    ///
    /// # Examples
    ///
    /// A WiX Object (wixobj) file identified as a WXS Source (wxs) file
    /// containing a [`bundle`] tag.
    ///
    /// ```
    /// use wix::create::WixObjKind;
    ///
    /// assert!(WixObjKind::Bundle.is_bundle())
    /// ```
    ///
    /// A WiX Object (wixobj) file identified as a WXS Source (wxs) file
    /// containing a [`product`] tag.
    ///
    /// ```
    /// use wix::create::WixObjKind;
    ///
    /// assert!(!WixObjKind::Product.is_bundle())
    /// ```
    ///
    /// [`bundle`]: https://wixtoolset.org/documentation/manual/v3/xsd/wix/bundle.html
    /// [`product`]: https://wixtoolset.org/documentation/manual/v3/xsd/wix/product.html
    pub fn is_bundle(&self) -> bool {
        match *self {
            Self::Bundle => true,
            Self::Fragment => false,
            Self::Product => false,
        }
    }
}

impl FromStr for WixObjKind {
    type Err = crate::Error;

    fn from_str(value: &str) -> Result<Self> {
        match &*value.to_lowercase() {
            "bundle" => Ok(Self::Bundle),
            "fragment" => Ok(Self::Fragment),
            "product" => Ok(Self::Product),
            v => Err(Self::Err::Generic(format!(
                "Unknown '{}' tag name from a WiX Object (wixobj) file.",
                v
            ))),
        }
    }
}

impl TryFrom<&PathBuf> for WixObjKind {
    type Error = crate::Error;

    fn try_from(path: &PathBuf) -> Result<Self> {
        let file = std::fs::File::open(path)?;
        let mut decoder = encoding_rs_io::DecodeReaderBytes::new(file);
        let mut content = String::new();
        decoder.read_to_string(&mut content)?;
        Self::try_from(content.as_str())
    }
}

impl TryFrom<&str> for WixObjKind {
    type Error = crate::Error;

    fn try_from(content: &str) -> Result<Self> {
        let package = sxd_document::parser::parse(content)?;
        let document = package.as_document();
        let mut context = sxd_xpath::Context::new();
        context.set_namespace("wix", "http://schemas.microsoft.com/wix/2006/objects");
        // The assumption is that the following cannot fail because the path is known to be valid at
        // compile-time.
        let xpath = sxd_xpath::Factory::new()
            .build("/wix:wixObject/wix:section/@type")
            .unwrap()
            .unwrap();
        let value = xpath.evaluate(&context, document.root())?.string();
        Self::from_str(&value)
    }
}

/// The kinds of installers that can be created using the WiX compiler
/// (candle.exe) and linker (light.exe).
#[derive(Debug, PartialEq)]
pub enum InstallerKind {
    /// An executable is used when an [Installation Package Bundle] is created.
    ///
    /// [Installation Package Bundle]: https://wixtoolset.org/documentation/manual/v3/bundle/
    Exe,
    /// A Microsoft installer. This is the more common and typical installer to be created.
    Msi,
}

impl InstallerKind {
    /// Gets the file extension _without_ the dot separator.
    ///
    /// # Examples
    ///
    /// The extension for an installer of an [Installation Package Bundle]. Also
    /// see the [`EXE_FILE_EXTENSION`] constant.
    ///
    /// ```
    /// use wix::create::InstallerKind;
    ///
    /// assert_eq!(InstallerKind::Exe.extension(), "exe")
    /// ```
    ///
    /// The extension for a typical Microsoft installer. Also see the
    /// [`MSI_FILE_EXTENSION`] constant.
    ///
    /// ```
    /// use wix::create::InstallerKind;
    ///
    /// assert_eq!(InstallerKind::Msi.extension(), "msi")
    /// ```
    ///
    /// [Installation Package Bundle]: https://wixtoolset.org/documentation/manual/v3/bundle/
    /// [`EXE_FILE_EXTENSION`]: lib.html#exe-file-extension
    /// [`MSI_FILE_EXTENSION`]: lib.html#msi-file-extension
    pub fn extension(&self) -> &'static str {
        match *self {
            Self::Exe => EXE_FILE_EXTENSION,
            Self::Msi => MSI_FILE_EXTENSION,
        }
    }
}

impl FromStr for InstallerKind {
    type Err = crate::Error;

    fn from_str(value: &str) -> Result<Self> {
        match &*value.to_lowercase() {
            "exe" => Ok(Self::Exe),
            "msi" => Ok(Self::Msi),
            _ => Err(Self::Err::Generic(format!(
                "Unknown '{}' file extension for an installer",
                value
            ))),
        }
    }
}

impl fmt::Display for InstallerKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.extension())
    }
}

impl TryFrom<Vec<WixObjKind>> for InstallerKind {
    type Error = crate::Error;

    fn try_from(v: Vec<WixObjKind>) -> Result<Self> {
        v.iter()
            .fold(None, |a, v| match v {
                WixObjKind::Bundle => Some(Self::Exe),
                WixObjKind::Product => a.or(Some(Self::Msi)),
                _ => a,
            })
            .ok_or_else(|| {
                Self::Error::Generic(String::from(
                    "Could not determine the installer kind based on the WiX \
                     object files. There needs to be at least one 'product' or \
                     'bundle' tag in the collective WiX source files (wxs).",
                ))
            })
    }
}

impl Default for InstallerKind {
    fn default() -> Self {
        InstallerKind::Msi
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
            assert!(actual.compiler_args.is_none());
            assert!(actual.culture.is_none());
            assert!(!actual.debug_build);
            assert!(!actual.debug_name);
            assert!(actual.includes.is_none());
            assert!(actual.input.is_none());
            assert!(actual.linker_args.is_none());
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
        fn compiler_args_with_single_value_works() {
            const EXPECTED: &str = "-nologo";
            let mut actual = Builder::new();
            actual.compiler_args(Some(vec![EXPECTED]));
            assert_eq!(actual.compiler_args, Some(vec![EXPECTED]));
        }

        #[test]
        fn compiler_args_with_multiple_values_works() {
            let expected: Vec<&str> = vec!["-arch", "x86"];
            let mut actual = Builder::new();
            actual.compiler_args(Some(expected.clone()));
            assert_eq!(actual.compiler_args, Some(expected));
        }

        #[test]
        fn culture_works() {
            const EXPECTED: &str = "FrFr";
            let mut actual = Builder::new();
            actual.culture(Some(EXPECTED));
            assert_eq!(actual.culture, Some(EXPECTED));
        }

        #[test]
        fn russian_culture_works() {
            const EXPECTED: &str = "RuRu";
            let mut actual = Builder::new();
            actual.culture(Some(EXPECTED));
            assert_eq!(actual.culture, Some(EXPECTED));
        }

        #[test]
        fn debug_build_works() {
            let mut actual = Builder::new();
            actual.debug_build(true);
            assert!(actual.debug_build);
        }

        #[test]
        fn debug_name_works() {
            let mut actual = Builder::new();
            actual.debug_name(true);
            assert!(actual.debug_name);
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
        fn linker_args_with_single_value_works() {
            const EXPECTED: &str = "-nologo";
            let mut actual = Builder::new();
            actual.linker_args(Some(vec![EXPECTED]));
            assert_eq!(actual.linker_args, Some(vec![EXPECTED]));
        }

        #[test]
        fn linker_args_with_multiple_values_works() {
            let expected: Vec<&str> = vec!["-ext", "HelloExtension"];
            let mut actual = Builder::new();
            actual.linker_args(Some(expected.clone()));
            assert_eq!(actual.linker_args, Some(expected));
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
            assert!(default_execution.compiler_args.is_none());
            assert!(default_execution.culture.is_none());
            assert!(!default_execution.debug_build);
            assert!(!default_execution.debug_name);
            assert!(default_execution.includes.is_none());
            assert!(default_execution.input.is_none());
            assert!(default_execution.linker_args.is_none());
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
            const EXPECTED_COMPILER_ARGS: &str = "-nologo";
            const EXPECTED_INCLUDES: &str = "C:\\tmp\\hello_world\\wix\\main.wxs";
            const EXPECTED_INPUT: &str = "C:\\tmp\\hello_world\\Cargo.toml";
            const EXPECTED_LINKER_ARGS: &str = "-nologo";
            const EXPECTED_LOCALE: &str = "C:\\tmp\\hello_world\\wix\\main.wxl";
            const EXPECTED_NAME: &str = "Name";
            const EXPECTED_OUTPUT: &str = "C:\\tmp\\hello_world\\output";
            const EXPECTED_VERSION: &str = "1.2.3";
            let mut b = Builder::new();
            b.bin_path(Some(EXPECTED_BIN_PATH));
            b.capture_output(false);
            b.culture(Some(EXPECTED_CULTURE));
            b.compiler_args(Some(vec![EXPECTED_COMPILER_ARGS]));
            b.debug_build(true);
            b.debug_name(true);
            b.includes(Some(vec![EXPECTED_INCLUDES]));
            b.input(Some(EXPECTED_INPUT));
            b.linker_args(Some(vec![EXPECTED_LINKER_ARGS]));
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
            assert_eq!(
                execution.compiler_args,
                Some(vec![String::from(EXPECTED_COMPILER_ARGS)])
            );
            assert_eq!(execution.culture, Some(EXPECTED_CULTURE).map(String::from));
            assert!(execution.debug_build);
            assert!(execution.debug_name);
            assert_eq!(
                execution.includes,
                Some(vec![PathBuf::from(EXPECTED_INCLUDES)])
            );
            assert_eq!(execution.input, Some(PathBuf::from(EXPECTED_INPUT)));
            assert_eq!(
                execution.linker_args,
                Some(vec![String::from(EXPECTED_LINKER_ARGS)])
            );
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
        fn debug_build_metadata_works() {
            const PKG_META_WIX: &str = r#"{
                "wix": {
                    "dbg-build": true
                }
            }"#;
            let execution = Execution::default();
            let debug_build = execution.debug_build(&PKG_META_WIX.parse::<Value>().unwrap());
            assert!(debug_build);
        }

        #[test]
        fn debug_name_metadata_works() {
            const PKG_META_WIX: &str = r#"{
                "wix": {
                    "dbg-name": true
                }
            }"#;
            let execution = Execution::default();
            let debug_name = execution.debug_name(&PKG_META_WIX.parse::<Value>().unwrap());
            assert!(debug_name);
        }

        #[test]
        fn version_metadata_works() {
            const PKG_META_WIX: &str = r#"
            {
                "name": "Example",
                "version": "0.1.0",
                "authors": ["First Last <first.last@example.com>"],
                "license": "Apache-2.0",

                "id": "",
                "dependencies": [],
                "targets": [],
                "features": {},
                "manifest_path": "",

                "metadata": {
                    "wix": {
                        "version": "2.1.0"
                    }
                }
            }"#;
            let execution = Execution::default();
            let version = execution
                .version(&serde_json::from_str(PKG_META_WIX).unwrap())
                .unwrap();
            assert_eq!(version, Version::parse("2.1.0").unwrap());
        }

        #[test]
        fn name_metadata_works() {
            const PKG_META_WIX: &str = r#"{
                "name": "Example",
                "version": "0.1.0",
                "metadata": {
                    "wix": {
                        "name": "Metadata"
                    }
                },

                "id": "",
                "dependencies": [],
                "targets": [],
                "features": {},
                "manifest_path": ""
            }"#;
            let execution = Execution::default();
            let name = execution
                .name(&serde_json::from_str(PKG_META_WIX).unwrap())
                .unwrap();
            assert_eq!(name, "Metadata".to_owned());
        }

        #[test]
        fn no_build_metadata_works() {
            const PKG_META_WIX: &str = r#"{
                "wix": {
                    "no-build": true
                }
            }"#;
            let execution = Execution::default();
            let no_build = execution.no_build(&PKG_META_WIX.parse::<Value>().unwrap());
            assert!(no_build);
        }

        #[test]
        fn culture_metadata_works() {
            const PKG_META_WIX: &str = r#"{
                "wix": {
                    "culture": "Fr-Fr"
                }
            }"#;
            let execution = Execution::default();
            let culture = execution
                .culture(&PKG_META_WIX.parse::<Value>().unwrap())
                .unwrap();
            assert_eq!(culture, Cultures::FrFr);
        }

        #[test]
        fn locale_metadata_works() {
            const PKG_META_WIX: &str = r#"{
                "wix": {
                    "locale": "wix/French.wxl"
                }
            }"#;
            let execution = Execution::default();
            let locale = execution
                .locale(&PKG_META_WIX.parse::<Value>().unwrap())
                .unwrap();
            assert_eq!(locale, Some(PathBuf::from("wix/French.wxl")));
        }

        #[test]
        fn output_metadata_works() {
            const PKG_META_WIX: &str = r#"{
                "name": "Example",
                "version": "0.1.0",
                "authors": ["First Last <first.last@example.com>"],
                "license": "XYZ",

                "id": "",
                "dependencies": [],
                "targets": [],
                "features": {},
                "manifest_path": "",
                "metadata": {
                    "wix": {
                        "output": "target/wix/test.msi"
                    }
                }
            }"#;
            let execution = Execution::default();
            let output = execution
                .installer_destination(
                    "Different",
                    &"2.1.0".parse::<Version>().unwrap(),
                    Platform {
                        target: "x86_64-pc-windows-msvc".into(),
                    },
                    false,
                    &InstallerKind::default(),
                    &serde_json::from_str(PKG_META_WIX).unwrap(),
                    Path::new("target/"),
                )
                .unwrap();
            assert_eq!(output, PathBuf::from("target/wix/test.msi"));
        }

        #[test]
        fn include_metadata_works() {
            const PKG_META_WIX: &str = r#"{
                "name": "Example",
                "version": "0.1.0",
                "authors": ["First Last <first.last@example.com>"],
                "license": "XYZ",

                "id": "",
                "dependencies": [],
                "targets": [],
                "features": {},
                "manifest_path": "C:\\Cargo.toml",
                "metadata": {
                    "wix": {
                        "include": ["Cargo.toml"]
                    }
                }
            }"#;
            let execution = Execution::default();
            let sources = execution
                .wxs_sources(&serde_json::from_str(PKG_META_WIX).unwrap())
                .unwrap();
            assert_eq!(sources, vec![PathBuf::from("Cargo.toml")]);
        }

        #[test]
        fn compiler_args_override_works() {
            const PKG_META_WIX: &str = r#"{
                "wix": {
                    "compiler-args": ["-nologo"]
                }
            }"#;
            let mut builder = Builder::default();
            builder.compiler_args(Some(vec!["-ws"]));
            let execution = builder.build();
            let args = execution.compiler_args(&PKG_META_WIX.parse::<Value>().unwrap());
            assert_eq!(args, Some(vec![String::from("-ws")]));
        }

        #[test]
        fn compiler_args_metadata_works() {
            const PKG_META_WIX: &str = r#"{
                "wix": {
                    "compiler-args": ["-nologo", "-ws"]
                }
            }"#;
            let execution = Execution::default();
            let args = execution.compiler_args(&PKG_META_WIX.parse::<Value>().unwrap());
            assert_eq!(
                args,
                Some(vec![String::from("-nologo"), String::from("-ws")])
            );
        }

        #[test]
        fn linker_args_override_works() {
            const PKG_META_WIX: &str = r#"{
                "wix": {
                    "linker-args": ["-nologo"]
                }
            }"#;
            let mut builder = Builder::default();
            builder.linker_args(Some(vec!["-ws"]));
            let execution = builder.build();
            let args = execution.linker_args(&PKG_META_WIX.parse::<Value>().unwrap());
            assert_eq!(args, Some(vec![String::from("-ws")]));
        }

        #[test]
        fn linker_args_metadata_works() {
            const PKG_META_WIX: &str = r#"{
                "wix": {
                    "linker-args": ["-nologo", "-ws"]
                }
            }"#;
            let execution = Execution::default();
            let args = execution.linker_args(&PKG_META_WIX.parse::<Value>().unwrap());
            assert_eq!(
                args,
                Some(vec![String::from("-nologo"), String::from("-ws")])
            );
        }

        const EMPTY_PKG_META_WIX: &str = r#"{"wix": {}}"#;

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
                execution.wixobj_destination(Path::new("target")).unwrap(),
                PathBuf::from("target").join("wix")
            )
        }
    }

    mod wixobj_kind {
        use super::*;

        // I do not know if any of these XML strings are actually valid WiX
        // Object (wixobj) files. These are just snippets to test the XPath
        // evaluation.

        const PRODUCT_WIXOBJ: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
            <wixObject version="3.0.2002.0" xmlns="http://schemas.microsoft.com/wix/2006/objects">
                <section id="*" type="product"></section>
            </wixObject>"#;

        const BUNDLE_WIXOBJ: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
            <wixObject version="3.0.2002.0" xmlns="http://schemas.microsoft.com/wix/2006/objects">
                <section id="*" type="bundle"></section>
            </wixObject>"#;

        const FRAGMENT_WIXOBJ: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
            <wixObject version="3.0.2002.0" xmlns="http://schemas.microsoft.com/wix/2006/objects">
                <section id="*" type="fragment"></section>
            </wixObject>"#;

        const UNKNOWN_WIXOBJ: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
            <wixObject version="3.0.2002.0" xmlns="http://schemas.microsoft.com/wix/2006/objects">
                <section id="*" type="unknown"></section>
            </wixObject>"#;

        const BUNDLE_AND_PRODUCT_WIXOBJ: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
            <wixObject version="3.0.2002.0" xmlns="http://schemas.microsoft.com/wix/2006/objects">
                <section id="*" type="bundle"></section>
                <section id="*" type="product"></section>
            </wixObject>"#;

        const PRODUCT_AND_FRAGMENT_WIXOBJ: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
            <wixObject version="3.0.2002.0" xmlns="http://schemas.microsoft.com/wix/2006/objects">
                <section id="*" type="product"></section>
                <section id="*" type="fragment"></section>
            </wixObject>"#;

        #[test]
        fn try_from_product_object_works() {
            assert_eq!(
                WixObjKind::try_from(PRODUCT_WIXOBJ),
                Ok(WixObjKind::Product)
            );
        }

        #[test]
        fn try_from_bundle_object_works() {
            assert_eq!(WixObjKind::try_from(BUNDLE_WIXOBJ), Ok(WixObjKind::Bundle));
        }

        #[test]
        fn try_from_fragment_object_works() {
            assert_eq!(
                WixObjKind::try_from(FRAGMENT_WIXOBJ),
                Ok(WixObjKind::Fragment)
            );
        }

        #[test]
        fn try_from_bundle_and_product_object_works() {
            assert_eq!(
                WixObjKind::try_from(BUNDLE_AND_PRODUCT_WIXOBJ),
                Ok(WixObjKind::Bundle)
            );
        }

        #[test]
        fn try_from_product_and_fragment_object_works() {
            assert_eq!(
                WixObjKind::try_from(PRODUCT_AND_FRAGMENT_WIXOBJ),
                Ok(WixObjKind::Product)
            );
        }

        #[test]
        #[should_panic]
        fn try_from_unknown_object_fails() {
            WixObjKind::try_from(UNKNOWN_WIXOBJ).unwrap();
        }
    }

    mod installer_kind {
        use super::*;

        #[test]
        fn try_from_wixobj_single_product_works() {
            assert_eq!(
                InstallerKind::try_from(vec![WixObjKind::Product]),
                Ok(InstallerKind::Msi)
            )
        }

        #[test]
        fn try_from_wixobj_single_bundle_works() {
            assert_eq!(
                InstallerKind::try_from(vec![WixObjKind::Bundle]),
                Ok(InstallerKind::Exe)
            )
        }

        #[test]
        #[should_panic]
        fn try_from_wixobj_single_fragment_fails() {
            InstallerKind::try_from(vec![WixObjKind::Fragment]).unwrap();
        }

        #[test]
        #[should_panic]
        fn try_from_wixobj_multiple_fragments_fails() {
            InstallerKind::try_from(vec![
                WixObjKind::Fragment,
                WixObjKind::Fragment,
                WixObjKind::Fragment,
            ])
            .unwrap();
        }

        #[test]
        fn try_from_wixobj_product_and_bundle_works() {
            assert_eq!(
                InstallerKind::try_from(vec![WixObjKind::Product, WixObjKind::Bundle]),
                Ok(InstallerKind::Exe)
            )
        }

        #[test]
        fn try_from_wixobj_multiple_products_and_single_bundle_works() {
            assert_eq!(
                InstallerKind::try_from(vec![
                    WixObjKind::Product,
                    WixObjKind::Product,
                    WixObjKind::Bundle,
                    WixObjKind::Product
                ]),
                Ok(InstallerKind::Exe)
            )
        }

        #[test]
        fn try_from_wixobj_multiple_fragments_and_single_product_works() {
            assert_eq!(
                InstallerKind::try_from(vec![
                    WixObjKind::Fragment,
                    WixObjKind::Fragment,
                    WixObjKind::Product,
                    WixObjKind::Fragment
                ]),
                Ok(InstallerKind::Msi)
            )
        }

        #[test]
        fn try_from_wixobj_multiple_fragments_and_single_bundle_works() {
            assert_eq!(
                InstallerKind::try_from(vec![
                    WixObjKind::Fragment,
                    WixObjKind::Fragment,
                    WixObjKind::Bundle,
                    WixObjKind::Fragment
                ]),
                Ok(InstallerKind::Exe)
            )
        }

        #[test]
        fn try_from_wixobj_multiple_fragments_and_products_works() {
            assert_eq!(
                InstallerKind::try_from(vec![
                    WixObjKind::Fragment,
                    WixObjKind::Fragment,
                    WixObjKind::Product,
                    WixObjKind::Fragment,
                    WixObjKind::Product
                ]),
                Ok(InstallerKind::Msi)
            )
        }

        #[test]
        fn try_from_wixobj_multiple_products_and_bundles_works() {
            assert_eq!(
                InstallerKind::try_from(vec![
                    WixObjKind::Product,
                    WixObjKind::Product,
                    WixObjKind::Bundle,
                    WixObjKind::Product,
                    WixObjKind::Bundle,
                    WixObjKind::Product
                ]),
                Ok(InstallerKind::Exe)
            )
        }

        #[test]
        fn try_from_wixobj_multiple_products_fragments_and_single_bundle_works() {
            assert_eq!(
                InstallerKind::try_from(vec![
                    WixObjKind::Product,
                    WixObjKind::Product,
                    WixObjKind::Bundle,
                    WixObjKind::Fragment,
                    WixObjKind::Fragment,
                    WixObjKind::Product
                ]),
                Ok(InstallerKind::Exe)
            )
        }
    }
}
