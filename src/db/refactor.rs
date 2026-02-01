//! Database queries for refactor mode.
//!
//! Handles persistence of:
//! - System production mode state
//! - Refactor sessions
//! - Proposed changes

use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use sqlx::PgPool;
use uuid::Uuid;

use crate::types::{ChangeId, ChangeStatus, ChangeType, ProductionMode, RefactorChange, RefactorId, RefactorSession};

// =============================================================================
// System State
// =============================================================================

/// Get the current production mode
pub async fn get_production_mode(pool: &PgPool) -> Result<ProductionMode> {
    let row: Option<(String,)> = sqlx::query_as("SELECT value FROM system_state WHERE key = 'production_mode'")
        .fetch_optional(pool)
        .await
        .context("Failed to fetch production mode")?;

    Ok(row.map(|(v,)| ProductionMode::from_str(&v)).unwrap_or_default())
}

/// Set the production mode
pub async fn set_production_mode(pool: &PgPool, mode: ProductionMode) -> Result<()> {
    let value = mode.as_str();

    sqlx::query(
        r#"
        INSERT INTO system_state (key, value, updated_at)
        VALUES ('production_mode', $1, $2)
        ON CONFLICT(key) DO UPDATE SET
            value = excluded.value,
            updated_at = excluded.updated_at
        "#,
    )
    .bind(value)
    .bind(Utc::now())
    .execute(pool)
    .await
    .context("Failed to set production mode")?;

    Ok(())
}

// =============================================================================
// Milestone Limit
// =============================================================================

/// Get the current milestone limit (None = no limit)
pub async fn get_milestone_limit(pool: &PgPool) -> Result<Option<u8>> {
    let row: Option<(String,)> = sqlx::query_as("SELECT value FROM system_state WHERE key = 'milestone_limit'")
        .fetch_optional(pool)
        .await
        .context("Failed to fetch milestone limit")?;

    Ok(row.and_then(|(v,)| v.parse::<u8>().ok()))
}

/// Set the milestone limit (None clears it)
pub async fn set_milestone_limit(pool: &PgPool, milestone: Option<u8>) -> Result<()> {
    match milestone {
        Some(m) => {
            // Validate milestone is in range 1-9
            if !(1..=9).contains(&m) {
                anyhow::bail!("Milestone must be between 1 and 9");
            }
            let value = m.to_string();
            sqlx::query(
                r#"
                INSERT INTO system_state (key, value, updated_at)
                VALUES ('milestone_limit', $1, $2)
                ON CONFLICT(key) DO UPDATE SET
                    value = excluded.value,
                    updated_at = excluded.updated_at
                "#,
            )
            .bind(&value)
            .bind(Utc::now())
            .execute(pool)
            .await
            .context("Failed to set milestone limit")?;
        }
        None => {
            // Delete the entry to clear the limit
            sqlx::query("DELETE FROM system_state WHERE key = 'milestone_limit'")
                .execute(pool)
                .await
                .context("Failed to clear milestone limit")?;
        }
    }

    Ok(())
}

// =============================================================================
// Refactor Sessions
// =============================================================================

/// Insert a new refactor session
pub async fn insert_refactor_session(pool: &PgPool, session: &RefactorSession) -> Result<()> {
    sqlx::query(
        r#"
        INSERT INTO refactor_sessions (id, started_at, ended_at, production_halted, changes_applied)
        VALUES ($1, $2, $3, $4, $5)
        "#,
    )
    .bind(session.id.0)
    .bind(session.started_at)
    .bind(session.ended_at)
    .bind(session.production_halted)
    .bind(session.changes_applied as i32)
    .execute(pool)
    .await
    .context("Failed to insert refactor session")?;

    Ok(())
}

/// Get a refactor session by ID
pub async fn get_refactor_session(pool: &PgPool, id: &RefactorId) -> Result<Option<RefactorSession>> {
    let row: Option<RefactorSessionRow> = sqlx::query_as("SELECT id, started_at, ended_at, production_halted, changes_applied FROM refactor_sessions WHERE id = $1")
        .bind(id.0)
        .fetch_optional(pool)
        .await
        .context("Failed to fetch refactor session")?;

    match row {
        Some(row) => {
            let session_id = RefactorId(row.id);
            let mut session = row.into_session();
            session.proposed_changes = list_changes_for_session(pool, &session_id).await?;
            Ok(Some(session))
        }
        None => Ok(None),
    }
}

