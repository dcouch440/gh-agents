//! Repository traits for database operations.
//!
//! Each trait abstracts the DB operations for a specific domain module.
//! Production code uses `PgRepo` (see `pg_repo.rs`). Tests use `MockXxxRepo` from mockall.

use anyhow::Result;
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use uuid::Uuid;

use crate::db::{
    AgentRow, ChatMessageRow, ClusterRow, DocumentRow, DocumentSearchResult, OutputSchemaRow, PipelineRow, PipelineRunRow, PipelineStageRow, PromptTemplateRow, ScheduleRow, SessionRow,
    StageExecutionRow, StageSideTaskRow, StepDocumentRow, ToolRow, TriggerRow, UsageSummaryRow, WorkflowRow, WorkflowStepEdgeRow, WorkflowStepRow,
};
use crate::github::{PrQueueEntry, QueueError as MergeQueueError};
use crate::orchestration::DependencyError;
use crate::orchestration::QueueError as TaskQueueError;
use crate::types::{ChangeId, ChangeStatus, CostRecord, ProductionMode, RefactorChange, RefactorSession, Task, TaskId, TaskStatus, User, UserId};

// ============================================================================
// Merge Queue Repository
// ============================================================================

/// Database operations for the PR merge queue.
#[cfg_attr(test, mockall::automock)]
#[async_trait]
pub trait MergeQueueRepo: Send + Sync {
    /// Insert or update a queue entry (upsert).
    async fn insert_queue_entry(&self, id: Uuid, owner: String, repo: String, pr_number: u32, position: u32, now: DateTime<Utc>) -> Result<(), MergeQueueError>;

    /// Get the next queue position for a repo.
    async fn get_next_position(&self, owner: String, repo: String) -> Result<u32, MergeQueueError>;

    /// Delete a queue entry. Returns true if a row was deleted.
    async fn delete_queue_entry(&self, owner: String, repo: String, pr_number: u32) -> Result<bool, MergeQueueError>;

    /// Get all queue entries for a repo, ordered by position.
    async fn get_queue_entries(&self, owner: String, repo: String) -> Result<Vec<PrQueueEntry>, MergeQueueError>;

    /// Update the status (and optional error message) of a queue entry.
    async fn update_entry_status(&self, owner: String, repo: String, pr_number: u32, status: String, error_message: Option<String>, now: DateTime<Utc>) -> Result<bool, MergeQueueError>;

    /// Set conflict info on a queue entry.
    async fn set_entry_conflict(&self, owner: String, repo: String, pr_number: u32, conflict_json: String, now: DateTime<Utc>) -> Result<bool, MergeQueueError>;

    /// Update the position of a queue entry by ID.
    async fn update_entry_position(&self, id: Uuid, position: u32, now: DateTime<Utc>) -> Result<(), MergeQueueError>;

    /// Reset in_progress entries back to pending.
    async fn reset_interrupted(&self, owner: String, repo: String, now: DateTime<Utc>) -> Result<u32, MergeQueueError>;

    /// Delete merged/skipped entries older than cutoff.
    async fn cleanup_old(&self, owner: String, repo: String, cutoff: DateTime<Utc>) -> Result<u32, MergeQueueError>;
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
    async fn save_dependency(&self, task_id: TaskId, depends_on: TaskId, now: DateTime<Utc>) -> Result<(), DependencyError>;

    /// Remove a dependency.
    async fn remove_dependency(&self, task_id: TaskId, depends_on: TaskId) -> Result<(), DependencyError>;

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
    async fn update_task_status(&self, id: TaskId, status: TaskStatus) -> Result<(), TaskQueueError>;

    /// Update a task for requeue (status, priority, updated_at) and log a task event.
    async fn update_task_for_requeue(&self, task_id: TaskId, priority_str: String, policy_description: String, now: DateTime<Utc>) -> Result<(), TaskQueueError>;
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
    async fn get_cost_records(&self, since: Option<DateTime<Utc>>) -> Result<Vec<CostRecord>, String>;
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
    async fn list_tasks(&self, user_id: UserId, status: Option<String>, limit: Option<u32>) -> Result<Vec<Task>>;

