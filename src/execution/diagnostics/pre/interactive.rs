//! Detects commands that open interactive sessions and will hang until timeout.
//!
//! Known interactive commands: bare `python`, `node`, `mysql` without `-e`,
//! `psql` without `-c`, `ssh`, `ftp`, `telnet`, `irb`, `ghci`.
//!
//! Also detects `apt-get install` without `-y` (prompts for confirmation).

use super::super::envelope::{Diagnostic, DiagnosticCategory, Severity};
use super::PreCheck;

pub struct InteractiveCheck;

impl PreCheck for InteractiveCheck {
    fn check(&self, command: &str) -> Option<Diagnostic> {
        let trimmed = command.trim();
        let first_token = first_command_token(trimmed);

        match first_token {
            "python" | "python3" => {
                if is_bare_repl(trimmed, first_token) {
                    return Some(repl_warning(
                        first_token,
                        "Use 'python script.py' or 'python -c \"code\"'",
                    ));
                }
            }
            "node" => {
                if is_bare_repl(trimmed, first_token) {
                    return Some(repl_warning(
                        "node",
                        "Use 'node script.js' or 'node -e \"code\"'",
                    ));
                }
            }
            "irb" => {
                if is_bare_repl(trimmed, first_token) {
                    return Some(repl_warning(
                        "irb",
                        "Use 'ruby script.rb' or 'ruby -e \"code\"'",
                    ));
                }
            }
            "ghci" => {
                if is_bare_repl(trimmed, first_token) {
                    return Some(repl_warning("ghci", "Use 'runghc script.hs' instead"));
                }
            }
            "mysql" => {
                if !has_flag(trimmed, &["-e", "--execute"]) {
                    return Some(Diagnostic {
                        severity: Severity::Warning,
                        category: DiagnosticCategory::InteractiveCommand,
                        message: "mysql without -e opens an interactive client that will hang."
                            .to_string(),
                        suggestion: Some("Add -e \"SQL\" to run a query directly.".to_string()),
                    });
                }
            }
            "psql" => {
                if !has_flag(trimmed, &["-c", "--command", "-f", "--file"]) {
                    return Some(Diagnostic {
                        severity: Severity::Warning,
                        category: DiagnosticCategory::InteractiveCommand,
                        message: "psql without -c opens an interactive client that will hang."
                            .to_string(),
                        suggestion: Some("Add -c \"SQL\" to run a query directly.".to_string()),
                    });
                }
            }
            "ssh" | "ftp" | "telnet" => {
                return Some(Diagnostic {
                    severity: Severity::Warning,
                    category: DiagnosticCategory::InteractiveCommand,
                    message: format!(
                        "{} opens an interactive remote session — not supported in this environment.",
                        first_token
                    ),
                    suggestion: None,
                });
            }
            "apt-get" => {
                if trimmed.contains("install") && !has_flag(trimmed, &["-y", "--yes"]) {
                    return Some(Diagnostic {
                        severity: Severity::Info,
                        category: DiagnosticCategory::InteractiveCommand,
                        message: "apt-get install without -y will prompt for confirmation."
                            .to_string(),
                        suggestion: Some("Add -y to auto-confirm.".to_string()),
                    });
                }
            }
            _ => {}
        }

        None
    }
}

/// Extract the first command token (handles pipes, env var prefixes).
fn first_command_token(cmd: &str) -> &str {
    // Skip env var assignments like FOO=bar
    let mut parts = cmd.split_whitespace();
    for part in &mut parts {
        if part.contains('=') && !part.starts_with('-') {
            continue;
        }
        return part;
    }
    ""
}

/// Check if a command is a bare REPL invocation (no script file or -c/-e arg).
fn is_bare_repl(cmd: &str, token: &str) -> bool {
    let rest = cmd[token.len()..].trim();
    // Allow version flags
    if rest.is_empty() || rest == "-V" || rest == "--version" {
        return true;
    }
    false
}

/// Check if the command contains any of the given flags.
fn has_flag(cmd: &str, flags: &[&str]) -> bool {
    cmd.split_whitespace().any(|w| flags.contains(&w))
}

fn repl_warning(name: &str, suggestion: &str) -> Diagnostic {
    Diagnostic {
        severity: Severity::Warning,
        category: DiagnosticCategory::InteractiveCommand,
        message: format!(
            "'{}' without arguments opens an interactive REPL that will hang until timeout.",
            name
        ),
        suggestion: Some(suggestion.to_string()),
    }
}
