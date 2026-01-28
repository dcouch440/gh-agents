//! Database query helpers

use anyhow::{Context, Result};
use chrono::Utc;
use sqlx::SqlitePool;

use crate::types::{AgentTier, Priority, Task, TaskId, TaskStatus};

/// Insert a new task into the database
pub async fn insert_task(pool: &SqlitePool, task: &Task) -> Result<()> {
    let id = task.id.0.to_string();
    let slice_id = task.slice_id.as_ref().map(|s| s.0.to_string());
    let tier = format!("{:?}", task.assigned_tier).to_lowercase();
    let agent_id = task.assigned_agent.as_ref().map(|a| a.0.to_string());
    let status = format!("{:?}", task.status).to_lowercase();
    let priority = format!("{:?}", task.priority).to_lowercase();
    let context_files = serde_json::to_string(&task.context_files)?;
    let metadata = task
        .metadata
        .as_ref()
        .map(|m| serde_json::to_string(m).unwrap_or_default());
    let created_at = task.created_at.to_rfc3339();
    let updated_at = task.updated_at.to_rfc3339();

    sqlx::query(
        r#"
        INSERT INTO tasks (id, slice_id, title, description, assigned_tier, assigned_agent, status, priority, context_files, metadata, created_at, updated_at)
        VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
        "#,
    )
    .bind(&id)
    .bind(&slice_id)
    .bind(&task.title)
    .bind(&task.description)
    .bind(&tier)
    .bind(&agent_id)
    .bind(&status)
    .bind(&priority)
    .bind(&context_files)
    .bind(&metadata)
    .bind(&created_at)
    .bind(&updated_at)
    .execute(pool)
    .await
    .context("Failed to insert task")?;

    Ok(())
}

/// Get a task by ID
pub async fn get_task(pool: &SqlitePool, id: &TaskId) -> Result<Option<Task>> {
    let id_str = id.0.to_string();

    let row: Option<TaskRow> = sqlx::query_as(
        "SELECT id, slice_id, title, description, assigned_tier, assigned_agent, status, priority, context_files, metadata, created_at, updated_at FROM tasks WHERE id = ?"
    )
    .bind(&id_str)
    .fetch_optional(pool)
    .await
    .context("Failed to fetch task")?;

    match row {
        Some(row) => Ok(Some(row.into_task()?)),
        None => Ok(None),
    }
}

/// Update task status
pub async fn update_task_status(pool: &SqlitePool, id: &TaskId, status: TaskStatus) -> Result<()> {
    let id_str = id.0.to_string();
    let status_str = format!("{:?}", status).to_lowercase();
    let updated_at = Utc::now().to_rfc3339();

    sqlx::query("UPDATE tasks SET status = ?, updated_at = ? WHERE id = ?")
        .bind(&status_str)
        .bind(&updated_at)
        .bind(&id_str)
        .execute(pool)
        .await
        .context("Failed to update task status")?;

    Ok(())
}

/// List tasks by status
pub async fn list_tasks_by_status(pool: &SqlitePool, status: TaskStatus) -> Result<Vec<Task>> {
    let status_str = format!("{:?}", status).to_lowercase();

    let rows: Vec<TaskRow> = sqlx::query_as(
        "SELECT id, slice_id, title, description, assigned_tier, assigned_agent, status, priority, context_files, metadata, created_at, updated_at FROM tasks WHERE status = ? ORDER BY created_at DESC"
    )
    .bind(&status_str)
    .fetch_all(pool)
    .await
    .context("Failed to list tasks")?;

    rows.into_iter().map(|r| r.into_task()).collect()
}

/// List all tasks with optional status filter and limit
pub async fn list_tasks(
    pool: &SqlitePool,
    status: Option<&str>,
    limit: Option<u32>,
) -> Result<Vec<Task>> {
    let limit = limit.unwrap_or(100).min(1000) as i64;

    let rows: Vec<TaskRow> = if let Some(status_filter) = status {
        sqlx::query_as(
            "SELECT id, slice_id, title, description, assigned_tier, assigned_agent, status, priority, context_files, metadata, created_at, updated_at FROM tasks WHERE status = ? ORDER BY created_at DESC LIMIT ?"
        )
        .bind(status_filter)
        .bind(limit)
        .fetch_all(pool)
        .await
        .context("Failed to list tasks")?
    } else {
        sqlx::query_as(
            "SELECT id, slice_id, title, description, assigned_tier, assigned_agent, status, priority, context_files, metadata, created_at, updated_at FROM tasks ORDER BY created_at DESC LIMIT ?"
        )
        .bind(limit)
        .fetch_all(pool)
        .await
        .context("Failed to list tasks")?
    };

    rows.into_iter().map(|r| r.into_task()).collect()
}

/// Get a task by UUID (string version for API)
pub async fn get_task_by_uuid(pool: &SqlitePool, id: uuid::Uuid) -> Result<Option<Task>> {
    let task_id = TaskId(id);
    get_task(pool, &task_id).await
}

// Internal row type for sqlx
#[derive(sqlx::FromRow)]
struct TaskRow {
    id: String,
    slice_id: Option<String>,
    title: String,
    description: String,
    assigned_tier: String,
    assigned_agent: Option<String>,
    status: String,
    priority: String,
    context_files: String,
    metadata: Option<String>,
    created_at: String,
    updated_at: String,
}

