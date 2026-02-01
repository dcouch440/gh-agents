//! Database initialization and connection management

pub mod pg_repo;
pub mod prd;
mod queries;
mod refactor;
#[cfg(test)]
pub mod test_utils;
pub mod traits;

pub use queries::*;
pub use refactor::*;

use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use sqlx::postgres::PgPoolOptions;
use sqlx::PgPool;
use uuid::Uuid;

/// Row type for persisted agent definitions.
#[derive(Debug, Clone)]
pub struct AgentRow {
    pub id: Uuid,
    pub tier: String,
    pub persona_name: String,
    pub persona_prompt: String,
    pub persona_style: String,
    pub model_provider: String,
    pub model_id: String,
    pub model_max_tokens: i32,
    pub model_temperature: f32,
    pub status: String,
    pub router_mode: bool,
}

/// Row type for persisted pipeline definitions.
#[derive(Debug, Clone)]
pub struct PipelineRow {
    pub id: Uuid,
    pub name: String,
}

/// Row type for persisted pipeline stages.
#[derive(Debug, Clone)]
pub struct PipelineStageRow {
    pub pipeline_id: Uuid,
    pub stage_number: i32,
    pub agent_id: Option<Uuid>,
    pub cluster_id: Option<Uuid>,
    pub role: Option<String>,
    pub approval_required: bool,
    pub fan_out: bool,
    pub stage_name: String,
    pub input_definitions: serde_json::Value,
    pub output_description: String,
    pub output_schema: serde_json::Value,
}

/// Row type for persisted schedules.
#[derive(Debug, Clone)]
pub struct ScheduleRow {
    pub id: Uuid,
    pub name: String,
    pub agent_id: Uuid,
    pub interval_seconds: i32,
    pub task_title: String,
    pub task_description: String,
    pub role: Option<String>,
    pub enabled: bool,
    pub last_run_at: Option<DateTime<Utc>>,
}

/// Row type for persisted triggers.
#[derive(Debug, Clone)]
pub struct TriggerRow {
    pub id: Uuid,
    pub name: String,
    pub event_type: String,
    pub agent_id: Uuid,
    pub task_title: String,
    pub task_description: String,
    pub role: Option<String>,
}

/// Row type for persisted tool definitions.
#[derive(Debug, Clone)]
pub struct ToolRow {
    pub id: Uuid,
    pub name: String,
    pub description: String,
    pub category: String,
    pub parameter_schema: serde_json::Value,
    pub output_schema: serde_json::Value,
    pub enabled: bool,
    pub cluster_id: Option<Uuid>,
    pub is_builtin: bool,
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

/// Row type for persisted stage side tasks.
#[derive(Debug, Clone)]
pub struct StageSideTaskRow {
    pub id: Uuid,
    pub pipeline_id: Uuid,
    pub stage_number: i32,
    pub agent_id: Uuid,
    pub input_definitions: serde_json::Value,
    pub output_name: String,
    pub blocking: bool,
    pub output_schema: serde_json::Value,
}

/// Row type for persisted documents.
#[derive(Debug, Clone, serde::Serialize, sqlx::FromRow)]
pub struct DocumentRow {
    pub id: Uuid,
    pub user_id: Uuid,
    pub session_id: Option<Uuid>,
    pub title: String,
    pub content: String,
    pub summary: String,
    pub doc_type: String,
    pub ref_tag: String,
    pub tags: Vec<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Search result for documents (no full content).
#[derive(Debug, Clone, serde::Serialize, sqlx::FromRow)]
pub struct DocumentSearchResult {
    pub id: Uuid,
    pub title: String,
    pub summary: String,
    pub ref_tag: String,
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
}

/// Row type for token usage summary.
#[derive(Debug, Clone, serde::Serialize, sqlx::FromRow)]
pub struct UsageSummaryRow {
    pub tier: String,
    pub model_id: String,
    pub total_input: i64,
    pub total_output: i64,
    pub call_count: i64,
}

pub struct PipelineRunRow {
    pub id: Uuid,
    pub pipeline_id: Uuid,
    pub user_id: Uuid,
    pub status: String,
    pub initial_task: String,
    pub stage_outputs: serde_json::Value,
    pub current_stage: i32,
    pub started_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
    pub total_input_tokens: i64,
    pub total_output_tokens: i64,
}

pub struct StageExecutionRow {
    pub id: Uuid,
    pub run_id: Uuid,
    pub stage_number: i32,
    pub stage_name: String,
    pub agent_id: Option<Uuid>,
    pub status: String,
    pub rendered_prompt: Option<String>,
    pub output: Option<String>,
    pub structured_output: Option<serde_json::Value>,
    pub user_input: Option<String>,
    pub input_tokens: i64,
    pub output_tokens: i64,
    pub started_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
    pub duration_ms: i64,
}

/// Row type for routing events (tool routing observability and analytics).
#[derive(Debug, Clone)]
pub struct RoutingEventRow {
    pub id: Uuid,
    pub user_id: Option<Uuid>,
    pub session_id: Option<Uuid>,
    pub task_id: Option<Uuid>,
    pub router_agent_id: Uuid,
    pub cluster_agent_id: Option<Uuid>,
    pub cluster_id: Option<Uuid>,
    pub cluster_name: String,
    pub tool_name: String,
    pub request: String,
    pub parameters: serde_json::Value,
    pub response: Option<String>,
    pub error: Option<String>,
    pub status: String,
    pub agent_tier: Option<String>,
    pub model_id: Option<String>,
    pub input_tokens: i64,
    pub output_tokens: i64,
    pub duration_ms: Option<i64>,
    pub created_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
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
