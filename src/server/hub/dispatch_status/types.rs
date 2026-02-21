//! Dispatch status types.
//!
//! Decoupled from `TaskRegistry` internals so the render layer
//! works with pure snapshots — no runtime dependencies.

/// Status of a dispatch task for rendering.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DispatchStatus {
    InProgress,
    Completed,
    Failed,
    Cancelled,
}

impl DispatchStatus {
    /// XML-friendly status string.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::InProgress => "in_progress",
            Self::Completed => "completed",
            Self::Failed => "failed",
            Self::Cancelled => "cancelled",
        }
    }

    /// Whether this status represents a finished (non-running) task.
    pub fn is_terminal(self) -> bool {
        !matches!(self, Self::InProgress)
    }
}

/// A snapshot of a single dispatch task, ready for rendering.
#[derive(Debug, Clone)]
pub struct DispatchSnapshot {
    /// Short execution id (first 8 chars).
    pub id: String,
    /// Task instruction (truncated).
    pub instruction: String,
    /// Current status.
    pub status: DispatchStatus,
    /// Relative time string ("45s ago", "2m ago").
    pub elapsed: String,
    /// Result summary for terminal tasks (truncated).
    pub result: Option<String>,
}
