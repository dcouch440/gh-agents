//! CommandEnvelope — the structured result of every `run_command` invocation.
//!
//! The envelope carries raw output alongside diagnostics (pre-warnings,
//! post-analysis, file changes, workspace digest). The `render()` method
//! converts it to text optimized for LLM consumption.

use super::loop_detector::LoopStatus;
use super::post::stderr_classifier::classify_stderr;
use super::post::truncation::truncate_stdout;
use super::types::FileChange;
use super::workspace::digest::WorkspaceDigest;

/// Top-level severity of a command result.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Severity {
    Ok,
    Info,
    NoOp,
    Warning,
    Error,
    Loop,
}

impl Severity {
    pub fn label(&self) -> &'static str {
        match self {
            Severity::Ok => "success",
            Severity::Info => "success",
            Severity::NoOp => "success (no-op)",
            Severity::Warning => "warning",
            Severity::Error => "failed",
            Severity::Loop => "loop detected",
        }
    }
}

/// Category of a diagnostic observation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiagnosticCategory {
    StatePersistence,
    InteractiveCommand,
    ShellCompat,
    NoOp,
    Truncation,
    StderrClassification,
    Suggestion,
    LoopDetected,
}

/// A single diagnostic observation about a command.
#[derive(Debug, Clone)]
pub struct Diagnostic {
    pub severity: Severity,
    pub category: DiagnosticCategory,
    pub message: String,
    pub suggestion: Option<String>,
}

/// The complete result of a diagnosed command execution.
pub struct CommandEnvelope {
    pub command: String,
    pub exit_code: i32,
    pub stdout: String,
    pub stderr: String,
    pub duration_ms: u64,
    pub severity: Severity,
    pub pre_warnings: Vec<Diagnostic>,
    pub post_diagnostics: Vec<Diagnostic>,
    pub file_changes: Vec<FileChange>,
    pub workspace_digest: Option<WorkspaceDigest>,
    pub loop_status: LoopStatus,
}

impl CommandEnvelope {
    /// Compute the maximum severity from all diagnostics.
    ///
    /// When `exit_code != 0` but files were created or modified, downgrades
    /// from Error to Warning — the command had an effect even if the shell
    /// returned non-zero (common with heredoc + chained commands).
    pub fn compute_severity(
        exit_code: i32,
        pre_warnings: &[Diagnostic],
        post_diagnostics: &[Diagnostic],
        has_file_changes: bool,
    ) -> Severity {
        let mut max = match (exit_code, has_file_changes) {
            (0, _) => Severity::Ok,
            (_, true) => Severity::Warning,
            (_, false) => Severity::Error,
        };
        for d in pre_warnings.iter().chain(post_diagnostics.iter()) {
            if d.severity > max {
                max = d.severity;
            }
        }
        max
    }

