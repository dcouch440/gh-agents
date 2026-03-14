//! Registry for tracking background dispatch tasks.
//!
//! Provides `TaskRegistry`, a concurrent map of dispatch task entries with
//! lifecycle management (spawn, cancel, complete, fail) and cancellation tokens.

use chrono::{DateTime, Utc};
use dashmap::DashMap;
use serde::Serialize;
use serde_json::Value;
use tokio_util::sync::CancellationToken;
use uuid::Uuid;

mod tests;

/// Status of a background dispatch task.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TaskStatus {
    Running,
    Completed,
    Cancelled,
    Failed,
}

impl TaskStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Running => "running",
            Self::Completed => "completed",
            Self::Cancelled => "cancelled",
            Self::Failed => "failed",
        }
    }
}

/// A single trace event recorded during dispatch execution.
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum TraceEvent {
    Token {
        content: String,
        ts: DateTime<Utc>,
    },
    ToolStart {
        tool_name: String,
        tool_id: String,
        input: Value,
        ts: DateTime<Utc>,
    },
    ToolEnd {
        tool_name: String,
        tool_id: String,
        result: Value,
        ts: DateTime<Utc>,
    },
    Error {
        error: String,
        ts: DateTime<Utc>,
    },
    SystemPrompt {
        content: String,
        agent_name: Option<String>,
        ts: DateTime<Utc>,
    },
}

/// A single background dispatch task entry.
#[derive(Debug, Clone)]
pub struct TaskEntry {
    pub execution_id: Uuid,
    pub step_id: Uuid,
    pub workflow_id: Uuid,
    pub session_id: Uuid,
    pub status: TaskStatus,
    pub instruction: String,
    pub cancel_token: CancellationToken,
    pub created_at: DateTime<Utc>,
    pub result: Option<String>,
    /// Execution trace — tokens, tool calls, errors.
    pub trace: Vec<TraceEvent>,
}

/// Centralized registry for background dispatch tasks.
///
/// Thread-safe via `DashMap`. Each task gets a `CancellationToken` that the
/// background runner monitors for cooperative cancellation.
pub struct TaskRegistry {
    tasks: DashMap<Uuid, TaskEntry>,
}

impl TaskRegistry {
    /// Create an empty registry.
    pub fn new() -> Self {
        Self {
            tasks: DashMap::new(),
        }
    }

    /// Register a new dispatch task and return its execution_id and cancel token.
    pub fn spawn_task(
        &self,
        step_id: Uuid,
        workflow_id: Uuid,
        session_id: Uuid,
        instruction: String,
    ) -> (Uuid, CancellationToken) {
        let execution_id = Uuid::new_v4();
        let cancel_token = CancellationToken::new();

        self.tasks.insert(
            execution_id,
            TaskEntry {
                execution_id,
                step_id,
                workflow_id,
                session_id,
                status: TaskStatus::Running,
                instruction,
                cancel_token: cancel_token.clone(),
                created_at: Utc::now(),
                result: None,
                trace: Vec::new(),
            },
        );

        (execution_id, cancel_token)
    }

    /// Cancel a running task. Returns `true` if the task existed and was running.
    pub fn cancel_task(&self, execution_id: Uuid) -> bool {
        if let Some(mut entry) = self.tasks.get_mut(&execution_id) {
            if entry.status == TaskStatus::Running {
                entry.cancel_token.cancel();
                entry.status = TaskStatus::Cancelled;
                return true;
            }
        }
        false
    }

    /// Mark a task as completed with an optional summary.
    pub fn mark_completed(&self, execution_id: Uuid, summary: Option<String>) {
        if let Some(mut entry) = self.tasks.get_mut(&execution_id) {
            entry.status = TaskStatus::Completed;
            entry.result = summary;
        }
    }

    /// Mark a task as failed with an error message.
    pub fn mark_failed(&self, execution_id: Uuid, error: String) {
        if let Some(mut entry) = self.tasks.get_mut(&execution_id) {
            entry.status = TaskStatus::Failed;
            entry.result = Some(error);
        }
    }

    /// Get a snapshot of a task entry.
    pub fn get_task(&self, execution_id: Uuid) -> Option<TaskEntry> {
        self.tasks.get(&execution_id).map(|e| e.clone())
    }

    /// List all tasks for a given step, ordered by creation time (newest first).
    pub fn list_tasks_for_step(&self, step_id: Uuid) -> Vec<TaskEntry> {
        let mut tasks: Vec<TaskEntry> = self
            .tasks
            .iter()
            .filter(|e| e.step_id == step_id)
            .map(|e| e.clone())
            .collect();
        tasks.sort_by(|a, b| b.created_at.cmp(&a.created_at));
        tasks
    }

    /// Cancel all running tasks. Returns the number of tasks cancelled.
    pub fn cancel_all(&self) -> usize {
        let mut cancelled = 0;
        for mut entry in self.tasks.iter_mut() {
            if entry.status == TaskStatus::Running {
                entry.cancel_token.cancel();
                entry.status = TaskStatus::Cancelled;
                cancelled += 1;
            }
        }
        cancelled
    }

    /// Remove completed/failed/cancelled entries older than the given cutoff.
    pub fn cleanup_before(&self, cutoff: DateTime<Utc>) {
        self.tasks
            .retain(|_, entry| entry.status == TaskStatus::Running || entry.created_at >= cutoff);
    }

    /// Append a trace event to a running task.
    pub fn append_trace(&self, execution_id: Uuid, event: TraceEvent) {
        if let Some(mut entry) = self.tasks.get_mut(&execution_id) {
            entry.trace.push(event);
        }
    }

    /// Return the number of currently running tasks.
    pub fn active_count(&self) -> usize {
        self.tasks
            .iter()
            .filter(|e| e.status == TaskStatus::Running)
            .count()
    }
}

impl Default for TaskRegistry {
    fn default() -> Self {
        Self::new()
    }
}
