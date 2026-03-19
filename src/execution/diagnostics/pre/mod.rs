//! Pre-execution checks — analyze the command string before running it.
//!
//! Each check implements [`PreCheck`] and returns an optional diagnostic
//! when a known anti-pattern is detected.

pub mod interactive;
pub mod shell_compat;
pub mod state_persistence;

mod tests;

use super::envelope::Diagnostic;

/// A pre-execution check that analyzes the command string.
pub trait PreCheck: Send + Sync {
    /// Analyze the command and return a diagnostic if a problem is detected.
    fn check(&self, command: &str) -> Option<Diagnostic>;
}

/// Run all pre-checks against a command, collecting diagnostics.
pub fn run_pre_checks(checks: &[Box<dyn PreCheck>], command: &str) -> Vec<Diagnostic> {
    checks.iter().filter_map(|c| c.check(command)).collect()
}
