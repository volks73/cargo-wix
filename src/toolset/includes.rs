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

use std::path::PathBuf;
use cargo_metadata::Package;
use log::{debug, trace};
use crate::{Error, WIX, WIX_SOURCE_FILE_EXTENSION};

use super::Project;

/// Trait for returning a list of project files to include
pub trait Includes {
    /// Returns included file paths
    fn includes(&self) -> Option<&Vec<PathBuf>>;

    /// Returns a list of paths to *.wxs sources
    fn wxs_sources(&self, package: &Package) -> crate::Result<Vec<PathBuf>> {
        let project_wix_dir = package
            .manifest_path
            .parent()
            .ok_or_else(|| {
                Error::Generic(format!(
                    "The '{}' path for the package's manifest file is invalid",
                    package.manifest_path
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
        if let Some(paths) = self.includes() {
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
}

/// Extension functions for implementations of Include
pub trait IncludesExt : Includes {
    /// Derives a project from a Package and the current `impl Includes`
    /// 
    /// Returns an error if the *.wxs sources cannot be enumerated, if
    /// a modern WiX toolset is not installed and available from PATH, or
    /// if a .wxs file has invalid XML
    fn create_project(&self, package: &Package) -> crate::Result<Project> {
        debug!("Getting wxs files");
        let wxs_sources = self.wxs_sources(package)?;
        debug!("wxs_sources = {wxs_sources:#?}");

        debug!("Trying to create new project");
        let mut project = Project::try_new(crate::toolset::Toolset::Modern)?;
        debug!("project = {project:#?}");

        for src in wxs_sources {
            debug!("Adding {src:?} to project");
            project.add_wxs(src)?;
        }

        debug!("Completed project = {project:#?}");
        Ok(project)
    }
}

impl<I: Includes> IncludesExt for I {}