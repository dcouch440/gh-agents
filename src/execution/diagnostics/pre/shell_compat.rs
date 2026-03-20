//! Detects bash-specific syntax that fails under `sh`.
//!
//! Container commands run via `sh -c "..."`, not bash. Bash-only features
//! like `[[`, arrays, and process substitution will fail silently or error.

use super::super::envelope::{Diagnostic, DiagnosticCategory, Severity};
use super::PreCheck;

pub struct ShellCompatCheck;

impl PreCheck for ShellCompatCheck {
    fn check(&self, command: &str) -> Option<Diagnostic> {
        let trimmed = command.trim();

        // [[ ... ]] — bash extended test
        if contains_unquoted(trimmed, "[[") {
            return Some(Diagnostic {
                severity: Severity::Warning,
                category: DiagnosticCategory::ShellCompat,
                message: "[[ ]] is bash-specific. This environment uses sh.".to_string(),
                suggestion: Some("Use [ ] (single brackets) instead.".to_string()),
            });
        }

        // `source` is handled by StatePersistenceCheck with more actionable
        // guidance (chain with &&). Skip here to avoid duplicate diagnostics.

        // Process substitution <(...) or >(...)
        if contains_unquoted(trimmed, "<(") || contains_unquoted(trimmed, ">(") {
            return Some(Diagnostic {
                severity: Severity::Warning,
                category: DiagnosticCategory::ShellCompat,
                message: "Process substitution <() is bash-specific.".to_string(),
                suggestion: Some("Use temporary files or pipes instead.".to_string()),
            });
        }

        // Bash arrays: var=(...)
        if has_bash_array_assignment(trimmed) {
            return Some(Diagnostic {
                severity: Severity::Warning,
                category: DiagnosticCategory::ShellCompat,
                message: "Bash arrays (var=(...)) are not available in sh.".to_string(),
                suggestion: Some("Use space-separated strings with a for loop.".to_string()),
            });
        }

        // Shell functions not available in sh -c
        for cmd in &["nvm use", "nvm install", "rvm use", "conda activate"] {
            if trimmed.starts_with(cmd) {
                return Some(Diagnostic {
                    severity: Severity::Warning,
                    category: DiagnosticCategory::ShellCompat,
                    message: format!(
                        "'{}' is a shell function — not available in sh -c.",
                        cmd.split_whitespace().next().unwrap_or(cmd)
                    ),
                    suggestion: Some(
                        "Use the underlying binary directly or set PATH manually.".to_string(),
                    ),
                });
            }
        }

        None
    }
}

/// Check if a pattern appears outside of single/double quotes.
fn contains_unquoted(cmd: &str, pattern: &str) -> bool {
    let pat_chars: Vec<char> = pattern.chars().collect();
    let cmd_chars: Vec<char> = cmd.chars().collect();
    let pat_len = pat_chars.len();
    let cmd_len = cmd_chars.len();

    if cmd_len < pat_len {
        return false;
    }

    let mut in_single = false;
    let mut in_double = false;
    let mut i = 0;

    while i < cmd_len {
        let c = cmd_chars[i];
        match c {
            '\\' if !in_single && i + 1 < cmd_len => {
                i += 2;
                continue;
            }
            '\'' if !in_double => in_single = !in_single,
            '"' if !in_single => in_double = !in_double,
            _ if !in_single && !in_double => {
                if i + pat_len <= cmd_len && cmd_chars[i..i + pat_len] == pat_chars[..] {
                    return true;
                }
            }
            _ => {}
        }
        i += 1;
    }
    false
}

/// Detect bash-style array assignment: `var=(...)`.
/// Avoids false positives from subshells `$(...)` and `$((..))`.
///
/// The key insight: for `var=(...)`, the char after `=` is `(`.
/// For `var=$(...)`, the char after `=` is `$`, so the `chars[i+1] == '('`
/// guard never fires and we never reach the `$` check.
fn has_bash_array_assignment(cmd: &str) -> bool {
    let chars: Vec<char> = cmd.chars().collect();
    let len = chars.len();
    let mut in_single = false;
    let mut in_double = false;
    let mut i = 0;

    while i < len {
        let c = chars[i];
        match c {
            '\\' if !in_single && i + 1 < len => {
                i += 2;
                continue;
            }
            '\'' if !in_double => in_single = !in_single,
            '"' if !in_single => in_double = !in_double,
            '=' if !in_single && !in_double => {
                // Check: char before is alphanumeric/underscore, char after is (
                if i > 0
                    && (chars[i - 1].is_alphanumeric() || chars[i - 1] == '_')
                    && i + 1 < len
                    && chars[i + 1] == '('
                {
                    return true;
                }
            }
            _ => {}
        }
        i += 1;
    }
    false
}