/// Get the currently active refactor session (if any)
pub async fn get_active_refactor_session(pool: &PgPool) -> Result<Option<RefactorSession>> {
    let row: Option<RefactorSessionRow> =
        sqlx::query_as("SELECT id, started_at, ended_at, production_halted, changes_applied FROM refactor_sessions WHERE ended_at IS NULL ORDER BY started_at DESC LIMIT 1")
            .fetch_optional(pool)
            .await
            .context("Failed to fetch active refactor session")?;

    match row {
        Some(row) => {
            let id = RefactorId(row.id);
            let mut session = row.into_session();
            session.proposed_changes = list_changes_for_session(pool, &id).await?;
            Ok(Some(session))
        }
        None => Ok(None),
    }
}

/// Update a refactor session
pub async fn update_refactor_session(pool: &PgPool, session: &RefactorSession) -> Result<()> {
    sqlx::query(
        r#"
        UPDATE refactor_sessions
        SET ended_at = $1, production_halted = $2, changes_applied = $3
        WHERE id = $4
        "#,
    )
    .bind(session.ended_at)
    .bind(session.production_halted)
    .bind(session.changes_applied as i32)
    .bind(session.id.0)
    .execute(pool)
    .await
    .context("Failed to update refactor session")?;

    Ok(())
}

/// List all refactor sessions (most recent first)
pub async fn list_refactor_sessions(pool: &PgPool) -> Result<Vec<RefactorSession>> {
    let rows: Vec<RefactorSessionRow> = sqlx::query_as("SELECT id, started_at, ended_at, production_halted, changes_applied FROM refactor_sessions ORDER BY started_at DESC")
        .fetch_all(pool)
        .await
        .context("Failed to list refactor sessions")?;

    let mut sessions = Vec::new();
    for row in rows {
        let id = RefactorId(row.id);
        let mut session = row.into_session();
        session.proposed_changes = list_changes_for_session(pool, &id).await?;
        sessions.push(session);
    }
    Ok(sessions)
}

// =============================================================================
// Refactor Changes
// =============================================================================

/// Insert a refactor change
pub async fn insert_refactor_change(pool: &PgPool, change: &RefactorChange) -> Result<()> {
    let change_type = match change.change_type {
        ChangeType::Create => "create",
        ChangeType::Modify => "modify",
        ChangeType::Delete => "delete",
        ChangeType::Rename => "rename",
    };
    let status = change.status.as_str();

    sqlx::query(
        r#"
        INSERT INTO refactor_changes (id, session_id, file_path, change_type, before_content, after_content, reason, status, created_at)
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
        "#,
    )
    .bind(change.id.0)
    .bind(change.session_id.0)
    .bind(&change.file_path)
    .bind(change_type)
    .bind(&change.before_content)
    .bind(&change.after_content)
    .bind(&change.reason)
    .bind(status)
    .bind(change.created_at)
    .execute(pool)
    .await
    .context("Failed to insert refactor change")?;

    Ok(())
}

/// Get a refactor change by ID
pub async fn get_refactor_change(pool: &PgPool, id: &ChangeId) -> Result<Option<RefactorChange>> {
    let row: Option<RefactorChangeRow> =
        sqlx::query_as("SELECT id, session_id, file_path, change_type, before_content, after_content, reason, status, created_at FROM refactor_changes WHERE id = $1")
            .bind(id.0)
            .fetch_optional(pool)
            .await
            .context("Failed to fetch refactor change")?;

    match row {
        Some(row) => Ok(Some(row.into_change())),
        None => Ok(None),
    }
}

