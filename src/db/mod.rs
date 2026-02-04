//! Database initialization and connection management

pub mod pg_repo;
mod queries;
#[cfg(test)]
pub mod test_utils;
pub mod traits;

use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
pub use queries::*;
use sqlx::postgres::PgPoolOptions;
use sqlx::PgPool;
use uuid::Uuid;

/// Row type for persisted agent definitions.
#[derive(Debug, Clone)]
pub struct AgentRow {
    pub id: Uuid,
    pub tier: Option<String>,
    pub name: String,
    pub system_prompt: String,
    pub persona_style: Option<String>,
    pub model_provider: String,
    pub model_id: String,
    pub model_max_tokens: i32,
    pub model_temperature: f32,
    pub status: Option<String>,
    pub router_mode: Option<bool>,
    pub output_schema_id: Option<Uuid>,
    pub version: i32,
}

/// Row type for persisted tool definitions.
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct ToolRow {
    pub id: Uuid,
    pub user_id: Uuid,
    pub name: String,
    pub display_name: String,
    pub description: String,
    pub parameters: serde_json::Value,
    pub created_at: DateTime<Utc>,
    pub version: i32,
}

/// Row type for persisted clusters.
#[derive(Debug, Clone)]
pub struct ClusterRow {
    pub id: Uuid,
    pub name: String,
    pub description: String,
    pub conventions: String,
    pub shared_files: serde_json::Value,
}

