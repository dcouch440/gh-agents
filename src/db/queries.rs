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
}
