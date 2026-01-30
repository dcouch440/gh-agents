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

/// Row type for persisted clusters.
#[derive(Debug, Clone)]
pub struct ClusterRow {
    pub id: Uuid,
    pub name: String,
    pub description: String,
    pub conventions: String,
    pub shared_files: serde_json::Value,
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
