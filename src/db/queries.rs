//! Database query helpers

use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use sqlx::PgPool;
use uuid::Uuid;

use crate::types::{AgentTier, Priority, Task, TaskId, TaskStatus};

/// Insert a new task into the database
pub async fn insert_task(pool: &PgPool, task: &Task) -> Result<()> {
    let tier = format!("{:?}", task.assigned_tier).to_lowercase();
    let agent_id = task.assigned_agent.as_ref().map(|a| a.0);
    let status = format!("{:?}", task.status).to_lowercase();
    let priority = format!("{:?}", task.priority).to_lowercase();
    let context_files = serde_json::to_value(&task.context_files)?;
    let metadata = task
        .metadata
        .as_ref()
        .map(|m| serde_json::to_value(m))
        .transpose()?;

    sqlx::query(
        r#"
        INSERT INTO tasks (id, slice_id, title, description, assigned_tier, assigned_agent, status, priority, context_files, metadata, created_at, updated_at)
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12)
        "#,
    )
    .bind(task.id.0)
    .bind(task.slice_id.as_ref().map(|s| s.0))
    .bind(&task.title)
    .bind(&task.description)
    .bind(&tier)
    .bind(agent_id)
    .bind(&status)
    .bind(&priority)
    .bind(&context_files)
    .bind(&metadata)
    .bind(task.created_at)
    .bind(task.updated_at)
    .execute(pool)
    .await
    .context("Failed to insert task")?;

    Ok(())
}

/// Get a task by ID
pub async fn get_task(pool: &PgPool, id: &TaskId) -> Result<Option<Task>> {
    let row: Option<TaskRow> = sqlx::query_as(
        "SELECT id, slice_id, title, description, assigned_tier, assigned_agent, status, priority, context_files, metadata, created_at, updated_at FROM tasks WHERE id = $1"
    )
    .bind(id.0)
    .fetch_optional(pool)
    .await
    .context("Failed to fetch task")?;

    match row {
        Some(row) => Ok(Some(row.into_task())),
        None => Ok(None),
    }
}

/// Update task status
pub async fn update_task_status(pool: &PgPool, id: &TaskId, status: TaskStatus) -> Result<()> {
    let status_str = format!("{:?}", status).to_lowercase();

    sqlx::query("UPDATE tasks SET status = $1, updated_at = $2 WHERE id = $3")
        .bind(&status_str)
        .bind(Utc::now())
        .bind(id.0)
        .execute(pool)
        .await
        .context("Failed to update task status")?;

    Ok(())
}

/// List tasks by status
pub async fn list_tasks_by_status(pool: &PgPool, status: TaskStatus) -> Result<Vec<Task>> {
    let status_str = format!("{:?}", status).to_lowercase();

    let rows: Vec<TaskRow> = sqlx::query_as(
        "SELECT id, slice_id, title, description, assigned_tier, assigned_agent, status, priority, context_files, metadata, created_at, updated_at FROM tasks WHERE status = $1 ORDER BY created_at DESC"
    )
    .bind(&status_str)
    .fetch_all(pool)
    .await
    .context("Failed to list tasks")?;

    Ok(rows.into_iter().map(|r| r.into_task()).collect())
}

/// List all tasks with optional status filter and limit
pub async fn list_tasks(
    pool: &PgPool,
    status: Option<&str>,
    limit: Option<u32>,
) -> Result<Vec<Task>> {
    let limit = limit.unwrap_or(100).min(1000) as i64;

    let rows: Vec<TaskRow> = if let Some(status_filter) = status {
        sqlx::query_as(
            "SELECT id, slice_id, title, description, assigned_tier, assigned_agent, status, priority, context_files, metadata, created_at, updated_at FROM tasks WHERE status = $1 ORDER BY created_at DESC LIMIT $2"
        )
        .bind(status_filter)
        .bind(limit)
        .fetch_all(pool)
        .await
        .context("Failed to list tasks")?
    } else {
        sqlx::query_as(
            "SELECT id, slice_id, title, description, assigned_tier, assigned_agent, status, priority, context_files, metadata, created_at, updated_at FROM tasks ORDER BY created_at DESC LIMIT $1"
        )
        .bind(limit)
        .fetch_all(pool)
        .await
        .context("Failed to list tasks")?
    };

    Ok(rows.into_iter().map(|r| r.into_task()).collect())
}

/// Get a task by UUID (string version for API)
pub async fn get_task_by_uuid(pool: &PgPool, id: Uuid) -> Result<Option<Task>> {
    let task_id = TaskId(id);
    get_task(pool, &task_id).await
}

