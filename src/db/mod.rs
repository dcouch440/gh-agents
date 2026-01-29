//! Database initialization and connection management

mod migrations;
pub mod prd;
mod queries;
mod refactor;

pub use migrations::run_migrations;
pub use queries::*;
pub use refactor::*;

use anyhow::{Context, Result};
use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};
use sqlx::SqlitePool;
use std::path::Path;
use std::str::FromStr;

/// Default database path
pub const DB_PATH: &str = ".nexor/state.db";

/// Initialize the database, creating the file and directory if needed
pub async fn init_db() -> Result<SqlitePool> {
    init_db_at(DB_PATH).await
}

/// Initialize the database at a specific path
pub async fn init_db_at(path: &str) -> Result<SqlitePool> {
    // Ensure parent directory exists
    if let Some(parent) = Path::new(path).parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("Failed to create directory {:?}", parent))?;
    }

    // Build connection options
    let options = SqliteConnectOptions::from_str(&format!("sqlite:{}", path))?
        .create_if_missing(true)
        .journal_mode(sqlx::sqlite::SqliteJournalMode::Wal)
        .synchronous(sqlx::sqlite::SqliteSynchronous::Normal);

    // Create connection pool
    let pool = SqlitePoolOptions::new()
        .max_connections(5)
        .connect_with(options)
        .await
        .with_context(|| format!("Failed to connect to database at {}", path))?;

    tracing::info!("Database initialized at {}", path);

    // Run migrations
    run_migrations(&pool).await?;

    Ok(pool)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[tokio::test]
    async fn init_creates_database_file() {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("test.db");
        let db_path_str = db_path.to_str().unwrap();

        let pool = init_db_at(db_path_str).await.unwrap();

        assert!(db_path.exists());
        pool.close().await;
    }

    #[tokio::test]
    async fn init_creates_parent_directory() {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("subdir").join("test.db");
        let db_path_str = db_path.to_str().unwrap();

        let pool = init_db_at(db_path_str).await.unwrap();

        assert!(db_path.exists());
        pool.close().await;
    }
}