    /// Get a single task by UUID.
    async fn get_task_by_uuid(&self, user_id: UserId, id: Uuid) -> Result<Option<Task>>;

    /// Insert a new task.
    async fn insert_task(&self, user_id: UserId, task: Task) -> Result<()>;

    /// Insert a chat message.
    async fn insert_chat_message(&self, user_id: UserId, id: Uuid, role: String, content: String) -> Result<()>;

    /// Get chat history with pagination.
    async fn get_chat_history(&self, user_id: UserId, limit: u32, offset: u32) -> Result<Vec<ChatMessageRow>>;

    /// Clear all chat history.
    async fn clear_chat_history(&self, user_id: UserId) -> Result<()>;

    /// Check if a password has been configured.
    async fn has_password(&self) -> Result<bool>;

    /// Store the password hash.
    async fn set_password(&self, password_hash: String) -> Result<()>;

    /// Get the stored password hash.
    async fn get_password(&self) -> Result<Option<String>>;

    // --- Agent persistence ---

    /// List all agents for a user.
    async fn list_persisted_agents(&self, user_id: UserId) -> Result<Vec<AgentRow>>;

    /// Insert or update an agent definition.
    async fn upsert_agent(&self, user_id: UserId, agent: AgentRow) -> Result<()>;

    /// Get a single agent by ID.
    async fn get_persisted_agent(&self, agent_id: Uuid) -> Result<Option<AgentRow>>;

    /// Delete an agent by ID.
    async fn delete_persisted_agent(&self, agent_id: Uuid) -> Result<()>;

    // --- Tool persistence ---

    /// List all tools for a user.
    async fn list_tools(&self, user_id: UserId) -> Result<Vec<ToolRow>>;

    /// Get a tool by ID.
    async fn get_tool(&self, tool_id: Uuid) -> Result<Option<ToolRow>>;

    /// Insert or update a tool.
    async fn upsert_tool(&self, user_id: UserId, tool: ToolRow) -> Result<()>;

    /// Delete a tool by ID.
    async fn delete_tool(&self, tool_id: Uuid) -> Result<()>;

    /// Get all tools assigned to an agent.
    async fn get_agent_tools(&self, agent_id: Uuid) -> Result<Vec<ToolRow>>;

    /// Set the full tool list for an agent (replaces existing).
    async fn set_agent_tools(&self, agent_id: Uuid, tool_ids: Vec<Uuid>) -> Result<()>;

    /// Seed the 11 built-in execution tools for a user. Idempotent.
    async fn seed_builtin_tools(&self, user_id: UserId) -> Result<()>;

    // --- Agent context (document linkage) ---

    /// Get all context documents assigned to an agent.
    async fn get_agent_context(&self, agent_id: Uuid) -> Result<Vec<DocumentRow>>;

    /// Set the full context document list for an agent (replaces existing).
    async fn set_agent_context(&self, agent_id: Uuid, document_ids: Vec<Uuid>) -> Result<()>;

    // --- Cluster persistence ---

    /// List all clusters for a user.
    async fn list_persisted_clusters(&self, user_id: UserId) -> Result<Vec<ClusterRow>>;

    /// Insert or update a cluster.
    async fn upsert_cluster(&self, user_id: UserId, cluster: ClusterRow) -> Result<()>;

    /// Delete a cluster by ID.
    async fn delete_cluster(&self, cluster_id: Uuid) -> Result<()>;

    /// List agent IDs in a cluster.
    async fn list_cluster_members(&self, cluster_id: Uuid) -> Result<Vec<Uuid>>;

    /// Add an agent to a cluster.
    async fn add_cluster_member(&self, cluster_id: Uuid, agent_id: Uuid) -> Result<()>;

    /// Remove an agent from a cluster.
    async fn remove_cluster_member(&self, cluster_id: Uuid, agent_id: Uuid) -> Result<()>;

    // --- Pipeline persistence ---

