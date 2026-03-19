//! Post-execution checks — analyze command results and filesystem changes.

pub mod noop;
pub mod stderr_classifier;
pub mod suggestions;
pub mod truncation;

mod tests;

use crate::execution::ContainerExecResult;

use super::envelope::Diagnostic;
use super::types::FileChange;

/// A post-execution check that analyzes the command result.
pub trait PostCheck: Send + Sync {
    /// Analyze the command, its result, and filesystem changes.
    fn check(
        &self,
        command: &str,
        result: &ContainerExecResult,
        changes: &[FileChange],
    ) -> Vec<Diagnostic>;
}

/// Run all post-checks, collecting diagnostics.
pub fn run_post_checks(
    checks: &[Box<dyn PostCheck>],
    command: &str,
    result: &ContainerExecResult,
    changes: &[FileChange],
) -> Vec<Diagnostic> {
    checks
        .iter()
        .flat_map(|c| c.check(command, result, changes))
        .collect()
}
