//! Task-related types

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
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

impl std::fmt::Display for TaskId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
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
    /// Optional metadata for routing hints and tracking
    pub metadata: Option<HashMap<String, String>>,
    /// Tasks that must complete before this task can start
    pub depends_on: Vec<TaskId>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl Task {
    /// Create a new task with default values
    pub fn new(title: impl Into<String>, tier: super::agent::AgentTier) -> Self {
        Self {
            id: TaskId::new(),
            slice_id: None,
            title: title.into(),
            description: String::new(),
            assigned_tier: tier,
            assigned_agent: None,
            status: TaskStatus::Pending,
            priority: Priority::Normal,
            context_files: vec![],
            metadata: None,
            depends_on: vec![],
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }

    /// Add a dependency to this task
    pub fn with_dependency(mut self, dep: TaskId) -> Self {
        self.depends_on.push(dep);
        self
    }

    /// Set dependencies for this task
    pub fn with_dependencies(mut self, deps: Vec<TaskId>) -> Self {
        self.depends_on = deps;
        self
    }
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
