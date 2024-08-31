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

//! # Toolset Module
//!
//! Contains types for integrating legacy and modern WiX toolsets and facilitating various
//! project setup utilities such as converting from legacy to modern, restoring extension dependencies, or
//! automatically determining if extension dependencies that are required are currently missing from the environment.

pub mod ext;
mod includes;
pub mod project;
pub mod source;
#[cfg(test)]
pub(crate) mod test;
pub use includes::Includes;
pub use includes::IncludesExt;
pub use project::Project;

use crate::BINARY_FOLDER_NAME;
use crate::EXE_FILE_EXTENSION;
use crate::WIX_COMPILER;
use crate::WIX_PATH_KEY;
use clap::ValueEnum;
use log::debug;
use log::trace;
use std::ops::Deref;
use std::ops::DerefMut;
use std::path::PathBuf;
use std::process::Command;
use std::process::Output;
use std::str::FromStr;

/// Enumeration of wix-toolset options
///
/// This controls which wix build binaries are used by cargo-wix
#[derive(ValueEnum, Clone)]
pub enum Toolset {
    /// The default wix toolset uses "candle.exe" and "light.exe" to build the installer
    Legacy,
    /// Modern wix toolsets use just "wix.exe" to build the installer
    Modern,
    /// Test toolkit, used to test code without needing any toolkit installed
    #[cfg(test)]
    #[clap(skip)]
    Test {
        is_legacy: bool,
        shim: test::SharedTestShim,
    },
}

impl std::fmt::Debug for Toolset {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Legacy => write!(f, "Legacy"),
            Self::Modern => write!(f, "Modern"),
            #[cfg(test)]
            Self::Test { is_legacy: allow_legacy, .. } => f.debug_tuple("Test").field(allow_legacy).finish(),
        }
    }
}

/// Wrapper over std::process::Command
///
/// Allows for checking args state and centralizes debug logging logic
#[derive(Debug)]
pub struct ToolsetCommand {
    pub(crate) inner: Command,
    action: ToolsetAction,
    #[cfg(test)]
    test: test::ToolsetTest,
}

/// Enumeration of toolset actions used by this module
#[derive(Default, Debug)]
pub enum ToolsetAction {
    /// `candle`
    Compile { bin_path: Option<PathBuf> },
    /// `wix convert`
    Convert,
    /// `wix build`
    Build,
    /// `wix extension add`
    AddExtension,
    /// `wix extension add --global`
    AddGlobalExtension,
    /// `wix extension list`
    ListExtension,
    /// `wix extension list --global`
    ListGlobalExtension,
    /// `wix --version`
    #[default]
    Version,
}

