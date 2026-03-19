//! No-op detection — the single most impactful diagnostic.
//!
//! Detects when a command succeeds but does nothing useful:
//! - `sed -i` with no pattern match → exit 0, no changes
//! - `grep` with no results → exit 0, empty stdout
//! - `pip install` already installed → "already satisfied" in stdout

use crate::execution::ContainerExecResult;

use super::super::envelope::{Diagnostic, DiagnosticCategory, Severity};
use super::super::types::FileChange;
use super::PostCheck;

pub struct NoOpCheck;

impl PostCheck for NoOpCheck {
    fn check(
        &self,
        command: &str,
        result: &ContainerExecResult,
        changes: &[FileChange],
    ) -> Vec<Diagnostic> {
        // Only check successful commands — failed commands aren't no-ops
        if !result.success {
            return vec![];
        }

        match classify_command(command) {
            CommandType::Mutation if changes.is_empty() => {
                vec![Diagnostic {
                    severity: Severity::NoOp,
                    category: DiagnosticCategory::NoOp,
                    message: "Command succeeded but no files were changed. The operation may have had no effect.".to_string(),
                    suggestion: Some(
                        "Verify the target file/path exists and the pattern matches.".to_string(),
                    ),
                }]
            }
            // Only report search no-ops when no files were created/modified either.
            // `cat > file << 'EOF'` has empty stdout but creates a file — not a no-op.
            CommandType::Search
                if result.stdout.trim().is_empty() && changes.is_empty() =>
            {
                vec![Diagnostic {
                    severity: Severity::NoOp,
                    category: DiagnosticCategory::NoOp,
                    message: "Search produced no results.".to_string(),
                    suggestion: Some(
                        "Check the pattern and path. Try a broader search.".to_string(),
                    ),
                }]
            }
            CommandType::PackageInstall if is_already_installed(&result.stdout) => {
                vec![Diagnostic {
                    severity: Severity::Info,
                    category: DiagnosticCategory::NoOp,
                    message: "Package was already installed. No new packages added.".to_string(),
                    suggestion: None,
                }]
            }
            _ => vec![],
        }
    }
}

/// Command classification for no-op detection.
#[derive(Debug, PartialEq)]
enum CommandType {
    /// Commands that should change files: sed -i, cp, mv, mkdir, chmod, etc.
    Mutation,
    /// Commands that should produce output: grep, find, cat, etc.
    Search,
    /// Package install commands: pip install, npm install, apt-get install.
    PackageInstall,
    /// Everything else — no no-op detection.
    Other,
}

/// Classify a command by its expected side effects.
fn classify_command(cmd: &str) -> CommandType {
    let trimmed = cmd.trim();

    // Handle chained commands — classify based on the first command
    // that could be a mutation/search. For `cd /app && sed -i ...`,
    // skip the `cd` and classify `sed`.
    for segment in split_chain_segments(trimmed) {
        let segment = segment.trim();
        if segment.is_empty() {
            continue;
        }

        let first_token = segment.split_whitespace().next().unwrap_or("");
        match first_token {
            // Mutation commands
            "sed" if segment.contains(" -i") => return CommandType::Mutation,
            "cp" | "mv" | "mkdir" | "chmod" | "chown" | "tee" | "touch" => {
                return CommandType::Mutation;
            }

            // cat with redirect or heredoc is a file write (mutation)
            "cat" if segment.contains('>') || segment.contains("<<") => {
                return CommandType::Mutation;
            }

            // Search commands
            "grep" | "find" | "awk" | "cat" | "head" | "tail" | "wc" => {
                return CommandType::Search;
            }
            // sed without -i is a search/transform (prints to stdout)
            "sed" => return CommandType::Search,

            // Package install commands
            "pip" | "pip3" if segment.contains("install") => {
                return CommandType::PackageInstall;
            }
            "npm" | "yarn" if segment.contains("install") || segment.contains("add") => {
                return CommandType::PackageInstall;
            }
            "apt-get" | "apt" if segment.contains("install") => {
                return CommandType::PackageInstall;
            }

            // Skip cd, export, etc. — continue to next segment
            "cd" | "export" | "source" => continue,

            _ => {}
        }
    }

    CommandType::Other
}

/// Split a command string on `&&` (outside quotes) into segments.
fn split_chain_segments(cmd: &str) -> Vec<&str> {
    let mut segments = Vec::new();
    let mut start = 0;
    let chars: Vec<char> = cmd.chars().collect();
    let len = chars.len();
    let mut in_single = false;
    let mut in_double = false;

    for i in 0..len {
        match chars[i] {
            '\'' if !in_double => in_single = !in_single,
            '"' if !in_single => in_double = !in_double,
            '&' if !in_single && !in_double && i + 1 < len && chars[i + 1] == '&' => {
                segments.push(&cmd[start..i]);
                start = i + 2; // skip both &
            }
            _ => {}
        }
    }
    segments.push(&cmd[start..]);
    segments
}

/// Check if stdout indicates the package was already installed.
fn is_already_installed(stdout: &str) -> bool {
    let lower = stdout.to_lowercase();
    // pip
    lower.contains("requirement already satisfied")
        || lower.contains("already satisfied")
        // npm
        || lower.contains("up to date")
        || lower.contains("added 0 packages")
        // apt
        || lower.contains("0 newly installed")
        || lower.contains("is already the newest version")
}