// Internal row type for sqlx
#[derive(sqlx::FromRow)]
struct TaskRow {
    id: Uuid,
    slice_id: Option<Uuid>,
    title: String,
    description: String,
    assigned_tier: String,
    assigned_agent: Option<Uuid>,
    status: String,
    priority: String,
    context_files: serde_json::Value,
    metadata: Option<serde_json::Value>,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

impl TaskRow {
    fn into_task(self) -> Task {
        let assigned_tier = match self.assigned_tier.as_str() {
            "orchestrator" => AgentTier::Orchestrator,
            "worker" => AgentTier::Worker,
            "utility" => AgentTier::Utility,
            _ => AgentTier::Worker,
        };

        let status = match self.status.as_str() {
            "pending" => TaskStatus::Pending,
            "inprogress" | "in_progress" => TaskStatus::InProgress,
            "review" => TaskStatus::Review,
            "completed" => TaskStatus::Completed,
            "failed" => TaskStatus::Failed,
            _ => TaskStatus::Pending,
        };

        let priority = match self.priority.as_str() {
            "low" => Priority::Low,
            "normal" => Priority::Normal,
            "high" => Priority::High,
            "urgent" => Priority::Urgent,
            _ => Priority::Normal,
        };

        let context_files: Vec<std::path::PathBuf> =
            serde_json::from_value(self.context_files).unwrap_or_default();
        let metadata: Option<std::collections::HashMap<String, String>> =
            self.metadata.and_then(|m| serde_json::from_value(m).ok());

        Task {
            id: TaskId(self.id),
            slice_id: self.slice_id.map(crate::types::SliceId),
            title: self.title,
            description: self.description,
            assigned_tier,
            assigned_agent: self.assigned_agent.map(crate::types::AgentId),
            status,
            priority,
            context_files,
            metadata,
            depends_on: vec![], // Dependencies loaded separately via DependencyTracker
            created_at: self.created_at,
            updated_at: self.updated_at,
        }
    }
}

// ============================================================================
// Chat Message Queries
// ============================================================================

/// A chat message between user and assistant
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct ChatMessageRow {
    pub id: Uuid,
    pub role: String,
    pub content: String,
    pub timestamp: DateTime<Utc>,
}

/// Insert a new chat message
pub async fn insert_chat_message(
    pool: &PgPool,
    id: &Uuid,
    role: &str,
    content: &str,
) -> Result<()> {
    sqlx::query("INSERT INTO chat_messages (id, role, content, timestamp) VALUES ($1, $2, $3, $4)")
        .bind(id)
        .bind(role)
        .bind(content)
        .bind(Utc::now())
        .execute(pool)
        .await
        .context("Failed to insert chat message")?;

    Ok(())
}

/// Get chat history with pagination
pub async fn get_chat_history(
    pool: &PgPool,
    limit: u32,
    offset: u32,
) -> Result<Vec<ChatMessageRow>> {
    let limit = limit.min(1000) as i64;
    let offset = offset as i64;

    let rows: Vec<ChatMessageRow> = sqlx::query_as(
        "SELECT id, role, content, timestamp FROM chat_messages ORDER BY timestamp ASC LIMIT $1 OFFSET $2",
    )
    .bind(limit)
    .bind(offset)
    .fetch_all(pool)
    .await
    .context("Failed to get chat history")?;

    Ok(rows)
}

/// Clear all chat history
pub async fn clear_chat_history(pool: &PgPool) -> Result<()> {
    sqlx::query("DELETE FROM chat_messages")
        .execute(pool)
        .await
        .context("Failed to clear chat history")?;

    Ok(())
}

// ============================================================================
// Auth queries
// ============================================================================

/// Check if a password has been configured
pub async fn has_password(pool: &PgPool) -> Result<bool> {
    let count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM auth_config WHERE id = 1")
        .fetch_one(pool)
        .await
        .context("Failed to check password existence")?;

    Ok(count.0 > 0)
}

/// Store the password hash (first-time setup)
pub async fn set_password(pool: &PgPool, password_hash: &str) -> Result<()> {
    sqlx::query("INSERT INTO auth_config (id, password_hash) VALUES (1, $1)")
        .bind(password_hash)
        .execute(pool)
        .await
        .context("Failed to set password")?;

    Ok(())
}

/// Get the stored password hash
pub async fn get_password(pool: &PgPool) -> Result<Option<String>> {
    let row: Option<(String,)> =
        sqlx::query_as("SELECT password_hash FROM auth_config WHERE id = 1")
            .fetch_optional(pool)
            .await
            .context("Failed to get password")?;

    Ok(row.map(|r| r.0))
}

#[cfg(test)]
mod tests {
    use super::*;

