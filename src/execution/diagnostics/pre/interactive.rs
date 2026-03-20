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
        let (first_token, rest) = first_command_token(trimmed);

        match first_token {
            "python" | "python3" => {
                if is_bare_repl(rest) {
                    return Some(repl_warning(
                        first_token,
                        "Use 'python script.py' or 'python -c \"code\"'",
                    ));
                }
            }
            "node" => {
                if is_bare_repl(rest) {
                    return Some(repl_warning(
                        "node",
                        "Use 'node script.js' or 'node -e \"code\"'",
                    ));
                }
            }
            "irb" => {
                if is_bare_repl(rest) {
                    return Some(repl_warning(
                        "irb",
                        "Use 'ruby script.rb' or 'ruby -e \"code\"'",
                    ));
                }
            }
            "ghci" => {
                if is_bare_repl(rest) {
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

/// Known command wrappers that take arguments before the real command.
const WRAPPER_COMMANDS: &[&str] = &["timeout", "env", "nice", "nohup", "sudo", "strace", "time"];

/// Extract the first command token and the remaining arguments after it.
///
/// Skips env var assignments (`FOO=bar`) and known wrapper commands
/// (`timeout`, `sudo`, etc.) to find the real command.
/// Returns `(command_token, rest_of_args)`.
fn first_command_token(cmd: &str) -> (&str, &str) {
    let mut parts = cmd.split_whitespace();
    while let Some(part) = parts.next() {
        // Skip env var assignments like FOO=bar
        if part.contains('=') && !part.starts_with('-') {
            continue;
        }
        // Skip wrapper commands and their flags, numeric args, and env assignments
        if WRAPPER_COMMANDS.contains(&part) {
            for arg in parts.by_ref() {
                if arg.starts_with('-')
                    || arg.parse::<f64>().is_ok()
                    || (arg.contains('=') && !arg.starts_with('-'))
                {
                    continue;
                }
                // Found the real command — rest is everything after it
                let arg_end = arg.as_ptr() as usize - cmd.as_ptr() as usize + arg.len();
                return (arg, cmd[arg_end..].trim());
            }
            return ("", "");
        }
        // Found the real command — rest is everything after it
        let part_end = part.as_ptr() as usize - cmd.as_ptr() as usize + part.len();
        return (part, cmd[part_end..].trim());
    }
    ("", "")
}

/// Check if a command has no meaningful arguments after the token (bare REPL).
fn is_bare_repl(rest: &str) -> bool {
    rest.is_empty() || rest == "-V" || rest == "--version"
}

/// Check if the command contains any of the given flags.
///
/// Handles both space-separated (`-e "SQL"`) and compact (`-e'SQL'`, `-e"SQL"`)
/// flag styles that agents commonly produce.
fn has_flag(cmd: &str, flags: &[&str]) -> bool {
    cmd.split_whitespace().any(|token| {
        flags.iter().any(|flag| {
            token == *flag
                || token.starts_with(&format!("{flag}'"))
                || token.starts_with(&format!("{flag}\""))
        })
    })
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
