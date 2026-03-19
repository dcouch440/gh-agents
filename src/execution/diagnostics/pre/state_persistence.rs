//! Detects standalone state-setting commands that won't persist between
//! `docker exec` invocations: `cd`, `export`, `alias`, `source`/`.`.
//!
//! "Standalone" means the command is the entire input — not chained with
//! `&&` or `;`. `cd /app && make` is fine; `cd /app` alone is a no-op.

use super::super::envelope::{Diagnostic, DiagnosticCategory, Severity};
use super::PreCheck;

pub struct StatePersistenceCheck;

impl PreCheck for StatePersistenceCheck {
    fn check(&self, command: &str) -> Option<Diagnostic> {
        let trimmed = command.trim();

        if is_standalone_cd(trimmed) {
            return Some(Diagnostic {
                severity: Severity::Warning,
                category: DiagnosticCategory::StatePersistence,
                message: "cd doesn't persist between commands. Each command runs in a fresh shell."
                    .to_string(),
                suggestion: Some(
                    "Chain with &&: cd /path && your_command, or use absolute paths.".to_string(),
                ),
            });
        }

        if is_standalone_export(trimmed) {
            return Some(Diagnostic {
                severity: Severity::Warning,
                category: DiagnosticCategory::StatePersistence,
                message:
                    "Environment variables set with 'export' don't persist between commands."
                        .to_string(),
                suggestion: Some(
                    "Use inline: VAR=value command, or write to a .env file and source it per command."
                        .to_string(),
                ),
            });
        }

        if is_standalone_alias(trimmed) {
            return Some(Diagnostic {
                severity: Severity::Warning,
                category: DiagnosticCategory::StatePersistence,
                message: "Aliases don't persist between commands.".to_string(),
                suggestion: Some(
                    "Define the alias inline or use a shell function file.".to_string(),
                ),
            });
        }

        if is_standalone_source(trimmed) {
            return Some(Diagnostic {
                severity: Severity::Info,
                category: DiagnosticCategory::StatePersistence,
                message: "source/. only affects this command's shell — state won't carry over."
                    .to_string(),
                suggestion: Some("Chain with &&: source file && your_command".to_string()),
            });
        }

        None
    }
}

/// Check if the command is a standalone `cd` (not chained).
fn is_standalone_cd(cmd: &str) -> bool {
    // Must start with "cd " or be exactly "cd"
    if !cmd.starts_with("cd ") && cmd != "cd" {
        return false;
    }
    // If chained with && or ;, it's fine
    !contains_chain_operator(cmd)
}

/// Check if the command is a standalone `export`.
fn is_standalone_export(cmd: &str) -> bool {
    if !cmd.starts_with("export ") {
        return false;
    }
    !contains_chain_operator(cmd)
}

/// Check if the command is a standalone `alias`.
fn is_standalone_alias(cmd: &str) -> bool {
    if !cmd.starts_with("alias ") {
        return false;
    }
    !contains_chain_operator(cmd)
}

/// Check if the command is a standalone `source` or `.` (dot-source).
fn is_standalone_source(cmd: &str) -> bool {
    let is_source = cmd.starts_with("source ") || cmd.starts_with(". ");
    if !is_source {
        return false;
    }
    !contains_chain_operator(cmd)
}

/// Returns true if the command contains `&&` or `;` outside of quotes.
fn contains_chain_operator(cmd: &str) -> bool {
    let mut in_single_quote = false;
    let mut in_double_quote = false;
    let chars: Vec<char> = cmd.chars().collect();
    let len = chars.len();

    for i in 0..len {
        let c = chars[i];
        match c {
            '\'' if !in_double_quote => in_single_quote = !in_single_quote,
            '"' if !in_single_quote => in_double_quote = !in_double_quote,
            '&' if !in_single_quote && !in_double_quote => {
                if i + 1 < len && chars[i + 1] == '&' {
                    return true;
                }
            }
            ';' if !in_single_quote && !in_double_quote => return true,
            _ => {}
        }
    }
    false
}