    /// List all pipelines for a user.
    async fn list_pipelines(&self, user_id: UserId) -> Result<Vec<PipelineRow>>;

    /// Insert or update a pipeline.
    async fn upsert_pipeline(&self, user_id: UserId, pipeline: PipelineRow) -> Result<()>;

    /// Delete a pipeline by ID.
    async fn delete_pipeline(&self, pipeline_id: Uuid) -> Result<()>;

    /// List stages for a pipeline.
    async fn list_pipeline_stages(&self, pipeline_id: Uuid) -> Result<Vec<PipelineStageRow>>;

    /// Insert or update a pipeline stage.
    async fn upsert_pipeline_stage(&self, stage: PipelineStageRow) -> Result<()>;

    // --- Stage side task persistence ---

    /// List side tasks for a pipeline stage.
    async fn list_stage_side_tasks(&self, pipeline_id: Uuid, stage_number: i32) -> Result<Vec<StageSideTaskRow>>;

    /// Insert or update a stage side task.
    async fn upsert_stage_side_task(&self, side_task: StageSideTaskRow) -> Result<()>;

    /// Delete a stage side task by ID.
    async fn delete_stage_side_task(&self, side_task_id: Uuid) -> Result<()>;

    // --- Schedule persistence ---

    /// List all schedules for a user.
    async fn list_schedules(&self, user_id: UserId) -> Result<Vec<ScheduleRow>>;

    /// Insert or update a schedule.
    async fn upsert_schedule(&self, user_id: UserId, schedule: ScheduleRow) -> Result<()>;

    /// Delete a schedule by ID.
    async fn delete_schedule(&self, schedule_id: Uuid) -> Result<()>;

    /// Update last_run_at for a schedule.
    async fn update_schedule_last_run(&self, schedule_id: Uuid, last_run_at: DateTime<Utc>) -> Result<()>;

    // --- Trigger persistence ---

    /// List all triggers for a user.
    async fn list_triggers(&self, user_id: UserId) -> Result<Vec<TriggerRow>>;

    /// Insert or update a trigger.
    async fn upsert_trigger(&self, user_id: UserId, trigger: TriggerRow) -> Result<()>;

    /// Delete a trigger by ID.
    async fn delete_trigger(&self, trigger_id: Uuid) -> Result<()>;

    // --- Session management ---

    /// Create a new chat session.
    async fn create_session(&self, user_id: UserId, session_id: Uuid, mode_id: &str, title: &str) -> Result<()>;

    /// List sessions for a user.
    async fn list_sessions(&self, user_id: UserId) -> Result<Vec<SessionRow>>;

    /// Get a session by ID.
    async fn get_session(&self, session_id: Uuid) -> Result<Option<SessionRow>>;

    /// Delete a session and its messages.
    async fn delete_session(&self, session_id: Uuid) -> Result<()>;

    /// Insert a chat message scoped to a session.
    async fn insert_session_message(&self, user_id: UserId, session_id: Uuid, id: Uuid, role: String, content: String) -> Result<()>;

    /// Get chat history for a session.
    async fn get_session_history(&self, session_id: Uuid, limit: u32) -> Result<Vec<ChatMessageRow>>;

    /// Update the title for a session.
    async fn update_session_title(&self, session_id: Uuid, title: &str) -> Result<()>;

    /// Update the summary for a session.
    async fn update_session_summary(&self, session_id: Uuid, summary: &str) -> Result<()>;

    /// Count messages in a session.
    async fn count_session_messages(&self, session_id: Uuid) -> Result<u32>;

    // --- Token usage tracking ---

    /// Insert a token usage record.
    async fn insert_token_usage(&self, session_id: Option<Uuid>, agent_id: Option<Uuid>, tier: &str, model_id: &str, input_tokens: i64, output_tokens: i64) -> Result<()>;

    /// Get aggregated usage summary for the last N hours.
    async fn get_usage_summary(&self, since_hours: u32) -> Result<Vec<UsageSummaryRow>>;

    // --- Pipeline run persistence ---