/// Enumeration of wix-toolset project setup patterns that can be applied
///
/// WiX toolset project setup consists of two major-operation, **upgrade** or **restore**.
///
/// - **upgrade**: Convert Wix3 and below *.wxs files to the modern WiX format. Technically only needs to be executed once,
///                but the setup will try and do the smart thing and upgrade on an as-needed basis.
/// - **restore**: Detect and install wix extensions required by *.wxs files. Requires all source files to be upgraded.
#[derive(ValueEnum, Copy, Clone, Debug, Default)]
pub enum ToolsetSetupMode {
    /// Do not apply any setup logic. (Default)
    #[default]
    None,
    /// Upgrade source files in place and install extensions to the global extension cache
    ///
    /// This option is suited to simple one file installer projects, where build pipeline network access will not be
    /// an issue and wix will likely be installed on a dev machine or during the build pipeline on demand.
    #[clap(name = "project")]
    Project,
    /// Setup project in "vendored" mode.
    ///
    ///
    /// Upgrade source files in place and install extensions to the current directory
    ///
    /// This option is suited to tracking changes with source version control systems such as git, and also when
    /// the build will happen in an offline environment
    #[clap(name = "vendor")]
    Vendor,
    /// Upgrade files side by side and install extensions to the sxs folder
    ///
    /// This will copy *.wxs files to a versioned folder with the format `wix{maj}` where `{maj}` is the major version value of wix
    ///
    /// This option is suited to tracking changes with source version control systems such as git with
    /// concise control when migrating to major WiX toolsets. For example, if time to bake is required when
    /// moving to wix toolsets, versioning in this manner seperates *.wxs and associated extensions into seperate folders
    /// so that they are unable to cross contaminate
    #[clap(name = "sxs")]
    SideBySide,
    /// Only Restore any missing extensions to the global WiX extension cache
    ///
    /// This option is suited for build pipelines where network access is not an issue when building
    /// the installer
    #[clap(name = "restore")]
    RestoreOnly,
    /// Only Restore any missing extensions to the current directory
    ///
    /// This option is suited for setting up build pipelines where network access is unavailable, and
    /// offline building is required
    #[clap(name = "restore-vendor")]
    RestoreVendorOnly,
    /// Upgrade files in place
    ///
    /// Upgrades all *.wxs files that are in the legacy format to the modern format. Only required to be executed once assuming
    /// all included *.wxs files remain constant
    ///
    /// This option is suited for debugging upgrades of simple legacy WiX projects
    #[clap(name = "upgrade")]
    UpgradeOnly,
    /// Upgrade source files in a SxS manner
    ///
    /// This will copy *.wxs files to a versioned folder with the format `wix{maj}` where `{maj}` is the major version value of wix
    ///
    /// Upgrades all *.wxs files that are in the legacy format to the modern format. Only required to be executed once assuming
    /// all included *.wxs files remain constant
    ///
    /// This option is suited for debugging upgrades of complex legacy WiX projects
    #[clap(name = "upgrade-sxs")]
    UpgradeSideBySideOnly,
}

#[cfg(test)]
impl Toolset {
    /// Returns true if the toolset in use is legacy
    pub fn is_legacy(&self) -> bool {
        matches!(self, Toolset::Test { is_legacy: true, .. } | Toolset::Legacy)
    }

    pub fn is_modern(&self) -> bool {
        matches!(self, Toolset::Test { is_legacy: false, .. } | Toolset::Modern)
    }
}

impl Toolset {
    /// Returns true if the toolset in use is legacy
    #[cfg(not(test))]
    pub fn is_legacy(&self) -> bool {
        matches!(self, Toolset::Legacy)
    }

    /// Returns true if the toolset in use is modern
    #[cfg(not(test))]
    pub fn is_modern(&self) -> bool {
        matches!(self, Toolset::Modern)
    }

    /// Returns a new ToolsetCommand for `wix.exe`
    pub fn wix(&self, action: impl Into<ToolsetAction>) -> crate::Result<ToolsetCommand> {
        if self.is_modern() {
            #[cfg(test)]
            let mut command = ToolsetCommand::try_from(action.into())?;

            #[cfg(test)]
            if let Toolset::Test { shim, .. } = self {
                command.test = shim.on_command(&command.action);
            }

            #[cfg(not(test))]
            let command = ToolsetCommand::try_from(action.into())?;
            Ok(command)
        } else {
            Err("Cannot use modern wix commands with legacy toolset".into())
        }
    }

    /// Returns a new ToolsetCommand for `candle.exe`
    pub fn compiler(&self, bin_path: Option<PathBuf>) -> crate::Result<ToolsetCommand> {
        if self.is_legacy() {
            #[cfg(test)]
            let mut command: ToolsetCommand = ToolsetAction::Compile { bin_path }.try_into()?;
            
            #[cfg(test)]
            if let Toolset::Test { shim, .. } = self {
                command.test = shim.on_command(&command.action);
            }

            #[cfg(not(test))]
            let command: ToolsetCommand = ToolsetAction::Compile { bin_path }.try_into()?;
            Ok(command)
        } else {
            Err("Cannot use legacy wix commands with modern toolset".into())
        }
    }
}

