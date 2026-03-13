//! Database initialization and connection management

#[cfg(test)]
pub mod fixtures;
pub mod pg_repo;
mod queries;
#[cfg(test)]
pub mod test_utils;
pub mod traits;
pub mod types;

use anyhow::{Context, Result};
pub use queries::*;
use sqlx::postgres::PgPoolOptions;
use sqlx::PgPool;
pub use types::*;

/// Type alias for the database pool
pub type DbPool = PgPool;

/// Initialize the database using the centralized Env config.
pub async fn init_db(env: &crate::env::Env) -> Result<PgPool> {
    init_db_with_config(&env.database_url, env.db_max_connections).await
}

/// Initialize the database with an explicit URL and max connections.
pub async fn init_db_with_config(database_url: &str, max_connections: u32) -> Result<PgPool> {
    tracing::info!("DB pool max_connections = {}", max_connections);

    let pool = PgPoolOptions::new()
        .max_connections(max_connections)
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
