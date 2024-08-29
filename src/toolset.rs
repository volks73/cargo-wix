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

pub use includes::Includes;
pub use includes::IncludesExt;
use log::debug;
pub use project::Project;

use clap::ValueEnum;
use std::process::Command;

/// Enumeration of wix-toolset options
///
/// This controls which wix build binaries are used by cargo-wix
#[derive(ValueEnum, Copy, Clone, Debug)]
pub enum Toolset {
    /// The default wix toolset uses "candle.exe" and "light.exe" to build the installer
    Legacy,
    /// Modern wix toolsets use just "wix.exe" to build the installer
    Modern,
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

impl Toolset {
    /// Returns true if the toolset in use is legacy
    pub fn is_legacy(&self) -> bool {
        matches!(self, Toolset::Legacy)
    }

    /// Returns true if the toolset in use is modern
    pub fn is_modern(&self) -> bool {
        matches!(self, Toolset::Modern)
    }

    /// Returns an error if the modern toolset is not found from PATH
    pub fn check_modern_toolset_installed() -> crate::Result<()> {
        let output = Command::new("wix").arg("--help").output()?;
        if !output.status.success() {
            Err("Modern toolset (wix.exe) could not be found from PATH, ensure that Wix4 or above is installed".into())
        } else {
            Ok(())
        }
    }
}

impl ToolsetSetupMode {
    /// Applies setup operations to a project
    pub fn setup(self, mut project: Project) -> crate::Result<()> {
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