    /// Create a pipeline run record.
    async fn create_pipeline_run(&self, run: &PipelineRunRow) -> Result<()>;

    /// Update a pipeline run record.
    async fn update_pipeline_run(&self, run: &PipelineRunRow) -> Result<()>;

    /// Get a pipeline run by ID.
    async fn get_pipeline_run(&self, run_id: Uuid) -> Result<Option<PipelineRunRow>>;

    /// List runs for a pipeline.
    async fn list_pipeline_runs(&self, pipeline_id: Uuid) -> Result<Vec<PipelineRunRow>>;

    /// Create a stage execution record.
    async fn create_stage_execution(&self, exec: &StageExecutionRow) -> Result<()>;

    /// Update a stage execution record.
    async fn update_stage_execution(&self, exec: &StageExecutionRow) -> Result<()>;

    /// List stage executions for a run.
    async fn list_stage_executions(&self, run_id: Uuid) -> Result<Vec<StageExecutionRow>>;

    // --- Tool call logging ---

    /// Insert a tool call record.
    async fn insert_tool_call(
        &self,
        session_id: Option<Uuid>,
        message_id: Uuid,
        round: i32,
        tool_name: &str,
        tool_use_id: &str,
        input: &serde_json::Value,
        output: &str,
        latency_ms: i32,
    ) -> Result<()>;
}

// ============================================================================
// User Repository
// ============================================================================

/// Database operations for user management.
#[cfg_attr(test, mockall::automock)]
#[async_trait]
pub trait UserRepo: Send + Sync {
    /// Create a new user with email and password.
    async fn create_user(&self, email: &str, password_hash: &str) -> Result<User>;
    /// Get a user by email.
    async fn get_user_by_email(&self, email: &str) -> Result<Option<User>>;
    /// Get a user by ID.
    async fn get_user_by_id(&self, id: UserId) -> Result<Option<User>>;
    /// Get a user by GitHub ID.
    async fn get_user_by_github_id(&self, github_id: i64) -> Result<Option<User>>;
    /// Link GitHub account to existing user.
    async fn link_github(&self, user_id: UserId, github_id: i64, github_login: &str, token_encrypted: &str) -> Result<()>;
    /// Create a new user from GitHub OAuth.
    async fn create_github_user(&self, email: &str, github_id: i64, github_login: &str, token_encrypted: &str) -> Result<User>;
}

// ============================================================================
// Document Repository
// ============================================================================

/// Database operations for document management.
#[cfg_attr(test, mockall::automock)]
#[async_trait]
pub trait DocumentRepo: Send + Sync {
    /// Create a new document.
    async fn create_document(&self, user_id: Uuid, session_id: Option<Uuid>, title: String, content: String, doc_type: String, ref_tag: String, tags: Vec<String>) -> Result<DocumentRow>;

    /// Update a document's content, title, and tags.
    async fn update_document(&self, doc_id: Uuid, content: Option<String>, title: Option<String>, tags: Option<Vec<String>>) -> Result<DocumentRow>;

    /// Update a document's summary.
    async fn update_document_summary(&self, doc_id: Uuid, summary: String) -> Result<()>;

    /// Get a document by ID.
    async fn get_document(&self, doc_id: Uuid) -> Result<Option<DocumentRow>>;

    /// Get a document by ref_tag.
    async fn get_document_by_ref_tag(&self, ref_tag: &str) -> Result<Option<DocumentRow>>;

    /// List all documents for a user.
    async fn list_documents(&self, user_id: Uuid) -> Result<Vec<DocumentRow>>;

    /// List all documents for a session.
    async fn list_session_documents(&self, session_id: Uuid) -> Result<Vec<DocumentRow>>;

    /// Full-text search documents for a user.
    async fn search_documents(&self, user_id: Uuid, query: &str) -> Result<Vec<DocumentSearchResult>>;

    /// Delete a document by ID.
    async fn delete_document(&self, doc_id: Uuid) -> Result<()>;
}

// ============================================================================
// Output Schema Repository
// ============================================================================