/// List changes for a session
pub async fn list_changes_for_session(pool: &PgPool, session_id: &RefactorId) -> Result<Vec<RefactorChange>> {
    let rows: Vec<RefactorChangeRow> = sqlx::query_as(
        "SELECT id, session_id, file_path, change_type, before_content, after_content, reason, status, created_at FROM refactor_changes WHERE session_id = $1 ORDER BY created_at ASC"
    )
    .bind(session_id.0)
    .fetch_all(pool)
    .await
    .context("Failed to list changes for session")?;

    Ok(rows.into_iter().map(|r| r.into_change()).collect())
}

/// Update change status
pub async fn update_change_status(pool: &PgPool, id: &ChangeId, status: ChangeStatus) -> Result<()> {
    let status_str = status.as_str();

    sqlx::query("UPDATE refactor_changes SET status = $1 WHERE id = $2")
        .bind(status_str)
        .bind(id.0)
        .execute(pool)
        .await
        .context("Failed to update change status")?;

    Ok(())
}

/// List changes by status
pub async fn list_changes_by_status(pool: &PgPool, status: ChangeStatus) -> Result<Vec<RefactorChange>> {
    let status_str = status.as_str();

    let rows: Vec<RefactorChangeRow> = sqlx::query_as(
        "SELECT id, session_id, file_path, change_type, before_content, after_content, reason, status, created_at FROM refactor_changes WHERE status = $1 ORDER BY created_at ASC",
    )
    .bind(status_str)
    .fetch_all(pool)
    .await
    .context("Failed to list changes by status")?;

    Ok(rows.into_iter().map(|r| r.into_change()).collect())
}

// =============================================================================
// Internal Row Types
// =============================================================================

#[derive(sqlx::FromRow)]
struct RefactorSessionRow {
    id: Uuid,
    started_at: DateTime<Utc>,
    ended_at: Option<DateTime<Utc>>,
    production_halted: bool,
    changes_applied: i32,
}

impl RefactorSessionRow {
    fn into_session(self) -> RefactorSession {
        RefactorSession {
            id: RefactorId(self.id),
            started_at: self.started_at,
            ended_at: self.ended_at,
            production_halted: self.production_halted,
            changes_applied: self.changes_applied != 0,
            proposed_changes: Vec::new(), // Loaded separately
        }
    }
}

#[derive(sqlx::FromRow)]
struct RefactorChangeRow {
    id: Uuid,
    session_id: Uuid,
    file_path: String,
    change_type: String,
    before_content: Option<String>,
    after_content: Option<String>,
    reason: String,
    status: String,
    created_at: DateTime<Utc>,
}

