use log::debug;

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
    pub shim: Option<SharedTestShim>,
}

pub fn ok_stdout(out: impl Into<String>) -> ToolsetTest {
    test::<true>(out, "")
}

pub fn fail_stdout(out: impl Into<String>) -> ToolsetTest {
    test::<false>(out, "")
}

/// Return a toolset test w/ stdout set
fn test<const SUCCESS: bool>(out: impl Into<String>, err: impl Into<String>) -> ToolsetTest {
    ToolsetTest {
        success: SUCCESS,
        stdout: out.into(),
        stderr: err.into(),
        shim: None,
    }
}

/// This shim allows unit test code without requiring dependencies to be installed on the test machine
pub trait TestShim {
    /// Intercept a command execution to return a toolset test based on a specific action
    ///
    /// Allows for logic that has multiple command executions using the same toolset
    fn on_command(&self, action: &ToolsetAction) -> ToolsetTest;

    fn on_output(&self, cmd: &std::process::Command);
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
    T: Fn(&ToolsetAction) -> ToolsetTest + Clone + 'static,
{
    fn on_command(&self, action: &ToolsetAction) -> ToolsetTest {
        let mut test = self(action);
        test.shim = Some(Arc::new(self.clone()));
        test
    }

    fn on_output(&self, _: &std::process::Command) {}
}

impl<T, C> TestShim for (T, C)
where
    T: Fn(&ToolsetAction) -> ToolsetTest + Clone + 'static,
    C: Fn() -> std::process::Command + Clone + 'static
{
    fn on_command(&self, action: &ToolsetAction) -> ToolsetTest {
        let mut test = self.0(action);
        test.shim = Some(Arc::new(self.clone()));
        test
    }

    fn on_output(&self, cmd: &std::process::Command) {
        let expected = self.1();
        debug!("Comparing {} == {:?}", format!("{:?}", cmd), expected);
        assert_eq!(format!("{:?}", cmd), format!("{:?}", expected))
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