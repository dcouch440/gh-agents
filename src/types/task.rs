//! Task-related types

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use uuid::Uuid;

use super::agent::AgentId;

/// Unique identifier for a task
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct TaskId(pub Uuid);

impl TaskId {
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
}

impl Default for TaskId {
    fn default() -> Self {
        Self::new()
    }
}

/// Task lifecycle status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum TaskStatus {
    #[default]
    Pending,
    InProgress,
    Review,
    Completed,
    Failed,
}

/// Task priority levels
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum Priority {
    Low,
    #[default]
    Normal,
    High,
    /// Can preempt other work
    Urgent,
}

/// Unique identifier for a vertical slice
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SliceId(pub Uuid);

impl SliceId {
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
}

impl Default for SliceId {
    fn default() -> Self {
        Self::new()
    }
}

/// A vertical slice of work (smallest deployable unit)
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct VerticalSlice {
    pub id: SliceId,
    pub ticket_id: Uuid,
    pub title: String,
    pub description: String,
    pub tasks: Vec<TaskId>,
    pub status: TaskStatus,
    pub created_at: DateTime<Utc>,
}

/// Individual task assigned to an agent
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Task {
    pub id: TaskId,
    pub slice_id: Option<SliceId>,
    pub title: String,
    pub description: String,
    pub assigned_tier: super::agent::AgentTier,
    pub assigned_agent: Option<AgentId>,
    pub status: TaskStatus,
    pub priority: Priority,
    pub context_files: Vec<PathBuf>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Types of task events (for append-only log)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TaskEventType {
    Created,
    Assigned,
    Started,
    ProgressUpdate,
    ContextRequested,
    SubmittedForReview,
    ReviewFeedback,
    Completed,
    Failed,
    Cancelled,
    Escalated,
}

/// Task state change log entry (append-only)
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TaskEvent {
    pub id: Uuid,
    pub task_id: TaskId,
    pub event_type: TaskEventType,
    pub agent_id: Option<AgentId>,
    pub details: String,
    pub timestamp: DateTime<Utc>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn task_id_generates_unique() {
        let id1 = TaskId::new();
        let id2 = TaskId::new();
        assert_ne!(id1, id2);
    }

    #[test]
    fn task_status_default_is_pending() {
        assert_eq!(TaskStatus::default(), TaskStatus::Pending);
    }

    #[test]
    fn priority_default_is_normal() {
        assert_eq!(Priority::default(), Priority::Normal);
    }
}