impl ToolsetSetupMode {
    /// Applies migration setup operation on the Project
    pub fn migrate(self, mut project: Project) -> crate::Result<()> {
        match self {
            ToolsetSetupMode::None => {}
            _ => {
                debug!("Starting toolset upgrade checks");
                let work_dir = if self.use_sxs() {
                    let current = std::env::current_dir()?;
                    let sxs_folder = current.join(project.sxs_folder_name());
                    debug!("Using SxS folder as work_dir: {sxs_folder:?}");
                    std::fs::create_dir_all(&sxs_folder)?;
                    Some(sxs_folder)
                } else {
                    None
                };

                debug!("Starting project upgrade");
                if self.can_upgrade() {
                    project.upgrade(work_dir.as_ref())?;
                }

                debug!("Restoring any missing extension packages");
                if self.can_restore() {
                    project.restore(self.use_global(), work_dir.as_ref())?;
                }
            }
        }
        Ok(())
    }

    /// Returns true if setup is enabled
    pub fn is_enabled(&self) -> bool {
        !matches!(self, ToolsetSetupMode::None)
    }

    /// Returns true if restore is allowed according to the toolset mode
    fn can_restore(&self) -> bool {
        matches!(
            self,
            ToolsetSetupMode::RestoreOnly
                | ToolsetSetupMode::Project
                | ToolsetSetupMode::Vendor
                | ToolsetSetupMode::SideBySide
        )
    }

    /// Returns true if convert is allowed according to the toolset mode
    fn can_upgrade(&self) -> bool {
        matches!(
            self,
            ToolsetSetupMode::UpgradeOnly
                | ToolsetSetupMode::UpgradeSideBySideOnly
                | ToolsetSetupMode::Project
                | ToolsetSetupMode::Vendor
                | ToolsetSetupMode::SideBySide
        )
    }

    /// Returns true if the global extension package cache should be used
    fn use_global(&self) -> bool {
        matches!(
            self,
            ToolsetSetupMode::Project | ToolsetSetupMode::RestoreOnly
        )
    }

    /// Returns true if SxS mode should be used
    fn use_sxs(&self) -> bool {
        matches!(
            self,
            ToolsetSetupMode::UpgradeSideBySideOnly | ToolsetSetupMode::SideBySide
        )
    }
}

impl ToolsetCommand {
    /// Consumes this reference and returns the inner std::process::Command and configured toolset action
    pub fn split_into_std(self) -> (std::process::Command, ToolsetAction) {
        (self.inner, self.action)
    }

    /// Consumes the toolset command and returns the output
    #[allow(unused_mut)] // For test shim
    pub fn output(mut self) -> crate::Result<Output> {
        debug!("command.action={:?}", self.action);
        #[cfg(not(test))]
        let output = self.inner.output()?;

        #[cfg(test)]
        let output = self.test_output();

        if output.status.success() {
            if log::log_enabled!(log::Level::Debug) && !output.stderr.is_empty() {
                let std_err = String::from_utf8(output.stderr.clone())?;
                for line in std_err.lines() {
                    debug!("{line}");
                }
            }
            Ok(output)
        } else {
            Err(match self.action {
                ToolsetAction::Convert => "(wix.exe) Could not convert wxs file",
                ToolsetAction::Build => "(wix.exe) Could not build installer",
                ToolsetAction::AddExtension => "(wix.exe) Could not add extension package to local cache",
                ToolsetAction::AddGlobalExtension => {
                    "(wix.exe) Could not add extension package to global cache"
                }
                ToolsetAction::ListExtension => {
                    "(wix.exe) Could not list installed extensions from local cache"
                }
                ToolsetAction::ListGlobalExtension => {
                    "(wix.exe) Could not list installed extensions from global cache"
                }
                ToolsetAction::Version => {
                    "(wix.exe) command was not found from PATH env, ensure a WiX4+ toolset is installed"
                }
                ToolsetAction::Compile { .. } => "(candle.exe) Could not compile wxs files",
            }
            .into())
        }
    }

    /// Test shim for converting test parameters into an output
    #[cfg(test)]
    fn test_output(&self) -> Output {
        #[cfg(windows)]
        use std::os::windows::process::ExitStatusExt;

        #[cfg(unix)]
        use std::os::unix::process::ExitStatusExt;

        let status = if self.test.success {
            std::process::ExitStatus::from_raw(0)
        } else {
            std::process::ExitStatus::from_raw(1)
        };

        if let Some(shim) = self.test.shim.as_ref() {
            shim.on_output(&self.inner);
        }

        Output {
            status,
            stdout: self.test.stdout.clone().into_bytes(),
            stderr: self.test.stderr.clone().into_bytes(),
        }
    }
}