/// Database operations for output schema management.
#[cfg_attr(test, mockall::automock)]
#[async_trait]
pub trait OutputSchemaRepo: Send + Sync {
    /// Create a new output schema.
    async fn create_output_schema(&self, user_id: Uuid, name: String, schema: serde_json::Value) -> Result<OutputSchemaRow>;

    /// Get an output schema by ID.
    async fn get_output_schema(&self, id: Uuid) -> Result<Option<OutputSchemaRow>>;

    /// List all output schemas for a user.
    async fn list_output_schemas(&self, user_id: Uuid) -> Result<Vec<OutputSchemaRow>>;

    /// Update an output schema's name and/or schema.
    async fn update_output_schema(&self, id: Uuid, name: Option<String>, schema: Option<serde_json::Value>) -> Result<OutputSchemaRow>;

    /// Delete an output schema by ID.
    async fn delete_output_schema(&self, id: Uuid) -> Result<()>;
}

// ============================================================================
// Prompt Template Repository
// ============================================================================

/// Database operations for prompt template management.
#[cfg_attr(test, mockall::automock)]
#[async_trait]
pub trait PromptTemplateRepo: Send + Sync {
    /// Create a new prompt template.
    async fn create_prompt_template(&self, user_id: Uuid, name: String, content: String) -> Result<PromptTemplateRow>;

    /// Get a prompt template by ID.
    async fn get_prompt_template(&self, id: Uuid) -> Result<Option<PromptTemplateRow>>;

    /// List all prompt templates for a user.
    async fn list_prompt_templates(&self, user_id: Uuid) -> Result<Vec<PromptTemplateRow>>;

    /// Update a prompt template's name and/or content.
    async fn update_prompt_template(&self, id: Uuid, name: Option<String>, content: Option<String>) -> Result<PromptTemplateRow>;

    /// Delete a prompt template by ID.
    async fn delete_prompt_template(&self, id: Uuid) -> Result<()>;
}

// ============================================================================
// Workflow Repository
// ============================================================================

/// Database operations for workflows, steps, edges, and step documents.
#[cfg_attr(test, mockall::automock)]
#[async_trait]
pub trait WorkflowRepo: Send + Sync {
    // --- Workflows ---
    async fn create_workflow(&self, user_id: Uuid, name: String, description: String) -> Result<WorkflowRow>;
    async fn get_workflow(&self, id: Uuid) -> Result<Option<WorkflowRow>>;
    async fn list_workflows(&self, user_id: Uuid) -> Result<Vec<WorkflowRow>>;
    async fn update_workflow(&self, id: Uuid, name: Option<String>, description: Option<String>) -> Result<WorkflowRow>;
    async fn delete_workflow(&self, id: Uuid) -> Result<()>;

    // --- Steps ---
    async fn create_step(&self, step: WorkflowStepRow) -> Result<WorkflowStepRow>;
    async fn get_step(&self, id: Uuid) -> Result<Option<WorkflowStepRow>>;
    async fn list_steps(&self, workflow_id: Uuid) -> Result<Vec<WorkflowStepRow>>;
    async fn update_step(&self, step: WorkflowStepRow) -> Result<WorkflowStepRow>;
    async fn delete_step(&self, id: Uuid) -> Result<()>;

    // --- Edges ---
    async fn set_edges(&self, workflow_id: Uuid, edges: Vec<WorkflowStepEdgeRow>) -> Result<()>;
    async fn list_edges(&self, workflow_id: Uuid) -> Result<Vec<WorkflowStepEdgeRow>>;
    async fn add_edge(&self, from_step_id: Uuid, to_step_id: Uuid) -> Result<()>;
    async fn remove_edge(&self, from_step_id: Uuid, to_step_id: Uuid) -> Result<()>;

    // --- Step documents ---
    async fn list_step_documents(&self, step_id: Uuid) -> Result<Vec<StepDocumentRow>>;
    async fn add_step_document(&self, step_id: Uuid, document_id: Uuid) -> Result<()>;
    async fn remove_step_document(&self, step_id: Uuid, document_id: Uuid) -> Result<()>;
}
