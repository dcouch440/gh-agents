//! Repository traits for database operations.
//!
//! Each trait abstracts the DB operations for a specific domain module.
//! Production code uses `PgRepo` (see `pg_repo.rs`). Tests use `MockXxxRepo` from mockall.

use anyhow::Result;
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use uuid::Uuid;

use crate::db::ChatMessageRow;
use crate::github::{PrQueueEntry, QueueError as MergeQueueError};
use crate::observability::{Decision, LlmCall};
use crate::orchestration::DependencyError;
use crate::orchestration::QueueError as TaskQueueError;
use crate::types::{
    ChangeId, ChangeStatus, CostRecord, ProductionMode, RefactorChange, RefactorSession, Task,
    TaskId, TaskStatus,
};

// ============================================================================
// Merge Queue Repository
// ============================================================================

/// Database operations for the PR merge queue.
#[cfg_attr(test, mockall::automock)]
#[async_trait]
pub trait MergeQueueRepo: Send + Sync {
    /// Insert or update a queue entry (upsert).
    async fn insert_queue_entry(
        &self,
        id: Uuid,
        owner: String,
        repo: String,
        pr_number: u32,
        position: u32,
        now: DateTime<Utc>,
    ) -> Result<(), MergeQueueError>;

    /// Get the next queue position for a repo.
    async fn get_next_position(&self, owner: String, repo: String) -> Result<u32, MergeQueueError>;

    /// Delete a queue entry. Returns true if a row was deleted.
    async fn delete_queue_entry(
        &self,
        owner: String,
        repo: String,
        pr_number: u32,
    ) -> Result<bool, MergeQueueError>;

    /// Get all queue entries for a repo, ordered by position.
    async fn get_queue_entries(
        &self,
        owner: String,
        repo: String,
    ) -> Result<Vec<PrQueueEntry>, MergeQueueError>;

    /// Update the status (and optional error message) of a queue entry.
    async fn update_entry_status(
        &self,
        owner: String,
        repo: String,
        pr_number: u32,
        status: String,
        error_message: Option<String>,
        now: DateTime<Utc>,
    ) -> Result<bool, MergeQueueError>;

    /// Set conflict info on a queue entry.
    async fn set_entry_conflict(
        &self,
        owner: String,
        repo: String,
        pr_number: u32,
        conflict_json: String,
        now: DateTime<Utc>,
    ) -> Result<bool, MergeQueueError>;

    /// Update the position of a queue entry by ID.
    async fn update_entry_position(
        &self,
        id: Uuid,
        position: u32,
        now: DateTime<Utc>,
    ) -> Result<(), MergeQueueError>;

    /// Reset in_progress entries back to pending.
    async fn reset_interrupted(
        &self,
        owner: String,
        repo: String,
        now: DateTime<Utc>,
    ) -> Result<u32, MergeQueueError>;

    /// Delete merged/skipped entries older than cutoff.
    async fn cleanup_old(
        &self,
        owner: String,
        repo: String,
        cutoff: DateTime<Utc>,
    ) -> Result<u32, MergeQueueError>;
}

// ============================================================================
// Observability Repository (LLM calls + decisions)
// ============================================================================

/// Database operations for LLM call and decision logging.
#[cfg_attr(test, mockall::automock)]
#[async_trait]
pub trait ObservabilityRepo: Send + Sync {
    /// Insert an LLM call record.
    async fn insert_llm_call(&self, call: LlmCall) -> Result<()>;

    /// Get all LLM calls for a task.
    async fn get_calls_for_task(&self, task_id: Uuid) -> Result<Vec<LlmCall>>;

    /// Get LLM calls within a time range.
    async fn get_calls_in_range(
        &self,
        start: DateTime<Utc>,
        end: DateTime<Utc>,
    ) -> Result<Vec<LlmCall>>;

    /// Insert a decision record.
    async fn insert_decision(&self, decision: Decision) -> Result<()>;

    /// Get all decisions for a task.
    async fn get_decisions_for_task(&self, task_id: Uuid) -> Result<Vec<Decision>>;

    /// Get decisions within a time range.
    async fn get_decisions_in_range(
        &self,
        start: DateTime<Utc>,
        end: DateTime<Utc>,
    ) -> Result<Vec<Decision>>;
}

// ============================================================================
// Dependency Repository
// ============================================================================

/// Database operations for task dependency tracking.
#[cfg_attr(test, mockall::automock)]
#[async_trait]
pub trait DependencyRepo: Send + Sync {
    /// Get the status of a task by ID.
    async fn get_task_status(&self, id: TaskId) -> Result<Option<TaskStatus>, DependencyError>;

    /// Get all task IDs that depend on the given task.
    async fn get_blocked_by(&self, task_id: TaskId) -> Result<Vec<TaskId>, DependencyError>;

    /// Get the dependency IDs of a task.
    async fn get_task_dependencies(&self, task_id: TaskId) -> Result<Vec<TaskId>, DependencyError>;

