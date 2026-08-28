//! Run-command diagnostics engine.
//!
//! Wraps every `run_command` invocation with rich, structured feedback:
//! pre-execution warnings, filesystem observation, no-op detection,
//! smart truncation, and workspace digests.
//!
//! The engine is per-agent (created when the agent starts, passed through
//! each tool call) to track cross-command state like loop detection.

pub mod envelope;
pub mod loop_detector;
pub mod post;
pub mod pre;
pub mod types;
pub mod workspace;

mod tests;

use std::collections::hash_map::Entry;
use std::collections::HashMap;
use std::path::PathBuf;

use envelope::{CommandEnvelope, Severity};
use loop_detector::LoopDetector;
use post::noop::NoOpCheck;
use post::PostCheck;
use pre::interactive::InteractiveCheck;
use pre::shell_compat::ShellCompatCheck;
use pre::state_persistence::StatePersistenceCheck;
use pre::PreCheck;
use types::{ChangeType, FileChange};
use workspace::snapshot::capture_snapshot;
use workspace::WorkspaceTracker;

use crate::execution::{ContainerError, ContainerHandle};

/// Diagnostics engine that wraps shell command execution with analysis.
///
/// One instance per agent execution. Tracks state across commands within
/// a single agent's lifecycle (command index, loop detection, workspace).
pub struct DiagnosticsEngine {
    pre_checks: Vec<Box<dyn PreCheck>>,
    post_checks: Vec<Box<dyn PostCheck>>,
    workspace: WorkspaceTracker,
    loop_detector: LoopDetector,
    command_index: usize,
    /// Every file this agent created or modified, keyed by path. First-seen
    /// change type wins; size tracks the latest write.
    touched: HashMap<PathBuf, FileChange>,
}

impl DiagnosticsEngine {
    /// Create a new engine with all default checks enabled.
    pub fn new() -> Self {
        Self {
            pre_checks: vec![
                Box::new(StatePersistenceCheck),
                Box::new(InteractiveCheck),
                Box::new(ShellCompatCheck),
                Box::new(pre::heredoc::HeredocCheck),
            ],
            post_checks: vec![Box::new(NoOpCheck)],
            workspace: WorkspaceTracker::new(),
            loop_detector: LoopDetector::new(),
            command_index: 0,
            touched: HashMap::new(),
        }
    }

    /// Run pre-checks, execute the command, assemble and render the envelope.
    ///
    /// Returns the rendered text to be used as the tool response.
    pub async fn execute(
        &mut self,
        command: &str,
        handle: &ContainerHandle,
    ) -> Result<String, ContainerError> {
        self.command_index += 1;

        // Phase 1: Pre-execution analysis
        let pre_warnings = pre::run_pre_checks(&self.pre_checks, command);

        // Phase 1b: The one blocking pre-check. A command cut mid-heredoc is
        // guaranteed to corrupt a file, so it is reported rather than run —
        // the shell would otherwise write the fragment and report success.
        if let Some(blocker) = pre_warnings.iter().find(|d| {
            d.severity == Severity::Error && d.category == envelope::DiagnosticCategory::Truncation
        }) {
            let envelope = CommandEnvelope {
                command: command.to_string(),
                exit_code: -1,
                stdout: String::new(),
                stderr: String::new(),
                duration_ms: 0,
                severity: Severity::Error,
                pre_warnings: vec![blocker.clone()],
                post_diagnostics: Vec::new(),
                file_changes: Vec::new(),
                workspace_digest: None,
                loop_status: loop_detector::LoopStatus::Clean,
            };
            return Ok(envelope.render());
        }

        // Phase 2: Snapshot → Execute → Diff
        let before = capture_snapshot(handle).await;
        self.workspace.initialize(&before); // baseline on first command
        let result = handle.exec_shell(command).await?;
        tracing::debug!(
            container = %handle.container_name(),
            exit_code = result.exit_code,
            duration_ms = result.duration_ms,
            "Container exec completed"
        );
        let after = capture_snapshot(handle).await;
        let file_changes = WorkspaceTracker::diff(&before, &after);

        // Phase 3: Post-execution analysis
        let mut post_diagnostics =
            post::run_post_checks(&self.post_checks, command, &result, &file_changes);

        // Phase 3b: Fix suggestions from stderr
        let suggestions = post::suggestions::suggest_fix(&result.stderr);
        post_diagnostics.extend(suggestions);

        // Accumulate this agent's output files for the downstream passdown.
        for change in &file_changes {
            match self.touched.entry(change.path.clone()) {
                Entry::Occupied(mut e) => e.get_mut().size = change.size,
                Entry::Vacant(e) => {
                    e.insert(change.clone());
                }
            }
        }

        // Phase 4: Loop detection
        let loop_status = self.loop_detector.record(self.command_index, &file_changes);

        // Build workspace digest
        let workspace_digest = Some(self.workspace.digest(&after, &file_changes));

        // Update workspace state for next command
        self.workspace.update(&after);

        // Compute severity
        let has_file_changes = !file_changes.is_empty();
        let severity = CommandEnvelope::compute_severity(
            result.exit_code,
            &pre_warnings,
            &post_diagnostics,
            has_file_changes,
        );

        // Assemble envelope
        let envelope = CommandEnvelope {
            exit_code: result.exit_code,
            stdout: result.stdout,
            stderr: result.stderr,
            duration_ms: result.duration_ms,
            severity,
            pre_warnings,
            post_diagnostics,
            file_changes,
            workspace_digest,
            loop_status,
            command: command.to_string(),
        };

        Ok(envelope.render())
    }

