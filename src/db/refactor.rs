//! Database queries for refactor mode.
//!
//! Handles persistence of:
//! - System production mode state
//! - Refactor sessions
//! - Proposed changes

use anyhow::{Context, Result};
use chrono::Utc;
use sqlx::SqlitePool;
use std::str::FromStr;

use crate::types::{
    ChangeId, ChangeStatus, ChangeType, ProductionMode, RefactorChange, RefactorId, RefactorSession,
};

// =============================================================================
// System State
// =============================================================================

/// Get the current production mode
pub async fn get_production_mode(pool: &SqlitePool) -> Result<ProductionMode> {
    let row: Option<(String,)> =
        sqlx::query_as("SELECT value FROM system_state WHERE key = 'production_mode'")
            .fetch_optional(pool)
            .await
            .context("Failed to fetch production mode")?;

    Ok(row
        .map(|(v,)| ProductionMode::from_str(&v))
        .unwrap_or_default())
}

/// Set the production mode
pub async fn set_production_mode(pool: &SqlitePool, mode: ProductionMode) -> Result<()> {
    let value = mode.as_str();
    let updated_at = Utc::now().to_rfc3339();

    sqlx::query(
        r#"
        INSERT INTO system_state (key, value, updated_at)
        VALUES ('production_mode', ?, ?)
        ON CONFLICT(key) DO UPDATE SET
            value = excluded.value,
            updated_at = excluded.updated_at
        "#,
    )
    .bind(value)
    .bind(&updated_at)
    .execute(pool)
    .await
    .context("Failed to set production mode")?;

    Ok(())
}

// =============================================================================
// Refactor Sessions
// =============================================================================

/// Insert a new refactor session
pub async fn insert_refactor_session(pool: &SqlitePool, session: &RefactorSession) -> Result<()> {
    let id = session.id.0.to_string();
    let started_at = session.started_at.to_rfc3339();
    let ended_at = session.ended_at.map(|t| t.to_rfc3339());
    let production_halted = session.production_halted as i32;
    let changes_applied = session.changes_applied as i32;

    sqlx::query(
        r#"
        INSERT INTO refactor_sessions (id, started_at, ended_at, production_halted, changes_applied)
        VALUES (?, ?, ?, ?, ?)
        "#,
    )
    .bind(&id)
    .bind(&started_at)
    .bind(&ended_at)
    .bind(production_halted)
    .bind(changes_applied)
    .execute(pool)
    .await
    .context("Failed to insert refactor session")?;

    Ok(())
}

/// Get a refactor session by ID
pub async fn get_refactor_session(
    pool: &SqlitePool,
    id: &RefactorId,
) -> Result<Option<RefactorSession>> {
    let id_str = id.0.to_string();

    let row: Option<RefactorSessionRow> = sqlx::query_as(
        "SELECT id, started_at, ended_at, production_halted, changes_applied FROM refactor_sessions WHERE id = ?"
    )
    .bind(&id_str)
    .fetch_optional(pool)
    .await
    .context("Failed to fetch refactor session")?;

    match row {
        Some(row) => {
            let mut session = row.into_session()?;
            // Load associated changes
            session.proposed_changes = list_changes_for_session(pool, id).await?;
            Ok(Some(session))
        }
        None => Ok(None),
    }
}

/// Get the currently active refactor session (if any)
pub async fn get_active_refactor_session(pool: &SqlitePool) -> Result<Option<RefactorSession>> {
    let row: Option<RefactorSessionRow> = sqlx::query_as(
        "SELECT id, started_at, ended_at, production_halted, changes_applied FROM refactor_sessions WHERE ended_at IS NULL ORDER BY started_at DESC LIMIT 1"
    )
    .fetch_optional(pool)
    .await
    .context("Failed to fetch active refactor session")?;

    match row {
        Some(row) => {
            let id = RefactorId(uuid::Uuid::from_str(&row.id)?);
            let mut session = row.into_session()?;
            session.proposed_changes = list_changes_for_session(pool, &id).await?;
            Ok(Some(session))
        }
        None => Ok(None),
    }
}

/// Update a refactor session
pub async fn update_refactor_session(pool: &SqlitePool, session: &RefactorSession) -> Result<()> {
    let id = session.id.0.to_string();
    let ended_at = session.ended_at.map(|t| t.to_rfc3339());
    let production_halted = session.production_halted as i32;
    let changes_applied = session.changes_applied as i32;

    sqlx::query(
        r#"
        UPDATE refactor_sessions
        SET ended_at = ?, production_halted = ?, changes_applied = ?
        WHERE id = ?
        "#,
    )
    .bind(&ended_at)
    .bind(production_halted)
    .bind(changes_applied)
    .bind(&id)
    .execute(pool)
    .await
    .context("Failed to update refactor session")?;

    Ok(())
}

