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
    pub model_id: String,
    pub status: String,
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
    pub agent_id: Uuid,
    pub role: Option<String>,
    pub approval_required: bool,
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

/// Row type for token usage summary.
#[derive(Debug, Clone, serde::Serialize, sqlx::FromRow)]
pub struct UsageSummaryRow {
    pub tier: String,
    pub model_id: String,
    pub total_input: i64,
    pub total_output: i64,
    pub call_count: i64,
}

/// Type alias for the database pool
pub type DbPool = PgPool;

/// Initialize the database using DATABASE_URL from environment
pub async fn init_db() -> Result<PgPool> {
    let database_url =
        std::env::var("DATABASE_URL").context("DATABASE_URL environment variable not set")?;
    init_db_with_url(&database_url).await
}

/// Initialize the database with an explicit URL
pub async fn init_db_with_url(database_url: &str) -> Result<PgPool> {
    let pool = PgPoolOptions::new()
        .max_connections(10)
        .connect(database_url)
        .await
        .with_context(|| format!("Failed to connect to database at {}", database_url))?;

    tracing::info!("Database connected to PostgreSQL");

    // Run migrations
    sqlx::migrate!()
        .run(&pool)
        .await
        .context("Failed to run database migrations")?;

    tracing::info!("All migrations complete");

    Ok(pool)
}
