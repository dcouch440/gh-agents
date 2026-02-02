//! Repository traits for database operations.
//!
//! Each trait abstracts the DB operations for a specific domain module.
//! Production code uses `PgRepo` (see `pg_repo.rs`). Tests use `MockXxxRepo` from mockall.

use anyhow::Result;
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use uuid::Uuid;

use crate::db::{
    AgentExecutionRow, AgentRow, ChatMessageRow, ContextStoreRow, DocumentRow, DocumentSearchResult, ExecutionMessageRow, OutputSchemaRow, PipelineRow, PipelineRunRow, PipelineStageMemberRow,
    PipelineStageRow, PromptTemplateRow, ResultRow, RouterRequestRow, SessionRow, StageExecutionRow, StepDocumentRow, TokenLedgerRow, ToolRouterRow, ToolRow, WorkflowRow, WorkflowStepEdgeRow,
    WorkflowStepRow,
};
use crate::github::{PrQueueEntry, QueueError as MergeQueueError};
use crate::types::{Task, User, UserId};

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

// ============================================================================
// Pipeline Stage Member Repository
// ============================================================================

/// Database operations for pipeline stage members.
#[cfg_attr(test, mockall::automock)]
#[async_trait]
pub trait PipelineStageMemberRepo: Send + Sync {
    async fn list_stage_members(&self, pipeline_id: Uuid, stage_number: i32) -> Result<Vec<PipelineStageMemberRow>>;
    async fn add_stage_member(&self, pipeline_id: Uuid, stage_number: i32, workflow_id: Uuid, display_order: i32) -> Result<PipelineStageMemberRow>;
    async fn remove_stage_member(&self, member_id: Uuid) -> Result<()>;
    async fn update_stage_member(&self, member_id: Uuid, display_order: i32) -> Result<PipelineStageMemberRow>;
}

// ============================================================================
// Agent Execution Repository
// ============================================================================

/// Database operations for agent executions and execution messages.
#[cfg_attr(test, mockall::automock)]
#[async_trait]
pub trait AgentExecutionRepo: Send + Sync {
    // --- Agent Executions ---
    async fn create_agent_execution(
        &self,
        stage_execution_id: Uuid,
        agent_id: Uuid,
        workflow_step_id: Option<Uuid>,
        is_interactive: bool,
        parent_agent_execution_id: Option<Uuid>,
        system_prompt_rendered: &str,
        input: &str,
    ) -> Result<AgentExecutionRow>;
    async fn get_agent_execution(&self, id: Uuid) -> Result<Option<AgentExecutionRow>>;
    async fn list_agent_executions_by_stage(&self, stage_execution_id: Uuid) -> Result<Vec<AgentExecutionRow>>;
    async fn update_agent_execution_status(
        &self,
        id: Uuid,
        status: &str,
        output: Option<String>,
        structured_output: Option<serde_json::Value>,
        input_tokens: i64,
        output_tokens: i64,
        cost_usd: f32,
    ) -> Result<AgentExecutionRow>;

    // --- Execution Messages ---
    async fn create_execution_message(&self, agent_execution_id: Uuid, role: &str, content: &str, tool_call_id: Option<String>, input_tokens: i64, output_tokens: i64) -> Result<ExecutionMessageRow>;
    async fn list_execution_messages(&self, agent_execution_id: Uuid) -> Result<Vec<ExecutionMessageRow>>;
}

// ============================================================================
// Token Ledger Repository
// ============================================================================

/// Aggregated spend by model.
#[derive(Debug, Clone, serde::Serialize, sqlx::FromRow)]
pub struct ModelSpendRow {
    pub model_id: String,
    pub total_input_tokens: i64,
    pub total_output_tokens: i64,
    pub total_cost_usd: f64,
    pub call_count: i64,
}

/// Database operations for token cost tracking.
#[cfg_attr(test, mockall::automock)]
#[async_trait]
pub trait TokenLedgerRepo: Send + Sync {
    async fn insert_ledger_entry(&self, user_id: Uuid, agent_execution_id: Uuid, model_id: &str, input_tokens: i64, output_tokens: i64, cost_usd: f32) -> Result<TokenLedgerRow>;
    async fn get_user_spend(&self, user_id: Uuid, since: Option<DateTime<Utc>>) -> Result<f64>;
    async fn get_run_spend(&self, run_id: Uuid) -> Result<f64>;
    async fn get_model_breakdown(&self, user_id: Uuid, since: Option<DateTime<Utc>>) -> Result<Vec<ModelSpendRow>>;
}