    /// Render the envelope to text for the LLM.
    pub fn render(&self) -> String {
        let mut out = String::with_capacity(self.stdout.len() + 512);

        // Result line
        out.push_str(&format!("result: {}\n", self.severity.label()));

        // Pre-execution warnings
        for w in &self.pre_warnings {
            out.push_str(&format!("\npre-execution warning:\n  {}\n", w.message));
            if let Some(ref s) = w.suggestion {
                out.push_str(&format!("  suggestion: {}\n", s));
            }
        }

        // stdout with smart truncation
        if !self.stdout.is_empty() {
            let truncated = truncate_stdout(&self.command, &self.stdout);
            out.push_str(&format!("\nstdout ({}):\n", truncated.summary()));
            for line in truncated.content.lines() {
                out.push_str("  ");
                out.push_str(line);
                out.push('\n');
            }
        }

        // stderr with classification
        if !self.stderr.is_empty() {
            let classified = classify_stderr(&self.stderr);
            out.push_str(&format!("\nstderr summary: {}\n", classified.summary));
            for e in &classified.errors {
                out.push_str(&format!("  [ERROR] {}\n", e));
            }
            // Show warnings only if there are few (avoid noise)
            if classified.warnings.len() <= 5 {
                for w in &classified.warnings {
                    out.push_str(&format!("  [WARN] {}\n", w));
                }
            }
        }

        // Post diagnostics (no-op warnings, suggestions, etc.)
        for d in &self.post_diagnostics {
            out.push('\n');
            out.push_str(&d.message);
            out.push('\n');
            if let Some(ref s) = d.suggestion {
                out.push_str(&format!("  suggestion: {}\n", s));
            }
        }

        // File changes
        if !self.file_changes.is_empty() {
            out.push_str("\nchanges:\n");
            for fc in &self.file_changes {
                out.push_str(&format!("  {}: {}\n", fc.change_type, fc.path.display()));
            }
        }

        // Loop status
        if self.loop_status.should_render() {
            out.push('\n');
            out.push_str(&self.loop_status.render());
            out.push('\n');
        }

        // Workspace digest
        if let Some(ref digest) = self.workspace_digest {
            out.push('\n');
            out.push_str(&digest.render());
            out.push('\n');
        }

        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::execution::diagnostics::loop_detector::LoopStatus;

    #[test]
    fn severity_ordering() {
        assert!(Severity::Error > Severity::Warning);
        assert!(Severity::Warning > Severity::NoOp);
        assert!(Severity::NoOp > Severity::Info);
        assert!(Severity::Info > Severity::Ok);
        assert!(Severity::Loop > Severity::Error);
    }

    #[test]
    fn render_basic_success() {
        let envelope = CommandEnvelope {
            command: "echo hello".to_string(),
            exit_code: 0,
            stdout: "hello world\n".to_string(),
            stderr: String::new(),
            duration_ms: 50,
            severity: Severity::Ok,
            pre_warnings: vec![],
            post_diagnostics: vec![],
            file_changes: vec![],
            workspace_digest: None,
            loop_status: LoopStatus::Clean,
        };
        let rendered = envelope.render();
        assert!(rendered.starts_with("result: success\n"));
        assert!(rendered.contains("hello world"));
    }

    #[test]
    fn render_with_pre_warning() {
        let envelope = CommandEnvelope {
            command: "cd /app".to_string(),
            exit_code: 0,
            stdout: String::new(),
            stderr: String::new(),
            duration_ms: 10,
            severity: Severity::Warning,
            pre_warnings: vec![Diagnostic {
                severity: Severity::Warning,
                category: DiagnosticCategory::StatePersistence,
                message: "cd doesn't persist between commands.".to_string(),
                suggestion: Some("Chain with && or use absolute paths.".to_string()),
            }],
            post_diagnostics: vec![],
            file_changes: vec![],
            workspace_digest: None,
            loop_status: LoopStatus::Clean,
        };
        let rendered = envelope.render();
        assert!(rendered.contains("pre-execution warning:"));
        assert!(rendered.contains("cd doesn't persist"));
        assert!(rendered.contains("suggestion: Chain with &&"));
    }

    #[test]
    fn render_with_file_changes_and_digest() {
        use crate::execution::diagnostics::types::{ChangeType, FileChange};
        use std::path::PathBuf;

        let envelope = CommandEnvelope {
            command: "python build.py".to_string(),
            exit_code: 0,
            stdout: "done\n".to_string(),
            stderr: String::new(),
            duration_ms: 100,
            severity: Severity::Ok,
            pre_warnings: vec![],
            post_diagnostics: vec![],
            file_changes: vec![
                FileChange {
                    path: PathBuf::from("src/main.py"),
                    change_type: ChangeType::Created,
                    size: 420,
                },
                FileChange {
                    path: PathBuf::from("config.json"),
                    change_type: ChangeType::Modified,
                    size: 150,
                },
            ],
            workspace_digest: Some(WorkspaceDigest {
                file_count: 16,
                file_delta: 2,
                dir_count: 4,
                total_size: 89 * 1024,
                last_modified: Some(PathBuf::from("src/main.py")),
            }),
            loop_status: LoopStatus::Clean,
        };
        let rendered = envelope.render();
        assert!(rendered.contains("changes:"));
        assert!(rendered.contains("created: src/main.py"));
        assert!(rendered.contains("modified: config.json"));
        assert!(rendered.contains("16 files (+2)"));
        assert!(rendered.contains("4 dirs"));
    }

    #[test]
    fn render_with_stderr_classification() {
        let envelope = CommandEnvelope {
            command: "cargo build".to_string(),
            exit_code: 1,
            stdout: String::new(),
            stderr: "warning: unused variable\nerror[E0308]: mismatched types\n".to_string(),
            duration_ms: 5000,
            severity: Severity::Error,
            pre_warnings: vec![],
            post_diagnostics: vec![],
            file_changes: vec![],
            workspace_digest: None,
            loop_status: LoopStatus::Clean,
        };
        let rendered = envelope.render();
        assert!(rendered.contains("stderr summary:"));
        assert!(rendered.contains("[ERROR]"));
        assert!(rendered.contains("[WARN]"));
    }

    #[test]
    fn render_with_loop_warning() {
        let envelope = CommandEnvelope {
            command: "sed -i 's/x/y/' main.py".to_string(),
            exit_code: 0,
            stdout: String::new(),
            stderr: String::new(),
            duration_ms: 10,
            severity: Severity::Warning,
            pre_warnings: vec![],
            post_diagnostics: vec![],
            file_changes: vec![],
            workspace_digest: None,
            loop_status: LoopStatus::Warning {
                file: std::path::PathBuf::from("main.py"),
                edit_count: 5,
                message: "LOOP DETECTED: main.py edited 5 times".to_string(),
            },
        };
        let rendered = envelope.render();
        assert!(rendered.contains("LOOP DETECTED"));
    }

    #[test]
    fn compute_severity_uses_exit_code() {
        assert_eq!(
            CommandEnvelope::compute_severity(1, &[], &[], false),
            Severity::Error
        );
        assert_eq!(
            CommandEnvelope::compute_severity(0, &[], &[], false),
            Severity::Ok
        );
    }

    #[test]
    fn compute_severity_downgrades_with_file_changes() {
        // Non-zero exit but files changed → Warning, not Error
        assert_eq!(
            CommandEnvelope::compute_severity(1, &[], &[], true),
            Severity::Warning
        );
        // Zero exit with file changes → still Ok
        assert_eq!(
            CommandEnvelope::compute_severity(0, &[], &[], true),
            Severity::Ok
        );
    }

    #[test]
    fn compute_severity_upgrades_from_diagnostics() {
        let diag = Diagnostic {
            severity: Severity::Warning,
            category: DiagnosticCategory::StatePersistence,
            message: "test".to_string(),
            suggestion: None,
        };
        assert_eq!(
            CommandEnvelope::compute_severity(0, &[diag], &[], false),
            Severity::Warning
        );
    }
}