/// List all refactor sessions (most recent first)
pub async fn list_refactor_sessions(pool: &SqlitePool) -> Result<Vec<RefactorSession>> {
    let rows: Vec<RefactorSessionRow> = sqlx::query_as(
        "SELECT id, started_at, ended_at, production_halted, changes_applied FROM refactor_sessions ORDER BY started_at DESC"
    )
    .fetch_all(pool)
    .await
    .context("Failed to list refactor sessions")?;

    let mut sessions = Vec::new();
    for row in rows {
        let id = RefactorId(uuid::Uuid::from_str(&row.id)?);
        let mut session = row.into_session()?;
        session.proposed_changes = list_changes_for_session(pool, &id).await?;
        sessions.push(session);
    }
    Ok(sessions)
}

// =============================================================================
// Refactor Changes
// =============================================================================

/// Insert a refactor change
pub async fn insert_refactor_change(pool: &SqlitePool, change: &RefactorChange) -> Result<()> {
    let id = change.id.0.to_string();
    let session_id = change.session_id.0.to_string();
    let change_type = match change.change_type {
        ChangeType::Create => "create",
        ChangeType::Modify => "modify",
        ChangeType::Delete => "delete",
        ChangeType::Rename => "rename",
    };
    let status = change.status.as_str();
    let created_at = change.created_at.to_rfc3339();

    sqlx::query(
        r#"
        INSERT INTO refactor_changes (id, session_id, file_path, change_type, before_content, after_content, reason, status, created_at)
        VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)
        "#,
    )
    .bind(&id)
    .bind(&session_id)
    .bind(&change.file_path)
    .bind(change_type)
    .bind(&change.before_content)
    .bind(&change.after_content)
    .bind(&change.reason)
    .bind(status)
    .bind(&created_at)
    .execute(pool)
    .await
    .context("Failed to insert refactor change")?;

    Ok(())
}

/// Get a refactor change by ID
pub async fn get_refactor_change(
    pool: &SqlitePool,
    id: &ChangeId,
) -> Result<Option<RefactorChange>> {
    let id_str = id.0.to_string();

    let row: Option<RefactorChangeRow> = sqlx::query_as(
        "SELECT id, session_id, file_path, change_type, before_content, after_content, reason, status, created_at FROM refactor_changes WHERE id = ?"
    )
    .bind(&id_str)
    .fetch_optional(pool)
    .await
    .context("Failed to fetch refactor change")?;

    match row {
        Some(row) => Ok(Some(row.into_change()?)),
        None => Ok(None),
    }
}

/// List changes for a session
pub async fn list_changes_for_session(
    pool: &SqlitePool,
    session_id: &RefactorId,
) -> Result<Vec<RefactorChange>> {
    let session_id_str = session_id.0.to_string();

    let rows: Vec<RefactorChangeRow> = sqlx::query_as(
        "SELECT id, session_id, file_path, change_type, before_content, after_content, reason, status, created_at FROM refactor_changes WHERE session_id = ? ORDER BY created_at ASC"
    )
    .bind(&session_id_str)
    .fetch_all(pool)
    .await
    .context("Failed to list changes for session")?;

    rows.into_iter().map(|r| r.into_change()).collect()
}

/// Update change status
pub async fn update_change_status(
    pool: &SqlitePool,
    id: &ChangeId,
    status: ChangeStatus,
) -> Result<()> {
    let id_str = id.0.to_string();
    let status_str = status.as_str();

    sqlx::query("UPDATE refactor_changes SET status = ? WHERE id = ?")
        .bind(status_str)
        .bind(&id_str)
        .execute(pool)
        .await
        .context("Failed to update change status")?;

    Ok(())
}

/// List changes by status
pub async fn list_changes_by_status(
    pool: &SqlitePool,
    status: ChangeStatus,
) -> Result<Vec<RefactorChange>> {
    let status_str = status.as_str();

    let rows: Vec<RefactorChangeRow> = sqlx::query_as(
        "SELECT id, session_id, file_path, change_type, before_content, after_content, reason, status, created_at FROM refactor_changes WHERE status = ? ORDER BY created_at ASC"
    )
    .bind(status_str)
    .fetch_all(pool)
    .await
    .context("Failed to list changes by status")?;

    rows.into_iter().map(|r| r.into_change()).collect()
}

// =============================================================================
// Internal Row Types
// =============================================================================

#[derive(sqlx::FromRow)]
struct RefactorSessionRow {
    id: String,
    started_at: String,
    ended_at: Option<String>,
    production_halted: i32,
    changes_applied: i32,
}