// ============================================================================
// Result Repository
// ============================================================================

/// Database operations for saved structured results.
#[cfg_attr(test, mockall::automock)]
#[async_trait]
pub trait ResultRepo: Send + Sync {
    async fn save_result(&self, user_id: Uuid, agent_execution_id: Uuid, output_schema_id: Option<Uuid>, name: &str, data: serde_json::Value) -> Result<ResultRow>;
    async fn get_result(&self, id: Uuid) -> Result<Option<ResultRow>>;
    async fn list_results(&self, user_id: Uuid) -> Result<Vec<ResultRow>>;
    async fn list_results_by_schema(&self, user_id: Uuid, output_schema_id: Uuid) -> Result<Vec<ResultRow>>;
    async fn delete_result(&self, id: Uuid) -> Result<()>;
}

// ============================================================================
// Tool Router Repository
// ============================================================================

/// Database operations for tool router management.
#[cfg_attr(test, mockall::automock)]
#[async_trait]
pub trait ToolRouterRepo: Send + Sync {
    /// List all tool routers for a user.
    async fn list_tool_routers(&self, user_id: Uuid) -> Result<Vec<ToolRouterRow>>;
    /// Get a tool router by ID.
    async fn get_tool_router(&self, id: Uuid) -> Result<Option<ToolRouterRow>>;
    /// Create a new tool router.
    async fn create_tool_router(&self, user_id: Uuid, name: &str, description: Option<&str>, system_prompt: &str, model_id: &str) -> Result<ToolRouterRow>;
    /// Update a tool router.
    async fn update_tool_router(&self, id: Uuid, name: Option<&str>, description: Option<&str>, system_prompt: Option<&str>, model_id: Option<&str>, is_active: Option<bool>) -> Result<ToolRouterRow>;
    /// Delete a tool router.
    async fn delete_tool_router(&self, id: Uuid) -> Result<()>;
    /// Get all tools assigned to a router.
    async fn get_router_tools(&self, router_id: Uuid) -> Result<Vec<ToolRow>>;
    /// Set the full tool list for a router (replaces existing).
    async fn set_router_tools(&self, router_id: Uuid, tool_ids: &[Uuid]) -> Result<()>;
}

// ============================================================================
// Context Store Repository
// ============================================================================

/// Database operations for the per-session context store.
#[cfg_attr(test, mockall::automock)]
#[async_trait]
pub trait ContextStoreRepo: Send + Sync {
    /// Add a context entry to a session.
    async fn add_context(&self, session_id: Uuid, source: &str, priority: f32, content: &str, metadata: Option<serde_json::Value>, expires_at: Option<DateTime<Utc>>) -> Result<ContextStoreRow>;
    /// Get active context for a session, ordered by priority descending.
    async fn get_active_context(&self, session_id: Uuid, limit: u32) -> Result<Vec<ContextStoreRow>>;
    /// Update the status of a context entry.
    async fn update_context_status(&self, id: Uuid, status: &str) -> Result<()>;
    /// Expire stale context entries (past expires_at). Returns count expired.
    async fn expire_stale_context(&self, session_id: Uuid) -> Result<u32>;
}

// ============================================================================
// Router Request Repository
// ============================================================================

/// Database operations for router request logging.
#[cfg_attr(test, mockall::automock)]
#[async_trait]
pub trait RouterRequestRepo: Send + Sync {
    /// Create a new router request log entry.
    async fn create_router_request(&self, session_id: Uuid, agent_execution_id: Option<Uuid>, intent: &str, priority: &str, callback_hint: Option<&str>) -> Result<RouterRequestRow>;
    /// Update a router request with routing decision and result.
    async fn update_router_request(&self, id: Uuid, routed_tool: Option<&str>, routed_args: Option<serde_json::Value>, is_async: bool, passdown: Option<&str>, chain: Option<serde_json::Value>, status: &str, result: Option<&str>) -> Result<RouterRequestRow>;
    /// Get a router request by ID.
    async fn get_router_request(&self, id: Uuid) -> Result<Option<RouterRequestRow>>;
    /// List all router requests for a session.
    async fn list_session_requests(&self, session_id: Uuid) -> Result<Vec<RouterRequestRow>>;
}
