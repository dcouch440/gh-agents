//! Task-related types

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use uuid::Uuid;

use super::agent::AgentId;

/// Unique identifier for a task
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, utoipa::ToSchema)]
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
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default, utoipa::ToSchema,
)]
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
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default, utoipa::ToSchema,
)]
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
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, utoipa::ToSchema)]
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
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, utoipa::ToSchema)]
pub struct Task {
    pub id: TaskId,
    pub slice_id: Option<SliceId>,
    pub title: String,
    pub description: String,
    pub assigned_agent: Option<AgentId>,
    pub status: TaskStatus,
    pub priority: Priority,
    #[schema(value_type = Vec<String>)]
    pub context_files: Vec<PathBuf>,
    /// Optional metadata for routing hints and tracking
    pub metadata: Option<HashMap<String, String>>,
    /// Tasks that must complete before this task can start
    pub depends_on: Vec<TaskId>,
    /// Number of times this task has been requeued after failure
    pub retry_count: u32,
    /// Maximum retries before the task is permanently failed
    pub max_retries: u32,
    /// Last error message when the task failed
    pub last_error: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl Task {
    /// Create a new task with default values
    pub fn new(title: impl Into<String>) -> Self {
        Self {
            id: TaskId::new(),
            slice_id: None,
            title: title.into(),
            description: String::new(),
            assigned_agent: None,
            status: TaskStatus::Pending,
            priority: Priority::Normal,
            context_files: vec![],
            metadata: None,
            depends_on: vec![],
            retry_count: 0,
            max_retries: crate::constants::TASK_MAX_RETRIES,
            last_error: None,
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

    #[test]
    fn task_id_display_formatting() {
        let uuid = Uuid::parse_str("550e8400-e29b-41d4-a716-446655440000").unwrap();
        let id = TaskId(uuid);
        assert_eq!(id.to_string(), "550e8400-e29b-41d4-a716-446655440000");
    }

    #[test]
    fn task_id_default() {
        let id = TaskId::default();
        // Should produce a valid UUID
        assert!(!id.0.is_nil());
    }

    #[test]
    fn slice_id_new_uniqueness() {
        let a = SliceId::new();
        let b = SliceId::new();
        assert_ne!(a, b);
    }

    #[test]
    fn slice_id_default() {
        let id = SliceId::default();
        assert!(!id.0.is_nil());
    }

    #[test]
    fn task_new_defaults() {
        let task = Task::new("Test task");
        assert_eq!(task.title, "Test task");
        assert_eq!(task.description, "");
        assert_eq!(task.status, TaskStatus::Pending);
        assert_eq!(task.priority, Priority::Normal);
        assert!(task.context_files.is_empty());
        assert!(task.metadata.is_none());
        assert!(task.depends_on.is_empty());
        assert_eq!(task.retry_count, 0);
        assert_eq!(task.max_retries, crate::constants::TASK_MAX_RETRIES);
        assert!(task.last_error.is_none());
        assert!(task.slice_id.is_none());
        assert!(task.assigned_agent.is_none());
    }

    #[test]
    fn task_with_dependency() {
        let dep = TaskId::new();
        let task = Task::new("t").with_dependency(dep.clone());
        assert_eq!(task.depends_on.len(), 1);
        assert_eq!(task.depends_on[0], dep);
    }

    #[test]
    fn task_with_dependencies_replaces() {
        let dep1 = TaskId::new();
        let dep2 = TaskId::new();
        let task = Task::new("t")
            .with_dependency(TaskId::new())
            .with_dependencies(vec![dep1.clone(), dep2.clone()]);
        assert_eq!(task.depends_on.len(), 2);
        assert_eq!(task.depends_on[0], dep1);
    }

    #[test]
    fn task_status_serde_roundtrip() {
        let variants = [
            TaskStatus::Pending,
            TaskStatus::InProgress,
            TaskStatus::Review,
            TaskStatus::Completed,
            TaskStatus::Failed,
        ];
        for v in &variants {
            let json = serde_json::to_string(v).unwrap();
            let parsed: TaskStatus = serde_json::from_str(&json).unwrap();
            assert_eq!(*v, parsed);
        }
    }

    #[test]
    fn priority_serde_roundtrip() {
        let variants = [
            Priority::Low,
            Priority::Normal,
            Priority::High,
            Priority::Urgent,
        ];
        for v in &variants {
            let json = serde_json::to_string(v).unwrap();
            let parsed: Priority = serde_json::from_str(&json).unwrap();
            assert_eq!(*v, parsed);
        }
    }

    #[test]
    fn task_event_type_serde_roundtrip() {
        let variants = [
            TaskEventType::Created,
            TaskEventType::Assigned,
            TaskEventType::Started,
            TaskEventType::ProgressUpdate,
            TaskEventType::ContextRequested,
            TaskEventType::SubmittedForReview,
            TaskEventType::ReviewFeedback,
            TaskEventType::Completed,
            TaskEventType::Failed,
            TaskEventType::Cancelled,
            TaskEventType::Escalated,
        ];
        for v in &variants {
            let json = serde_json::to_string(v).unwrap();
            let parsed: TaskEventType = serde_json::from_str(&json).unwrap();
            assert_eq!(*v, parsed);
        }
    }

    #[test]
    fn task_event_construction() {
        let event = TaskEvent {
            id: Uuid::new_v4(),
            task_id: TaskId::new(),
            event_type: TaskEventType::Created,
            agent_id: None,
            details: "created".to_string(),
            timestamp: chrono::Utc::now(),
        };
        assert_eq!(event.event_type, TaskEventType::Created);
        assert!(event.agent_id.is_none());
    }

    #[test]
    fn vertical_slice_construction() {
        let slice = VerticalSlice {
            id: SliceId::new(),
            ticket_id: Uuid::new_v4(),
            title: "Slice 1".to_string(),
            description: "desc".to_string(),
            tasks: vec![TaskId::new()],
            status: TaskStatus::Pending,
            created_at: chrono::Utc::now(),
        };
        assert_eq!(slice.title, "Slice 1");
        assert_eq!(slice.tasks.len(), 1);
    }
}
