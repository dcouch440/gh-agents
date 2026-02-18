//! Legacy task-related types still referenced by tickets, messages, and GitHub integration.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

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
