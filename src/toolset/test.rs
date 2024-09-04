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

use super::{Toolset, ToolsetAction};
use std::{fmt::Debug, sync::Arc};

/// Type-alias for a boxed test shim
pub type SharedTestShim = Arc<dyn TestShim + 'static>;

/// Struct containing test output for a toolset command action
#[cfg(test)]
#[derive(Clone, Default)]
pub struct ToolsetTest {
    pub success: bool,
    pub stdout: String,
    pub stderr: String,
}

/// Returns an ok status and the provided string as stdout
pub fn ok_stdout(out: impl Into<String>) -> ToolsetTest {
    test::<true>(out, "")
}

/// Returns a failure status and the provided string as stdout
pub fn fail_stdout(out: impl Into<String>) -> ToolsetTest {
    test::<false>(out, "")
}

/// Return a toolset test w/ stdout set
fn test<const SUCCESS: bool>(out: impl Into<String>, err: impl Into<String>) -> ToolsetTest {
    ToolsetTest {
        success: SUCCESS,
        stdout: out.into(),
        stderr: err.into(),
    }
}

/// This shim allows unit test code without requiring dependencies to be installed on the test machine
pub trait TestShim {
    /// Called when `.output()` is called
    fn on_output(&self, action: &ToolsetAction, cmd: &std::process::Command) -> ToolsetTest;
}

/// Boxes a test shim, is_legacy will return false
pub fn toolset(shim: impl TestShim + 'static) -> Toolset {
    Toolset::Test {
        shim: Arc::new(shim),
        is_legacy: false,
    }
}

/// Returns a test toolset with shim, is_legacy will return true
pub fn legacy_toolset(shim: impl TestShim + 'static) -> Toolset {
    Toolset::Test {
        shim: Arc::new(shim),
        is_legacy: true,
    }
}

impl<T> TestShim for T
where
    T: Fn(&ToolsetAction, &std::process::Command) -> ToolsetTest + Clone + 'static,
{
    fn on_output(&self, action: &ToolsetAction, cmd: &std::process::Command) -> ToolsetTest {
        match action {
            ToolsetAction::Compile { .. } => {
                assert!(cmd.get_program().to_string_lossy().to_string().ends_with("candle.exe"));
            }
            ToolsetAction::Convert => {
                assert_eq!("wix", cmd.get_program());
                assert_eq!(["convert"], &cmd.get_args().take(1).collect::<Vec<_>>()[..]);
            }
            ToolsetAction::Build => {
                assert_eq!("wix", cmd.get_program());
                assert_eq!(["build"], &cmd.get_args().take(1).collect::<Vec<_>>()[..]);
            }
            ToolsetAction::AddExtension => {
                assert_eq!("wix", cmd.get_program());
                assert_eq!(
                    ["extension", "add"],
                    &cmd.get_args().take(2).collect::<Vec<_>>()[..]
                );
            }
            ToolsetAction::AddGlobalExtension => {
                assert_eq!("wix", cmd.get_program());
                assert_eq!(
                    ["extension", "add", "--global"],
                    &cmd.get_args().take(3).collect::<Vec<_>>()[..]
                );
            }
            ToolsetAction::ListExtension => {
                assert_eq!("wix", cmd.get_program());
                assert_eq!(
                    ["extension", "list"],
                    &cmd.get_args().collect::<Vec<_>>()[..]
                );
            }
            ToolsetAction::ListGlobalExtension => {
                assert_eq!("wix", cmd.get_program());
                assert_eq!(
                    ["extension", "list", "--global"],
                    &cmd.get_args().collect::<Vec<_>>()[..]
                );
            }
            ToolsetAction::Version => {
                assert_eq!("wix", cmd.get_program());
                assert_eq!(["--version"], &cmd.get_args().collect::<Vec<_>>()[..]);
            }
        }
        self(action, cmd)
    }
}

impl Debug for ToolsetTest {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ToolsetTest")
            .field("success", &self.success)
            .field("stdout", &self.stdout)
            .field("stderr", &self.stderr)
            .finish()
    }
}