impl Default for ToolsetCommand {
    fn default() -> Self {
        Self {
            inner: Command::new("wix"),
            action: ToolsetAction::Version,
            #[cfg(test)]
            test: test::ToolsetTest::default(),
        }
    }
}

impl From<std::process::Command> for ToolsetCommand {
    fn from(value: std::process::Command) -> Self {
        let action = ToolsetAction::from(value.get_program().to_string_lossy().as_ref());

        ToolsetCommand {
            inner: value,
            action,
            #[cfg(test)]
            test: test::ToolsetTest::default(),
        }
    }
}

impl<'a> FromStr for ToolsetCommand {
    type Err = crate::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        ToolsetAction::from(s).try_into()
    }
}

impl<'a> From<&'a str> for ToolsetAction {
    fn from(s: &'a str) -> Self {
        match s {
            "candle.exe" | "candle" | "compile" => ToolsetAction::Compile { bin_path: None },
            "convert" => ToolsetAction::Convert,
            "build" => ToolsetAction::Build,
            "extension add" => ToolsetAction::AddExtension,
            "extension list" => ToolsetAction::ListExtension,
            "extension add --global" => ToolsetAction::AddGlobalExtension,
            "extension list --global" => ToolsetAction::ListGlobalExtension,
            "--version" | _ => ToolsetAction::Version,
        }
    }
}

impl TryFrom<ToolsetAction> for ToolsetCommand {
    type Error = crate::Error;

    fn try_from(value: ToolsetAction) -> crate::Result<Self> {
        let mut command = ToolsetCommand::default();
        match value {
            ToolsetAction::Convert => {
                command.action = value;
                command.arg("convert");
                Ok(command)
            }
            ToolsetAction::Build => {
                command.action = value;
                command.arg("build");
                Ok(command)
            }
            ToolsetAction::AddExtension => {
                command.action = value;
                command.args(["extension", "add"]);
                Ok(command)
            }
            ToolsetAction::ListExtension => {
                command.action = value;
                command.args(["extension", "list"]);
                Ok(command)
            }
            ToolsetAction::AddGlobalExtension => {
                command.action = value;
                command.args(["extension", "list", "--global"]);
                Ok(command)
            }
            ToolsetAction::ListGlobalExtension => {
                command.action = value;
                command.args(["extension", "add", "--global"]);
                Ok(command)
            }
            ToolsetAction::Version => {
                command.action = value;
                command.arg("--version");
                Ok(command)
            }
            ToolsetAction::Compile { bin_path } => {
                if let Some(mut path) = bin_path.as_ref().map(|s| {
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
                        Err(crate::Error::Generic(format!(
                            "The compiler application ('{}') does not exist at the '{}' path specified via \
                            the '-b,--bin-path' command line argument. Please check the path is correct and \
                            the compiler application exists at the path.",
                            WIX_COMPILER,
                            path.display()
                        )))
                    } else {
                        Ok(ToolsetCommand {
                            inner: Command::new(path),
                            action: ToolsetAction::Compile { bin_path },
                            #[cfg(test)]
                            test: test::ToolsetTest::default(),
                        })
                    }
                } else if let Some(mut path) = std::env::var_os(WIX_PATH_KEY).map(|s| {
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
                        Err(crate::Error::Generic(format!(
                            "The compiler application ('{}') does not exist at the '{}' path specified \
                             via the {} environment variable. Please check the path is correct and the \
                             compiler application exists at the path.",
                            WIX_COMPILER,
                            path.display(),
                            WIX_PATH_KEY
                        )))
                    } else {
                        Ok(ToolsetCommand {
                            inner: Command::new(path),
                            action: ToolsetAction::Compile { bin_path: None },
                            #[cfg(test)]
                            test: test::ToolsetTest::default(),
                        })
                    }
                } else {
                    Ok(ToolsetCommand {
                        inner: Command::new(WIX_COMPILER),
                        action: ToolsetAction::Compile { bin_path: None },
                        #[cfg(test)]
                        test: test::ToolsetTest::default(),
                    })
                }
            }
        }
    }
}