impl RefactorChangeRow {
    fn into_change(self) -> RefactorChange {
        let change_type = match self.change_type.as_str() {
            "create" => ChangeType::Create,
            "modify" => ChangeType::Modify,
            "delete" => ChangeType::Delete,
            "rename" => ChangeType::Rename,
            _ => ChangeType::Modify,
        };
        let status = ChangeStatus::from_str(&self.status);

        RefactorChange {
            id: ChangeId(self.id),
            session_id: RefactorId(self.session_id),
            file_path: self.file_path,
            change_type,
            before_content: self.before_content,
            after_content: self.after_content,
            reason: self.reason,
            status,
            created_at: self.created_at,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::db::test_utils::TestDb;

    #[tokio::test]
    #[ignore = "requires running Postgres"]
    async fn production_mode_default_is_running() {
        let db = TestDb::new().await;

        let mode = get_production_mode(&db.pool).await.unwrap();
        assert_eq!(mode, ProductionMode::Running);

        db.cleanup().await;
    }

    #[tokio::test]
    #[ignore = "requires running Postgres"]
    async fn can_set_and_get_production_mode() {
        let db = TestDb::new().await;

        set_production_mode(&db.pool, ProductionMode::RefactorMode).await.unwrap();
        let mode = get_production_mode(&db.pool).await.unwrap();
        assert_eq!(mode, ProductionMode::RefactorMode);

        set_production_mode(&db.pool, ProductionMode::Paused).await.unwrap();
        let mode = get_production_mode(&db.pool).await.unwrap();
        assert_eq!(mode, ProductionMode::Paused);

        db.cleanup().await;
    }

    #[tokio::test]
    #[ignore = "requires running Postgres"]
    async fn can_insert_and_get_refactor_session() {
        let db = TestDb::new().await;

        let session = RefactorSession::new();
        insert_refactor_session(&db.pool, &session).await.unwrap();

        let retrieved = get_refactor_session(&db.pool, &session.id).await.unwrap();
        assert!(retrieved.is_some());
        let retrieved = retrieved.unwrap();
        assert_eq!(retrieved.id, session.id);
        assert!(retrieved.is_active());

        db.cleanup().await;
    }

    #[tokio::test]
    #[ignore = "requires running Postgres"]
    async fn can_get_active_refactor_session() {
        let db = TestDb::new().await;

        // No active session initially
        let active = get_active_refactor_session(&db.pool).await.unwrap();
        assert!(active.is_none());

        // Create a session
        let session = RefactorSession::new();
        insert_refactor_session(&db.pool, &session).await.unwrap();

        // Now there's an active session
        let active = get_active_refactor_session(&db.pool).await.unwrap();
        assert!(active.is_some());
        assert_eq!(active.unwrap().id, session.id);

        db.cleanup().await;
    }

    #[tokio::test]
    #[ignore = "requires running Postgres"]
    async fn can_update_refactor_session() {
        let db = TestDb::new().await;

        let mut session = RefactorSession::new();
        insert_refactor_session(&db.pool, &session).await.unwrap();

        session.halt_production();
        session.end();
        update_refactor_session(&db.pool, &session).await.unwrap();

        let retrieved = get_refactor_session(&db.pool, &session.id).await.unwrap().unwrap();
        assert!(retrieved.production_halted);
        assert!(!retrieved.is_active());

        db.cleanup().await;
    }

    #[tokio::test]
    #[ignore = "requires running Postgres"]
    async fn can_insert_and_get_refactor_change() {
        let db = TestDb::new().await;

        let session = RefactorSession::new();
        insert_refactor_session(&db.pool, &session).await.unwrap();

        let change = RefactorChange::create(
            session.id.clone(),
            "decomp/M2/2.7.md".to_string(),
            "# Ticket 2.7\n\nNew ticket content".to_string(),
            "Adding new ticket for feature X".to_string(),
        );
        insert_refactor_change(&db.pool, &change).await.unwrap();

        let retrieved = get_refactor_change(&db.pool, &change.id).await.unwrap();
        assert!(retrieved.is_some());
        let retrieved = retrieved.unwrap();
        assert_eq!(retrieved.file_path, "decomp/M2/2.7.md");
        assert_eq!(retrieved.change_type, ChangeType::Create);
        assert_eq!(retrieved.status, ChangeStatus::Proposed);

        db.cleanup().await;
    }

    #[tokio::test]
    #[ignore = "requires running Postgres"]
    async fn can_list_changes_for_session() {
        let db = TestDb::new().await;

        let session = RefactorSession::new();
        insert_refactor_session(&db.pool, &session).await.unwrap();

        let change1 = RefactorChange::create(session.id.clone(), "file1.md".to_string(), "content1".to_string(), "reason1".to_string());
        let change2 = RefactorChange::modify(session.id.clone(), "file2.md".to_string(), "old".to_string(), "new".to_string(), "reason2".to_string());

        insert_refactor_change(&db.pool, &change1).await.unwrap();
        insert_refactor_change(&db.pool, &change2).await.unwrap();

        let changes = list_changes_for_session(&db.pool, &session.id).await.unwrap();
        assert_eq!(changes.len(), 2);

        db.cleanup().await;
    }

    #[tokio::test]
    #[ignore = "requires running Postgres"]
    async fn can_update_change_status() {
        let db = TestDb::new().await;

        let session = RefactorSession::new();
        insert_refactor_session(&db.pool, &session).await.unwrap();

        let change = RefactorChange::create(session.id.clone(), "test.md".to_string(), "content".to_string(), "reason".to_string());
        insert_refactor_change(&db.pool, &change).await.unwrap();

        update_change_status(&db.pool, &change.id, ChangeStatus::Approved).await.unwrap();

        let retrieved = get_refactor_change(&db.pool, &change.id).await.unwrap().unwrap();
        assert_eq!(retrieved.status, ChangeStatus::Approved);

        db.cleanup().await;
    }

    #[tokio::test]
    #[ignore = "requires running Postgres"]
    async fn can_list_changes_by_status() {
        let db = TestDb::new().await;

        let session = RefactorSession::new();
        insert_refactor_session(&db.pool, &session).await.unwrap();

        let change1 = RefactorChange::create(session.id.clone(), "file1.md".to_string(), "content1".to_string(), "reason1".to_string());
        let change2 = RefactorChange::create(session.id.clone(), "file2.md".to_string(), "content2".to_string(), "reason2".to_string());

        insert_refactor_change(&db.pool, &change1).await.unwrap();
        insert_refactor_change(&db.pool, &change2).await.unwrap();

        // Both are proposed
        let proposed = list_changes_by_status(&db.pool, ChangeStatus::Proposed).await.unwrap();
        assert_eq!(proposed.len(), 2);

        // Approve one
        update_change_status(&db.pool, &change1.id, ChangeStatus::Approved).await.unwrap();

        let proposed = list_changes_by_status(&db.pool, ChangeStatus::Proposed).await.unwrap();
        assert_eq!(proposed.len(), 1);

        let approved = list_changes_by_status(&db.pool, ChangeStatus::Approved).await.unwrap();
        assert_eq!(approved.len(), 1);

        db.cleanup().await;
    }

    #[tokio::test]
    #[ignore = "requires running Postgres"]
    async fn session_includes_changes_when_fetched() {
        let db = TestDb::new().await;

        let session = RefactorSession::new();
        insert_refactor_session(&db.pool, &session).await.unwrap();

        let change = RefactorChange::create(session.id.clone(), "test.md".to_string(), "content".to_string(), "reason".to_string());
        insert_refactor_change(&db.pool, &change).await.unwrap();

        // Fetch session and verify changes are included
        let retrieved = get_refactor_session(&db.pool, &session.id).await.unwrap().unwrap();
        assert_eq!(retrieved.proposed_changes.len(), 1);
        assert_eq!(retrieved.proposed_changes[0].file_path, "test.md");

        db.cleanup().await;
    }

    #[tokio::test]
    #[ignore = "requires running Postgres"]
    async fn milestone_limit_default_is_none() {
        let db = TestDb::new().await;

        let limit = get_milestone_limit(&db.pool).await.unwrap();
        assert!(limit.is_none());

        db.cleanup().await;
    }

    #[tokio::test]
    #[ignore = "requires running Postgres"]
    async fn can_set_and_get_milestone_limit() {
        let db = TestDb::new().await;

        set_milestone_limit(&db.pool, Some(3)).await.unwrap();
        let limit = get_milestone_limit(&db.pool).await.unwrap();
        assert_eq!(limit, Some(3));

        set_milestone_limit(&db.pool, Some(7)).await.unwrap();
        let limit = get_milestone_limit(&db.pool).await.unwrap();
        assert_eq!(limit, Some(7));

        db.cleanup().await;
    }

    #[tokio::test]
    #[ignore = "requires running Postgres"]
    async fn can_clear_milestone_limit() {
        let db = TestDb::new().await;

        set_milestone_limit(&db.pool, Some(5)).await.unwrap();
        assert_eq!(get_milestone_limit(&db.pool).await.unwrap(), Some(5));

        set_milestone_limit(&db.pool, None).await.unwrap();
        assert!(get_milestone_limit(&db.pool).await.unwrap().is_none());

        db.cleanup().await;
    }

    #[tokio::test]
    #[ignore = "requires running Postgres"]
    async fn milestone_limit_validates_range() {
        let db = TestDb::new().await;

        // Valid range 1-9
        assert!(set_milestone_limit(&db.pool, Some(1)).await.is_ok());
        assert!(set_milestone_limit(&db.pool, Some(9)).await.is_ok());

        // Invalid: 0 and 10
        assert!(set_milestone_limit(&db.pool, Some(0)).await.is_err());
        assert!(set_milestone_limit(&db.pool, Some(10)).await.is_err());

        db.cleanup().await;
    }
}