    /// Save a single dependency.
    async fn save_dependency(
        &self,
        task_id: TaskId,
        depends_on: TaskId,
        now: DateTime<Utc>,
    ) -> Result<(), DependencyError>;

    /// Remove a dependency.
    async fn remove_dependency(
        &self,
        task_id: TaskId,
        depends_on: TaskId,
    ) -> Result<(), DependencyError>;

    /// Get all pending task IDs whose dependencies are all completed.
    async fn get_ready_task_ids(&self) -> Result<Vec<TaskId>, DependencyError>;
}

// ============================================================================
// Task Queue Repository
// ============================================================================

/// Database operations for the persistent task queue.
#[cfg_attr(test, mockall::automock)]
#[async_trait]
pub trait TaskQueueRepo: Send + Sync {
    /// List all tasks with the given status.
    async fn list_tasks_by_status(&self, status: TaskStatus) -> Result<Vec<Task>, TaskQueueError>;

    /// Update a task's status.
    async fn update_task_status(
        &self,
        id: TaskId,
        status: TaskStatus,
    ) -> Result<(), TaskQueueError>;

    /// Update a task for requeue (status, priority, updated_at) and log a task event.
    async fn update_task_for_requeue(
        &self,
        task_id: TaskId,
        priority_str: String,
        policy_description: String,
        now: DateTime<Utc>,
    ) -> Result<(), TaskQueueError>;
}

// ============================================================================
// Cost Repository
// ============================================================================

/// Database operations for cost tracking.
#[cfg_attr(test, mockall::automock)]
#[async_trait]
pub trait CostRepo: Send + Sync {
    /// Persist a cost record.
    async fn persist_cost_record(&self, record: CostRecord) -> Result<(), String>;

    /// Get all cost records, optionally filtered by timestamp.
    async fn get_cost_records(
        &self,
        since: Option<DateTime<Utc>>,
    ) -> Result<Vec<CostRecord>, String>;
}

// ============================================================================
// Planner Repository
// ============================================================================

/// Database operations for the planner (saving decomposition output).
#[cfg_attr(test, mockall::automock)]
#[async_trait]
pub trait PlannerRepo: Send + Sync {
    /// Save a planner output (slices + tasks) in a single transaction.
    async fn save_planner_output(
        &self,
        output: crate::orchestration::PlannerOutput,
    ) -> Result<(), String>;
}

// ============================================================================
// Scheduler Repository
// ============================================================================

/// Database operations for the scheduler (production mode).
#[cfg_attr(test, mockall::automock)]
#[async_trait]
pub trait SchedulerRepo: Send + Sync {
    /// Get the current production mode.
    async fn get_production_mode(&self) -> Result<ProductionMode, anyhow::Error>;

    /// Set the production mode.
    async fn set_production_mode(&self, mode: ProductionMode) -> Result<(), anyhow::Error>;
}

// ============================================================================
// Refactor Repository
// ============================================================================

/// Database operations for refactor sessions and changes.
#[cfg_attr(test, mockall::automock)]
#[async_trait]
pub trait RefactorRepo: Send + Sync {
    /// Get the currently active refactor session (if any).
    async fn get_active_refactor_session(&self) -> Result<Option<RefactorSession>>;

    /// Insert a new refactor session.
    async fn insert_refactor_session(&self, session: RefactorSession) -> Result<()>;

    /// Update a refactor session.
    async fn update_refactor_session(&self, session: RefactorSession) -> Result<()>;

    /// Insert a refactor change.
    async fn insert_refactor_change(&self, change: RefactorChange) -> Result<()>;

    /// Update the status of a refactor change.
    async fn update_change_status(&self, id: ChangeId, status: ChangeStatus) -> Result<()>;
}

// ============================================================================
// Server Repository (API handler DB operations)
// ============================================================================

/// Database operations used by the HTTP API handlers.
#[cfg_attr(test, mockall::automock)]
#[async_trait]
pub trait ServerRepo: Send + Sync {
    /// Check database connectivity (returns true if alive).
    async fn health_check(&self) -> bool;

    /// List tasks with optional status filter and limit.
    async fn list_tasks(&self, status: Option<String>, limit: Option<u32>) -> Result<Vec<Task>>;

    /// Get a single task by UUID.
    async fn get_task_by_uuid(&self, id: Uuid) -> Result<Option<Task>>;

    /// Insert a new task.
    async fn insert_task(&self, task: Task) -> Result<()>;

    /// Insert a chat message.
    async fn insert_chat_message(&self, id: Uuid, role: String, content: String) -> Result<()>;

    /// Get chat history with pagination.
    async fn get_chat_history(&self, limit: u32, offset: u32) -> Result<Vec<ChatMessageRow>>;

    /// Clear all chat history.
    async fn clear_chat_history(&self) -> Result<()>;

    /// Check if a password has been configured.
    async fn has_password(&self) -> Result<bool>;

    /// Store the password hash.
    async fn set_password(&self, password_hash: String) -> Result<()>;

    /// Get the stored password hash.
    async fn get_password(&self) -> Result<Option<String>>;
}