impl Deref for ToolsetCommand {
    type Target = std::process::Command;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl DerefMut for ToolsetCommand {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.inner
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::test;
    use super::ToolsetAction;

    #[test]
    #[cfg(windows)]
    fn test_toolset_compiler_is_correct_with_defaults() {
        use std::{path::PathBuf, process::Command};

        use crate::{
            toolset::{Toolset, ToolsetAction},
            BINARY_FOLDER_NAME, EXE_FILE_EXTENSION, WIX_COMPILER, WIX_PATH_KEY,
        };

        let expected = Command::new(
            std::env::var_os(WIX_PATH_KEY)
                .map(|s| {
                    let mut p = PathBuf::from(s);
                    p.push(BINARY_FOLDER_NAME);
                    p.push(WIX_COMPILER);
                    p.set_extension(EXE_FILE_EXTENSION);
                    p
                })
                .unwrap(),
        );
        let e = Toolset::Legacy.compiler(None).unwrap();
        let (actual, action) = e.split_into_std();
        assert_eq!(format!("{actual:?}"), format!("{expected:?}"));
        assert!(
            matches!(action, ToolsetAction::Compile { bin_path: None }),
            "{action:?}"
        );
    }

    #[test]
    fn test_toolset_wix_action() {
        let toolset = super::test::toolset(|action: &ToolsetAction| match action {
            ToolsetAction::Compile { .. } => test::fail_stdout("out"),
            ToolsetAction::Convert => test::ok_stdout("converted"),
            ToolsetAction::Build => test::ok_stdout("built"),
            ToolsetAction::AddExtension => test::ok_stdout("added"),
            ToolsetAction::AddGlobalExtension => test::ok_stdout("added_global"),
            ToolsetAction::ListExtension => test::ok_stdout("list_ext"),
            ToolsetAction::ListGlobalExtension => test::ok_stdout("list_global_ext"),
            ToolsetAction::Version => test::ok_stdout("version"),
        });

        let output = toolset.wix("convert").unwrap();
        assert_eq!(&b"converted"[..], &output.output().unwrap().stdout[..]);

        let output = toolset.wix("build").unwrap();
        assert_eq!(&b"built"[..], &output.output().unwrap().stdout[..]);

        let output = toolset.wix("extension add").unwrap();
        assert_eq!(&b"added"[..], &output.output().unwrap().stdout[..]);

        let output = toolset.wix("extension add --global").unwrap();
        assert_eq!(&b"added_global"[..], &output.output().unwrap().stdout[..]);

        let output = toolset.wix("extension list").unwrap();
        assert_eq!(&b"list_ext"[..], &output.output().unwrap().stdout[..]);

        let output = toolset.wix("extension list --global").unwrap();
        assert_eq!(
            &b"list_global_ext"[..],
            &output.output().unwrap().stdout[..]
        );

        let output = toolset.wix("--version").unwrap();
        assert_eq!(&b"version"[..], &output.output().unwrap().stdout[..]);

        let output = toolset.wix("--version").unwrap();
        assert_eq!(&b"version"[..], &output.output().unwrap().stdout[..]);

        toolset.compiler(None).expect_err("should not be allowed to create a compiler command if toolset is not legacy");
    }

    #[test]
    fn test_toolset_legacy_action() {
        let toolset = super::test::legacy_toolset(|action: &ToolsetAction| match action {
            ToolsetAction::Compile { bin_path: None } => test::ok_stdout("compile_bin_path_none"),
            ToolsetAction::Compile { bin_path: Some(..) } => unreachable!("should fail before this can be executed"),
            _ => {
                panic!("should not execute non-legacy actions")
            }
        });

        let output = toolset.compiler(None).unwrap();
        assert_eq!(&b"compile_bin_path_none"[..], &output.output().unwrap().stdout[..]);

        toolset.compiler(Some(PathBuf::new())).expect_err("should not have candle in an empty dir");
    }
}
