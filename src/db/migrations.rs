//! Database migrations

use anyhow::{Context, Result};
use sqlx::SqlitePool;

/// Run all database migrations
pub async fn run_migrations(pool: &SqlitePool) -> Result<()> {
    tracing::info!("Running database migrations...");

    // Create migrations table if not exists
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS _migrations (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            name TEXT NOT NULL UNIQUE,
            applied_at TEXT NOT NULL DEFAULT (datetime('now'))
        )
        "#,
    )
    .execute(pool)
    .await
    .context("Failed to create migrations table")?;

    // Run each migration
    run_migration(
        pool,
        "001_create_tasks",
        include_str!("../../migrations/001_create_tasks.sql"),
    )
    .await?;
    run_migration(
        pool,
        "002_create_task_events",
        include_str!("../../migrations/002_create_task_events.sql"),
    )
    .await?;
    run_migration(
        pool,
        "003_create_agents",
        include_str!("../../migrations/003_create_agents.sql"),
    )
    .await?;
    run_migration(
        pool,
        "004_create_messages",
        include_str!("../../migrations/004_create_messages.sql"),
    )
    .await?;
    run_migration(
        pool,
        "005_create_cost_records",
        include_str!("../../migrations/005_create_cost_records.sql"),
    )
    .await?;
    run_migration(
        pool,
        "006_create_tickets",
        include_str!("../../migrations/006_create_tickets.sql"),
    )
    .await?;
    run_migration(
        pool,
        "007_create_refactors",
        include_str!("../../migrations/007_create_refactors.sql"),
    )
    .await?;
    run_migration(
        pool,
        "008_add_task_metadata",
        include_str!("../../migrations/008_add_task_metadata.sql"),
    )
    .await?;
    run_migration(
        pool,
        "009_add_task_dependencies",
        include_str!("../../migrations/009_add_task_dependencies.sql"),
    )
    .await?;
    run_migration(
        pool,
        "010_create_pr_merge_queue",
        include_str!("../../migrations/010_create_pr_merge_queue.sql"),
    )
    .await?;
    run_migration(
        pool,
        "011_create_observability_tables",
        include_str!("../../migrations/011_create_observability_tables.sql"),
    )
    .await?;
    run_migration(
        pool,
        "012_create_chat_messages",
        include_str!("../../migrations/012_create_chat_messages.sql"),
    )
    .await?;
    run_migration(
        pool,
        "013_create_auth_tables",
        include_str!("../../migrations/013_create_auth_tables.sql"),
    )
    .await?;
    run_migration(
        pool,
        "014_add_prds",
        include_str!("../../migrations/014_add_prds.sql"),
    )
    .await?;

    tracing::info!("All migrations complete");
    Ok(())
}

async fn run_migration(pool: &SqlitePool, name: &str, sql: &str) -> Result<()> {
    // Check if already applied
    let applied: Option<(i32,)> = sqlx::query_as("SELECT 1 FROM _migrations WHERE name = ?")
        .bind(name)
        .fetch_optional(pool)
        .await?;

    if applied.is_some() {
        tracing::debug!("Migration {} already applied, skipping", name);
        return Ok(());
    }

    // Run migration
    tracing::info!("Applying migration: {}", name);

    // Execute each statement separately (SQLite doesn't support multiple statements in one query)
    for statement in sql.split(';') {
        let statement = statement.trim();
        if !statement.is_empty() {
            sqlx::query(statement)
                .execute(pool)
                .await
                .with_context(|| format!("Failed to execute migration {}: {}", name, statement))?;
        }
    }

    // Record migration
    sqlx::query("INSERT INTO _migrations (name) VALUES (?)")
        .bind(name)
        .execute(pool)
        .await?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[tokio::test]
    async fn migrations_are_idempotent() {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("test.db");

        let pool = crate::db::init_db_at(db_path.to_str().unwrap())
            .await
            .unwrap();

        // Run migrations again - should not fail
        let result = run_migrations(&pool).await;
        assert!(result.is_ok());

        pool.close().await;
    }
}