impl TaskRow {
    fn into_task(self) -> Result<Task> {
        use std::str::FromStr;

        let id = TaskId(uuid::Uuid::from_str(&self.id)?);
        let slice_id = self
            .slice_id
            .map(|s| crate::types::SliceId(uuid::Uuid::from_str(&s).unwrap()));
        let assigned_agent = self
            .assigned_agent
            .map(|a| crate::types::AgentId(uuid::Uuid::from_str(&a).unwrap()));

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

        let context_files: Vec<std::path::PathBuf> = serde_json::from_str(&self.context_files)?;
        let metadata: Option<std::collections::HashMap<String, String>> =
            self.metadata.and_then(|m| serde_json::from_str(&m).ok());
        let created_at =
            chrono::DateTime::parse_from_rfc3339(&self.created_at)?.with_timezone(&chrono::Utc);
        let updated_at =
            chrono::DateTime::parse_from_rfc3339(&self.updated_at)?.with_timezone(&chrono::Utc);

        Ok(Task {
            id,
            slice_id,
            title: self.title,
            description: self.description,
            assigned_tier,
            assigned_agent,
            status,
            priority,
            context_files,
            metadata,
            depends_on: vec![], // Dependencies loaded separately via DependencyTracker
            created_at,
            updated_at,
        })
    }
}

// ============================================================================
// Chat Message Queries
// ============================================================================

/// A chat message between user and assistant
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct ChatMessageRow {
    pub id: String,
    pub role: String,
    pub content: String,
    pub timestamp: String,
}

/// Insert a new chat message
pub async fn insert_chat_message(
    pool: &SqlitePool,
    id: &uuid::Uuid,
    role: &str,
    content: &str,
) -> Result<()> {
    let id_str = id.to_string();
    let timestamp = Utc::now().to_rfc3339();

    sqlx::query("INSERT INTO chat_messages (id, role, content, timestamp) VALUES (?, ?, ?, ?)")
        .bind(&id_str)
        .bind(role)
        .bind(content)
        .bind(&timestamp)
        .execute(pool)
        .await
        .context("Failed to insert chat message")?;

    Ok(())
}

/// Get chat history with pagination
pub async fn get_chat_history(
    pool: &SqlitePool,
    limit: u32,
    offset: u32,
) -> Result<Vec<ChatMessageRow>> {
    let limit = limit.min(1000) as i64;
    let offset = offset as i64;

    let rows: Vec<ChatMessageRow> = sqlx::query_as(
        "SELECT id, role, content, timestamp FROM chat_messages ORDER BY timestamp ASC LIMIT ? OFFSET ?",
    )
    .bind(limit)
    .bind(offset)
    .fetch_all(pool)
    .await
    .context("Failed to get chat history")?;

    Ok(rows)
}

/// Clear all chat history
pub async fn clear_chat_history(pool: &SqlitePool) -> Result<()> {
    sqlx::query("DELETE FROM chat_messages")
        .execute(pool)
        .await
        .context("Failed to clear chat history")?;

    Ok(())
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
        let (pool, _temp_dir) = setup_test_db().await;
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
        let (pool, _temp_dir) = setup_test_db().await;
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
        let (pool, _temp_dir) = setup_test_db().await;

        let task1 = create_test_task();
        let task2 = create_test_task();

        insert_task(&pool, &task1).await.unwrap();
        insert_task(&pool, &task2).await.unwrap();

        let pending = list_tasks_by_status(&pool, TaskStatus::Pending)
            .await
            .unwrap();
        assert_eq!(pending.len(), 2);

        pool.close().await;
    }

    // Chat message tests

    #[tokio::test]
    async fn can_insert_and_get_chat_message() {
        let (pool, _temp_dir) = setup_test_db().await;
        let id = uuid::Uuid::new_v4();

        insert_chat_message(&pool, &id, "user", "Hello, world!")
            .await
            .unwrap();

        let history = get_chat_history(&pool, 50, 0).await.unwrap();
        assert_eq!(history.len(), 1);
        assert_eq!(history[0].role, "user");
        assert_eq!(history[0].content, "Hello, world!");

        pool.close().await;
    }

    #[tokio::test]
    async fn chat_history_pagination_works() {
        let (pool, _temp_dir) = setup_test_db().await;

        // Insert 5 messages
        for i in 0..5 {
            let id = uuid::Uuid::new_v4();
            insert_chat_message(&pool, &id, "user", &format!("Message {}", i))
                .await
                .unwrap();
        }

        // Get first 2
        let history = get_chat_history(&pool, 2, 0).await.unwrap();
        assert_eq!(history.len(), 2);

        // Get next 2
        let history = get_chat_history(&pool, 2, 2).await.unwrap();
        assert_eq!(history.len(), 2);

        // Get last 1
        let history = get_chat_history(&pool, 2, 4).await.unwrap();
        assert_eq!(history.len(), 1);

        pool.close().await;
    }

    #[tokio::test]
    async fn can_clear_chat_history() {
        let (pool, _temp_dir) = setup_test_db().await;

        // Insert some messages
        for _ in 0..3 {
            let id = uuid::Uuid::new_v4();
            insert_chat_message(&pool, &id, "user", "Test message")
                .await
                .unwrap();
        }

        // Verify they exist
        let history = get_chat_history(&pool, 50, 0).await.unwrap();
        assert_eq!(history.len(), 3);

        // Clear history
        clear_chat_history(&pool).await.unwrap();

        // Verify empty
        let history = get_chat_history(&pool, 50, 0).await.unwrap();
        assert_eq!(history.len(), 0);

        pool.close().await;
    }
}
