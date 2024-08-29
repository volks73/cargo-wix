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

//! The implementation for the `setup` command. The default behavior will use the
//! "Project" setup mode, which will update any *.wxs source files to the modern format as well
//! as install the extension packages to the global cache.
//!
//! Specific information about the different toolset modes can be found in the documentation for
//! `ToolsetSetupMode`.

use crate::toolset::*;
use log::debug;
use std::path::PathBuf;

/// A builder for running the `cargo wix setup` subcommand.
#[derive(Debug, Clone)]
pub struct Builder<'a> {
    input: Option<&'a str>,
    package: Option<&'a str>,
    includes: Option<Vec<&'a str>>,
    restore_only: bool,
    upgrade_only: bool,
    sxs: bool,
    vendor: bool,
}

impl<'a> Builder<'a> {
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

    /// Sets the package.
    ///
    /// If the project is organized using a workspace, this selects the package
    /// by name to create an installer. If a workspace is not used, then this
    /// has no effect.
    pub fn package(&mut self, p: Option<&'a str>) -> &mut Self {
        self.package = p;
        self
    }

    /// Will only upgrade any legacy *.wxs files
    pub fn upgrade_only(&mut self, upgrade: bool) -> &mut Self {
        self.upgrade_only = upgrade;
        self
    }

    /// Will only restore any missing packages, and will not try to convert any source files.
    ///
    /// However, if source files are not upgraded to the modern format dependencies cannot be detected.
    pub fn restore_only(&mut self, restore: bool) -> &mut Self {
        self.restore_only = restore;
        self
    }

    /// Will enable side by side mode
    ///
    /// Can be used with `--upgrade-only` but is ignored if combined with `--restore-only`, otherwise if neither
    /// flag is used will apply both upgrade and restore on the project
    pub fn sxs(&mut self, enable_side_by_side: bool) -> &mut Self {
        self.sxs = enable_side_by_side;
        self
    }

    /// Will enable vendor mode
    ///
    /// Can be used with `--restore-only` but will be ignored if `--upgrade-only` is applied, otherwise if neither
    /// flag is used will apply both upgrade and restore on the project
    pub fn vendor(&mut self, vendored: bool) -> &mut Self {
        self.vendor = vendored;
        self
    }

    /// Consumes the builder and returns the upgrade execution context
    ///
    /// Will resolve toolset setup mode from the provided flags
    pub fn build(self) -> crate::Result<Execution> {
        Ok(Execution {
            input: self.input.map(PathBuf::from),
            package: self.package.map(String::from),
            includes: self
                .includes
                .as_ref()
                .map(|v| v.iter().map(&PathBuf::from).collect()),
            toolset_setup_mode: if self.restore_only {
                if self.vendor {
                    ToolsetSetupMode::RestoreVendorOnly
                } else {
                    ToolsetSetupMode::RestoreOnly
                }
            } else if self.upgrade_only {
                if self.sxs {
                    ToolsetSetupMode::UpgradeSideBySideOnly
                } else {
                    ToolsetSetupMode::UpgradeOnly
                }
            } else if self.sxs {
                ToolsetSetupMode::SideBySide
            } else if self.vendor {
                ToolsetSetupMode::Vendor
            } else {
                ToolsetSetupMode::Project
            },
        })
    }
}

/// A context for setting up a WiX project
#[derive(Debug)]
pub struct Execution {
    input: Option<PathBuf>,
    package: Option<String>,
    includes: Option<Vec<PathBuf>>,
    toolset_setup_mode: ToolsetSetupMode,
}

impl Execution {
    /// Consumes the execution context and performs the setup
    pub fn run(self) -> crate::Result<()> {
        debug!("self.input = {:?}", self.input);
        debug!("self.package = {:?}", self.package);
        debug!("self.includes = {:?}", self.includes);
        debug!("self.toolset_upgrade = {:?}", self.toolset_setup_mode);

        debug!("Resolving manifest");
        let manifest = crate::manifest(self.input.as_ref())?;
        debug!("{manifest:?}");
        debug!("Resolving package");
        let package = crate::package(&manifest, self.package.as_deref())?;
        debug!("{package:?}");

        debug!("Evaluating project and beginning setup");
        let project = self.create_project(&package)?;
        self.toolset_setup_mode.setup(project)?;
        Ok(())
    }
}

impl Includes for Execution {
    fn includes(&self) -> Option<&Vec<PathBuf>> {
        self.includes.as_ref()
    }
}