impl RefactorSessionRow {
    fn into_session(self) -> Result<RefactorSession> {
        let id = RefactorId(uuid::Uuid::from_str(&self.id)?);
        let started_at =
            chrono::DateTime::parse_from_rfc3339(&self.started_at)?.with_timezone(&chrono::Utc);
        let ended_at = self
            .ended_at
            .map(|s| {
                chrono::DateTime::parse_from_rfc3339(&s).map(|d| d.with_timezone(&chrono::Utc))
            })
            .transpose()?;

        Ok(RefactorSession {
            id,
            started_at,
            ended_at,
            production_halted: self.production_halted != 0,
            changes_applied: self.changes_applied != 0,
            proposed_changes: Vec::new(), // Loaded separately
        })
    }
}

#[derive(sqlx::FromRow)]
struct RefactorChangeRow {
    id: String,
    session_id: String,
    file_path: String,
    change_type: String,
    before_content: Option<String>,
    after_content: Option<String>,
    reason: String,
    status: String,
    created_at: String,
}

impl RefactorChangeRow {
    fn into_change(self) -> Result<RefactorChange> {
        let id = ChangeId(uuid::Uuid::from_str(&self.id)?);
        let session_id = RefactorId(uuid::Uuid::from_str(&self.session_id)?);
        let change_type = match self.change_type.as_str() {
            "create" => ChangeType::Create,
            "modify" => ChangeType::Modify,
            "delete" => ChangeType::Delete,
            "rename" => ChangeType::Rename,
            _ => ChangeType::Modify,
        };
        let status = ChangeStatus::from_str(&self.status);
        let created_at =
            chrono::DateTime::parse_from_rfc3339(&self.created_at)?.with_timezone(&chrono::Utc);

        Ok(RefactorChange {
            id,
            session_id,
            file_path: self.file_path,
            change_type,
            before_content: self.before_content,
            after_content: self.after_content,
            reason: self.reason,
            status,
            created_at,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    async fn setup_test_db() -> (SqlitePool, TempDir) {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("test.db");
        let pool = crate::db::init_db_at(db_path.to_str().unwrap())
            .await
            .unwrap();
        (pool, temp_dir)
    }

    #[tokio::test]
    async fn production_mode_default_is_running() {
        let (pool, _temp_dir) = setup_test_db().await;

        let mode = get_production_mode(&pool).await.unwrap();
        assert_eq!(mode, ProductionMode::Running);

        pool.close().await;
    }

    #[tokio::test]
    async fn can_set_and_get_production_mode() {
        let (pool, _temp_dir) = setup_test_db().await;

        set_production_mode(&pool, ProductionMode::RefactorMode)
            .await
            .unwrap();
        let mode = get_production_mode(&pool).await.unwrap();
        assert_eq!(mode, ProductionMode::RefactorMode);

        set_production_mode(&pool, ProductionMode::Paused)
            .await
            .unwrap();
        let mode = get_production_mode(&pool).await.unwrap();
        assert_eq!(mode, ProductionMode::Paused);

        pool.close().await;
    }

    #[tokio::test]
    async fn can_insert_and_get_refactor_session() {
        let (pool, _temp_dir) = setup_test_db().await;

        let session = RefactorSession::new();
        insert_refactor_session(&pool, &session).await.unwrap();

        let retrieved = get_refactor_session(&pool, &session.id).await.unwrap();
        assert!(retrieved.is_some());
        let retrieved = retrieved.unwrap();
        assert_eq!(retrieved.id, session.id);
        assert!(retrieved.is_active());

        pool.close().await;
    }

    #[tokio::test]
    async fn can_get_active_refactor_session() {
        let (pool, _temp_dir) = setup_test_db().await;

        // No active session initially
        let active = get_active_refactor_session(&pool).await.unwrap();
        assert!(active.is_none());

        // Create a session
        let session = RefactorSession::new();
        insert_refactor_session(&pool, &session).await.unwrap();

        // Now there's an active session
        let active = get_active_refactor_session(&pool).await.unwrap();
        assert!(active.is_some());
        assert_eq!(active.unwrap().id, session.id);

        pool.close().await;
    }

    #[tokio::test]
    async fn can_update_refactor_session() {
        let (pool, _temp_dir) = setup_test_db().await;

        let mut session = RefactorSession::new();
        insert_refactor_session(&pool, &session).await.unwrap();

        session.halt_production();
        session.end();
        update_refactor_session(&pool, &session).await.unwrap();

        let retrieved = get_refactor_session(&pool, &session.id)
            .await
            .unwrap()
            .unwrap();
        assert!(retrieved.production_halted);
        assert!(!retrieved.is_active());

        pool.close().await;
    }

    #[tokio::test]
    async fn can_insert_and_get_refactor_change() {
        let (pool, _temp_dir) = setup_test_db().await;

        let session = RefactorSession::new();
        insert_refactor_session(&pool, &session).await.unwrap();

        let change = RefactorChange::create(
            session.id.clone(),
            "decomp/M2/2.7.md".to_string(),
            "# Ticket 2.7\n\nNew ticket content".to_string(),
            "Adding new ticket for feature X".to_string(),
        );
        insert_refactor_change(&pool, &change).await.unwrap();

        let retrieved = get_refactor_change(&pool, &change.id).await.unwrap();
        assert!(retrieved.is_some());
        let retrieved = retrieved.unwrap();
        assert_eq!(retrieved.file_path, "decomp/M2/2.7.md");
        assert_eq!(retrieved.change_type, ChangeType::Create);
        assert_eq!(retrieved.status, ChangeStatus::Proposed);

        pool.close().await;
    }

    #[tokio::test]
    async fn can_list_changes_for_session() {
        let (pool, _temp_dir) = setup_test_db().await;

        let session = RefactorSession::new();
        insert_refactor_session(&pool, &session).await.unwrap();

        let change1 = RefactorChange::create(
            session.id.clone(),
            "file1.md".to_string(),
            "content1".to_string(),
            "reason1".to_string(),
        );
        let change2 = RefactorChange::modify(
            session.id.clone(),
            "file2.md".to_string(),
            "old".to_string(),
            "new".to_string(),
            "reason2".to_string(),
        );

        insert_refactor_change(&pool, &change1).await.unwrap();
        insert_refactor_change(&pool, &change2).await.unwrap();

        let changes = list_changes_for_session(&pool, &session.id).await.unwrap();
        assert_eq!(changes.len(), 2);

        pool.close().await;
    }

    #[tokio::test]
    async fn can_update_change_status() {
        let (pool, _temp_dir) = setup_test_db().await;

        let session = RefactorSession::new();
        insert_refactor_session(&pool, &session).await.unwrap();

        let change = RefactorChange::create(
            session.id.clone(),
            "test.md".to_string(),
            "content".to_string(),
            "reason".to_string(),
        );
        insert_refactor_change(&pool, &change).await.unwrap();

        update_change_status(&pool, &change.id, ChangeStatus::Approved)
            .await
            .unwrap();

        let retrieved = get_refactor_change(&pool, &change.id)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(retrieved.status, ChangeStatus::Approved);

        pool.close().await;
    }

    #[tokio::test]
    async fn can_list_changes_by_status() {
        let (pool, _temp_dir) = setup_test_db().await;

        let session = RefactorSession::new();
        insert_refactor_session(&pool, &session).await.unwrap();

        let change1 = RefactorChange::create(
            session.id.clone(),
            "file1.md".to_string(),
            "content1".to_string(),
            "reason1".to_string(),
        );
        let change2 = RefactorChange::create(
            session.id.clone(),
            "file2.md".to_string(),
            "content2".to_string(),
            "reason2".to_string(),
        );

        insert_refactor_change(&pool, &change1).await.unwrap();
        insert_refactor_change(&pool, &change2).await.unwrap();

        // Both are proposed
        let proposed = list_changes_by_status(&pool, ChangeStatus::Proposed)
            .await
            .unwrap();
        assert_eq!(proposed.len(), 2);

        // Approve one
        update_change_status(&pool, &change1.id, ChangeStatus::Approved)
            .await
            .unwrap();

        let proposed = list_changes_by_status(&pool, ChangeStatus::Proposed)
            .await
            .unwrap();
        assert_eq!(proposed.len(), 1);

        let approved = list_changes_by_status(&pool, ChangeStatus::Approved)
            .await
            .unwrap();
        assert_eq!(approved.len(), 1);

        pool.close().await;
    }

    #[tokio::test]
    async fn session_includes_changes_when_fetched() {
        let (pool, _temp_dir) = setup_test_db().await;

        let session = RefactorSession::new();
        insert_refactor_session(&pool, &session).await.unwrap();

        let change = RefactorChange::create(
            session.id.clone(),
            "test.md".to_string(),
            "content".to_string(),
            "reason".to_string(),
        );
        insert_refactor_change(&pool, &change).await.unwrap();

        // Fetch session and verify changes are included
        let retrieved = get_refactor_session(&pool, &session.id)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(retrieved.proposed_changes.len(), 1);
        assert_eq!(retrieved.proposed_changes[0].file_path, "test.md");

        pool.close().await;
    }
}