/// Row type for persisted documents.
#[derive(Debug, Clone, serde::Serialize, sqlx::FromRow)]
pub struct DocumentRow {
    pub id: Uuid,
    pub user_id: Uuid,
    pub session_id: Option<Uuid>,
    pub title: String,
    pub content: String,
    pub summary: Option<String>,
    pub doc_type: Option<String>,
    pub ref_tag: Option<String>,
    pub tags: Option<Vec<String>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Search result for documents (no full content).
#[derive(Debug, Clone, serde::Serialize, sqlx::FromRow, utoipa::ToSchema)]
pub struct DocumentSearchResult {
    pub id: Uuid,
    pub title: String,
    pub summary: Option<String>,
    pub ref_tag: Option<String>,
    pub snippet: String,
}

/// Row type for persisted output schema definitions.
#[derive(Debug, Clone, serde::Serialize, sqlx::FromRow)]
pub struct OutputSchemaRow {
    pub id: Uuid,
    pub user_id: Uuid,
    pub name: String,
    pub schema: serde_json::Value,
    pub created_at: DateTime<Utc>,
    pub version: i32,
}

/// Row type for persisted prompt template definitions.
#[derive(Debug, Clone, serde::Serialize, sqlx::FromRow)]
pub struct PromptTemplateRow {
    pub id: Uuid,
    pub user_id: Uuid,
    pub name: String,
    pub content: String,
    pub created_at: DateTime<Utc>,
    pub version: i32,
}

/// Row type for persisted workflow definitions.
#[derive(Debug, Clone, serde::Serialize, sqlx::FromRow)]
pub struct WorkflowRow {
    pub id: Uuid,
    pub user_id: Uuid,
    pub name: String,
    pub description: String,
    pub execution_mode: String,
    pub created_at: DateTime<Utc>,
    pub version: i32,
}

/// Row type for a workflow step (DAG node).
#[derive(Debug, Clone, serde::Serialize, sqlx::FromRow)]
pub struct WorkflowStepRow {
    pub id: Uuid,
    pub workflow_id: Uuid,
    pub agent_id: Uuid,
    pub execution_mode: String,               // "single", "for_each", or "room"
    pub agent_execution_mode: Option<String>, // "sequential" or "parallel", NULL = inherit from workflow
    pub for_each_ref: Option<String>,
    pub prompt_template_id: Option<Uuid>,
    pub prompt_template: String,
    pub output_schema_id: Option<Uuid>,
    pub output_variable_name: Option<String>,
    pub interactive_agent_id: Option<Uuid>,
    pub for_each_label_field: Option<String>,
    pub room_id: Option<Uuid>,
    pub display_order: i32,
    pub version: i32,
}

/// Row type for a workflow step edge (DAG edge).
#[derive(Debug, Clone, serde::Serialize, sqlx::FromRow)]
pub struct WorkflowStepEdgeRow {
    pub from_step_id: Uuid,
    pub to_step_id: Uuid,
}

/// Row type for workflow collections (DAG of workflows).
#[derive(Debug, Clone, serde::Serialize, sqlx::FromRow)]
pub struct WorkflowCollectionRow {
    pub id: Uuid,
    pub user_id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub execution_mode: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Row type for collection workflows (which workflows belong to a collection).
#[derive(Debug, Clone, serde::Serialize, sqlx::FromRow)]
pub struct CollectionWorkflowRow {
    pub collection_id: Uuid,
    pub workflow_id: Uuid,
    pub display_order: i32,
    pub execution_mode: Option<String>,
}

/// Row type for collection workflow edges (DAG edges between workflows).
#[derive(Debug, Clone, serde::Serialize, sqlx::FromRow)]
pub struct CollectionWorkflowEdgeRow {
    pub from_workflow_id: Uuid,
    pub to_workflow_id: Uuid,
    pub collection_id: Uuid,
}

/// Row type for collection runs (execution tracking).
#[derive(Debug, Clone, serde::Serialize, sqlx::FromRow)]
pub struct CollectionRunRow {
    pub id: Uuid,
    pub collection_id: Uuid,
    pub user_id: Uuid,
    pub status: String,
    pub started_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
    pub error: Option<String>,
}

/// Row type for workflow executions (workflow-level execution within a collection run).
#[derive(Debug, Clone, serde::Serialize, sqlx::FromRow)]
pub struct WorkflowExecutionRow {
    pub id: Uuid,
    pub collection_run_id: Uuid,
    pub workflow_id: Uuid,
    pub user_id: Uuid,
    pub status: String,
    pub started_at: Option<DateTime<Utc>>,
    pub completed_at: Option<DateTime<Utc>>,
    pub outputs: Option<serde_json::Value>,
    pub error: Option<String>,
}

/// Row type for execution variables (for text editor variable capture).
#[derive(Debug, Clone, serde::Serialize, sqlx::FromRow)]
pub struct ExecutionVariableRow {
    pub id: Uuid,
    pub collection_run_id: Option<Uuid>,
    pub workflow_execution_id: Option<Uuid>,
    pub step_execution_id: Option<Uuid>,
    pub variable_name: String,
    pub variable_path: String,
    pub value: serde_json::Value,
    pub created_at: DateTime<Utc>,
}

/// Row type for workflow step agents (multi-agent step support).
#[derive(Debug, Clone, serde::Serialize, sqlx::FromRow)]
pub struct WorkflowStepAgentRow {
    pub step_id: Uuid,
    pub agent_id: Uuid,
    pub execution_strategy: String,
    pub agent_order: i32,
}

/// Row type for a step-document attachment.
#[derive(Debug, Clone, serde::Serialize, sqlx::FromRow)]
pub struct StepDocumentRow {
    pub step_id: Uuid,
    pub document_id: Uuid,
}

/// Row type for agent execution records.
#[derive(Debug, Clone, serde::Serialize, sqlx::FromRow)]
pub struct AgentExecutionRow {
    pub id: Uuid,
    pub agent_id: Uuid,
    pub workflow_step_id: Option<Uuid>,
    pub workflow_execution_id: Option<Uuid>,
    pub is_interactive: bool,
    pub parent_agent_execution_id: Option<Uuid>,
    pub system_prompt_rendered: String,
    pub input: String,
    pub output: Option<String>,
    pub structured_output: Option<serde_json::Value>,
    pub selected_mode_id: Option<Uuid>,
    pub room_session_id: Option<Uuid>,
    pub speaker_order: Option<i32>,
    pub status: String,
    pub started_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
}

/// Row type for execution message records.
#[derive(Debug, Clone, serde::Serialize, sqlx::FromRow)]
pub struct ExecutionMessageRow {
    pub id: Uuid,
    pub agent_execution_id: Uuid,
    pub role: String,
    pub content: String,
    pub tool_call_id: Option<String>,
    pub input_tokens: i64,
    pub output_tokens: i64,
    pub created_at: DateTime<Utc>,
}

/// Row type for saved structured results.
#[derive(Debug, Clone, serde::Serialize, sqlx::FromRow)]
pub struct ResultRow {
    pub id: Uuid,
    pub user_id: Uuid,
    pub agent_execution_id: Uuid,
    pub output_schema_id: Option<Uuid>,
    pub name: String,
    pub data: serde_json::Value,
    pub created_at: DateTime<Utc>,
}

/// Row type for token ledger entries.
#[derive(Debug, Clone, serde::Serialize, sqlx::FromRow)]
pub struct TokenLedgerRow {
    pub id: Uuid,
    pub user_id: Uuid,
    pub agent_execution_id: Option<Uuid>,
    pub model_id: String,
    pub input_tokens: i64,
    pub output_tokens: i64,
    pub cost_usd: f32,
    pub created_at: DateTime<Utc>,
}

/// Row type for tool router definitions.
#[derive(Debug, Clone, serde::Serialize, sqlx::FromRow, utoipa::ToSchema)]
pub struct ToolRouterRow {
    pub id: Uuid,
    pub user_id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub system_prompt: String,
    pub model_id: String,
    pub is_active: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Row type for context store entries.
#[derive(Debug, Clone, serde::Serialize, sqlx::FromRow, utoipa::ToSchema)]
pub struct ContextStoreRow {
    pub id: Uuid,
    pub session_id: Uuid,
    pub source: String,
    pub priority: f32,
    pub content: String,
    pub metadata: Option<serde_json::Value>,
    pub status: String,
    pub created_at: DateTime<Utc>,
    pub expires_at: Option<DateTime<Utc>>,
}

/// Row type for router request logs.
#[derive(Debug, Clone, serde::Serialize, sqlx::FromRow, utoipa::ToSchema)]
pub struct RouterRequestRow {
    pub id: Uuid,
    pub session_id: Uuid,
    pub agent_execution_id: Option<Uuid>,
    pub intent: String,
    pub priority: String,
    pub callback_hint: Option<String>,
    pub routed_tool: Option<String>,
    pub routed_args: Option<serde_json::Value>,
    pub is_async: bool,
    pub passdown: Option<String>,
    pub chain: Option<serde_json::Value>,
    pub status: String,
    pub result: Option<String>,
    pub created_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
}

/// Row type for room definitions (pipeline-scoped).
#[derive(Debug, Clone, serde::Serialize, sqlx::FromRow)]
pub struct RoomRow {
    pub id: Uuid,
    pub user_id: Uuid,
    pub collection_id: Option<Uuid>,
    pub name: String,
    pub gatekeeper_enabled: bool,
    pub gatekeeper_model_id: String,
    pub max_speakers_per_turn: i32,
    pub max_turns: i32,
    pub tools_enabled: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Row type for room membership (join table).
#[derive(Debug, Clone, serde::Serialize, sqlx::FromRow)]
pub struct RoomMemberRow {
    pub room_id: Uuid,
    pub agent_id: Uuid,
    pub display_name: Option<String>,
    pub role_description: String,
    pub display_order: i32,
}

/// Row type for room session records (runtime).
#[derive(Debug, Clone, serde::Serialize, sqlx::FromRow)]
pub struct RoomSessionRow {
    pub id: Uuid,
    pub room_id: Uuid,
    pub status: String,
    pub current_turn: i32,
    pub transcript_summary: Option<String>,
    pub started_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
}

/// Labeled entry from a room transcript (cross-execution join).
#[derive(Debug, Clone, serde::Serialize, sqlx::FromRow)]
pub struct RoomTranscriptEntry {
    pub agent_name: String,
    pub role_description: String,
    pub content: String,
    pub speaker_order: Option<i32>,
    pub created_at: DateTime<Utc>,
}

/// Type alias for the database pool
pub type DbPool = PgPool;

/// Initialize the database using DATABASE_URL from environment
pub async fn init_db() -> Result<PgPool> {
    let database_url = std::env::var(crate::constants::ENV_DATABASE_URL).context(format!("{} environment variable not set", crate::constants::ENV_DATABASE_URL))?;
    init_db_with_url(&database_url).await
}

/// Initialize the database with an explicit URL
pub async fn init_db_with_url(database_url: &str) -> Result<PgPool> {
    let max_connections: u32 = std::env::var(crate::constants::ENV_DB_MAX_CONNECTIONS).ok().and_then(|s| s.parse().ok()).unwrap_or(10);
    tracing::info!("DB pool max_connections = {}", max_connections);

    let pool = PgPoolOptions::new()
        .max_connections(max_connections)
        .connect(database_url)
        .await
        .with_context(|| format!("Failed to connect to database at {}", database_url))?;

    tracing::info!("Database connected to PostgreSQL");

    // Run migrations
    sqlx::migrate!().run(&pool).await.context("Failed to run database migrations")?;

    tracing::info!("All migrations complete");

    Ok(pool)
}
