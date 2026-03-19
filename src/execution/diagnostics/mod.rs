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

use envelope::CommandEnvelope;
use loop_detector::LoopDetector;
use post::noop::NoOpCheck;
use post::PostCheck;
use pre::interactive::InteractiveCheck;
use pre::shell_compat::ShellCompatCheck;
use pre::state_persistence::StatePersistenceCheck;
use pre::PreCheck;
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
}

impl DiagnosticsEngine {
    /// Create a new engine with all default checks enabled.
    pub fn new() -> Self {
        Self {
            pre_checks: vec![
                Box::new(StatePersistenceCheck),
                Box::new(InteractiveCheck),
                Box::new(ShellCompatCheck),
            ],
            post_checks: vec![Box::new(NoOpCheck)],
            workspace: WorkspaceTracker::new(),
            loop_detector: LoopDetector::new(),
            command_index: 0,
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

        // Phase 2: Snapshot → Execute → Diff
        let before = capture_snapshot(handle).await;
        let result = handle.exec_shell(command).await?;
        let after = capture_snapshot(handle).await;
        let file_changes = WorkspaceTracker::diff(&before, &after);

        // Phase 3: Post-execution analysis
        let mut post_diagnostics =
            post::run_post_checks(&self.post_checks, command, &result, &file_changes);

        // Phase 3b: Fix suggestions from stderr
        let suggestions = post::suggestions::suggest_fix(&result.stderr);
        post_diagnostics.extend(suggestions);

        // Phase 4: Loop detection
        let loop_status = self.loop_detector.record(self.command_index, &file_changes);

        // Build workspace digest
        let workspace_digest = Some(self.workspace.digest(&after));

        // Update workspace state for next command
        self.workspace.update(&after);

        // Compute severity
        let severity =
            CommandEnvelope::compute_severity(result.exit_code, &pre_warnings, &post_diagnostics);

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
}

/// Unescape HTML entities that some models (xAI/Grok) emit in tool inputs.
pub fn html_unescape(s: &str) -> String {
    s.replace("&amp;", "&")
        .replace("&gt;", ">")
        .replace("&lt;", "<")
        .replace("&quot;", "\"")
        .replace("&#39;", "'")
        .replace("&#x27;", "'")
}