    /// Current command index (1-based).
    pub fn command_index(&self) -> usize {
        self.command_index
    }

    /// Record a write made by a first-class file tool.
    ///
    /// `write_file` and `edit_file` never touch the shell, so the
    /// snapshot → exec → snapshot path in `execute()` never sees them. Without
    /// this bridge, the moment agents stop writing through heredocs the
    /// passdown `files:` line goes empty, `synthesize_tool_summary` degrades to
    /// a bare tool count, and the loop detector goes blind — including the
    /// repeat-edit nudge that fired in run dd27d008.
    ///
    /// Returns the loop status so the caller can surface a nudge on the tool
    /// result, exactly as the `run_command` envelope does.
    pub fn record_file_write(
        &mut self,
        path: PathBuf,
        change_type: ChangeType,
        size: u64,
    ) -> loop_detector::LoopStatus {
        self.command_index += 1;
        let change = FileChange {
            path,
            change_type,
            size,
        };
        match self.touched.entry(change.path.clone()) {
            Entry::Occupied(mut e) => e.get_mut().size = change.size,
            Entry::Vacant(e) => {
                e.insert(change.clone());
            }
        }
        self.loop_detector
            .record(self.command_index, std::slice::from_ref(&change))
    }

    /// Files this agent produced, ready for the downstream passdown.
    ///
    /// Deletions and workspace machinery are dropped, the largest `limit`
    /// entries are kept, and the count of anything dropped by the cap is
    /// returned alongside so the caller can say `(+N more)`.
    pub fn produced_files(&self, limit: usize) -> (Vec<FileChange>, usize) {
        let mut files: Vec<FileChange> = self
            .touched
            .values()
            .filter(|c| c.change_type != ChangeType::Deleted && !workspace::is_noise(&c.path))
            .cloned()
            .collect();

        // Largest first, path as a tiebreak so output is deterministic.
        files.sort_by(|a, b| b.size.cmp(&a.size).then_with(|| a.path.cmp(&b.path)));

        let dropped = files.len().saturating_sub(limit);
        files.truncate(limit);
        (files, dropped)
    }
}

impl Default for DiagnosticsEngine {
    fn default() -> Self {
        Self::new()
    }
}

/// Sanitize model tool inputs: unescape HTML entities and strip
/// model-specific XML artifacts (e.g. Grok citation tags).
pub fn html_unescape(s: &str) -> String {
    let s = s
        .replace("&amp;", "&")
        .replace("&gt;", ">")
        .replace("&lt;", "<")
        .replace("&quot;", "\"")
        .replace("&#39;", "'")
        .replace("&#x27;", "'")
        .replace("&apos;", "'");

    strip_grok_tags(&s)
}

/// Strip `<grok:render ...>...</grok:render>` citation blocks that Grok
/// embeds in tool call inputs. These are multi-line XML that pollute
/// file content and waste tokens on read-back.
fn strip_grok_tags(s: &str) -> String {
    const OPEN: &str = "<grok:render";
    const CLOSE: &str = "</grok:render>";

    let mut result = String::with_capacity(s.len());
    let mut remaining = s;

    while let Some(start) = remaining.find(OPEN) {
        // Keep everything before the tag
        result.push_str(&remaining[..start]);

        // Find the closing tag
        let after_open = &remaining[start..];
        if let Some(end_offset) = after_open.find(CLOSE) {
            // Skip past the closing tag
            remaining = &after_open[end_offset + CLOSE.len()..];
        } else {
            // No closing tag found — strip to end of line to avoid breaking content
            if let Some(nl) = after_open.find('\n') {
                remaining = &after_open[nl..];
            } else {
                remaining = "";
            }
        }
    }

    result.push_str(remaining);
    result
}