    async fn setup_test_db() -> PgPool {
        let database_url = std::env::var("DATABASE_URL")
            .unwrap_or_else(|_| "postgres://nexor:nexor@localhost:5432/nexor_test".to_string());
        let pool = sqlx::PgPool::connect(&database_url).await.unwrap();
        sqlx::migrate!().run(&pool).await.unwrap();
        // Clean tables for test isolation
        sqlx::query("DELETE FROM chat_messages")
            .execute(&pool)
            .await
            .unwrap();
        sqlx::query("DELETE FROM tasks")
            .execute(&pool)
            .await
            .unwrap();
        sqlx::query("DELETE FROM auth_config")
            .execute(&pool)
            .await
            .unwrap();
        pool
    }

    fn create_test_task() -> Task {
        Task {
            id: TaskId::new(),
            slice_id: None,
            title: "Test task".to_string(),
            description: "A test task".to_string(),
            assigned_tier: AgentTier::Worker,
            assigned_agent: None,
            status: TaskStatus::Pending,
            priority: Priority::Normal,
            context_files: vec![],
            metadata: None,
            depends_on: vec![],
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }

    #[tokio::test]
    async fn can_insert_and_get_task() {
        let pool = setup_test_db().await;
        let task = create_test_task();

        insert_task(&pool, &task).await.unwrap();

        let retrieved = get_task(&pool, &task.id).await.unwrap();
        assert!(retrieved.is_some());
        let retrieved = retrieved.unwrap();
        assert_eq!(retrieved.title, task.title);

        pool.close().await;
    }

    #[tokio::test]
    async fn can_update_task_status() {
        let pool = setup_test_db().await;
        let task = create_test_task();

        insert_task(&pool, &task).await.unwrap();
        update_task_status(&pool, &task.id, TaskStatus::InProgress)
            .await
            .unwrap();

        let retrieved = get_task(&pool, &task.id).await.unwrap().unwrap();
        assert_eq!(retrieved.status, TaskStatus::InProgress);

        pool.close().await;
    }

    #[tokio::test]
    async fn can_list_tasks_by_status() {
        let pool = setup_test_db().await;

        let task1 = create_test_task();
        let task2 = create_test_task();

        insert_task(&pool, &task1).await.unwrap();
        insert_task(&pool, &task2).await.unwrap();

        let pending = list_tasks_by_status(&pool, TaskStatus::Pending)
            .await
            .unwrap();
        assert!(pending.len() >= 2);

        pool.close().await;
    }

    // Chat message tests

    #[tokio::test]
    async fn can_insert_and_get_chat_message() {
        let pool = setup_test_db().await;
        let id = Uuid::new_v4();

        insert_chat_message(&pool, &id, "user", "Hello, world!")
            .await
            .unwrap();

        let history = get_chat_history(&pool, 50, 0).await.unwrap();
        assert!(history.len() >= 1);

        pool.close().await;
    }

    #[tokio::test]
    async fn chat_history_pagination_works() {
        let pool = setup_test_db().await;

        // Insert 5 messages
        for i in 0..5 {
            let id = Uuid::new_v4();
            insert_chat_message(&pool, &id, "user", &format!("Message {}", i))
                .await
                .unwrap();
        }

        // Get first 2
        let history = get_chat_history(&pool, 2, 0).await.unwrap();
        assert_eq!(history.len(), 2);

        pool.close().await;
    }

    #[tokio::test]
    async fn can_clear_chat_history() {
        let pool = setup_test_db().await;

        for _ in 0..3 {
            let id = Uuid::new_v4();
            insert_chat_message(&pool, &id, "user", "Test message")
                .await
                .unwrap();
        }

        let history = get_chat_history(&pool, 50, 0).await.unwrap();
        assert!(history.len() >= 3);

        clear_chat_history(&pool).await.unwrap();

        let history = get_chat_history(&pool, 50, 0).await.unwrap();
        assert_eq!(history.len(), 0);

        pool.close().await;
    }

    // Auth tests

    #[tokio::test]
    async fn test_password_flow() {
        let pool = setup_test_db().await;

        // Initially no password
        assert!(!has_password(&pool).await.unwrap());
        assert!(get_password(&pool).await.unwrap().is_none());

        // Set password
        set_password(&pool, "test_hash_123").await.unwrap();

        // Now has password
        assert!(has_password(&pool).await.unwrap());
        let stored = get_password(&pool).await.unwrap();
        assert_eq!(stored, Some("test_hash_123".to_string()));

        pool.close().await;
    }

    #[tokio::test]
    async fn test_set_password_twice_fails() {
        let pool = setup_test_db().await;

        // Set password first time
        set_password(&pool, "hash1").await.unwrap();

        // Setting again should fail (unique constraint on id=1)
        let result = set_password(&pool, "hash2").await;
        assert!(result.is_err());

        // Original password should still be there
        let stored = get_password(&pool).await.unwrap();
        assert_eq!(stored, Some("hash1".to_string()));

        pool.close().await;
    }
}
