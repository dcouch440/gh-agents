//! Database query helpers

use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use sqlx::PgPool;
use uuid::Uuid;

use crate::types::{AgentTier, Priority, Task, TaskId, TaskStatus, UserId};

/// Insert a new task into the database
pub async fn insert_task(pool: &PgPool, user_id: UserId, task: &Task) -> Result<()> {
    let tier = format!("{:?}", task.assigned_tier).to_lowercase();
    let agent_id = task.assigned_agent.as_ref().map(|a| a.0);
    let status = format!("{:?}", task.status).to_lowercase();
    let priority = format!("{:?}", task.priority).to_lowercase();
    let context_files = serde_json::to_value(&task.context_files)?;
    let metadata = task.metadata.as_ref().map(serde_json::to_value).transpose()?;

    sqlx::query(
        r#"
        INSERT INTO tasks (id, user_id, slice_id, title, description, assigned_tier, assigned_agent, status, priority, context_files, metadata, created_at, updated_at)
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13)
        "#,
    )
    .bind(task.id.0)
    .bind(user_id.0)
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
pub async fn get_task(pool: &PgPool, user_id: UserId, id: &TaskId) -> Result<Option<Task>> {
    let row: Option<TaskRow> = sqlx::query_as(
        "SELECT id, slice_id, title, description, assigned_tier, assigned_agent, status, priority, context_files, metadata, created_at, updated_at FROM tasks WHERE id = $1 AND user_id = $2",
    )
    .bind(id.0)
    .bind(user_id.0)
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
pub async fn list_tasks(pool: &PgPool, user_id: UserId, status: Option<&str>, limit: Option<u32>) -> Result<Vec<Task>> {
    let limit = limit.unwrap_or(crate::constants::DEFAULT_QUERY_LIMIT as u32).min(crate::constants::MAX_QUERY_LIMIT as u32) as i64;

    let rows: Vec<TaskRow> = if let Some(status_filter) = status {
        sqlx::query_as(
            "SELECT id, slice_id, title, description, assigned_tier, assigned_agent, status, priority, context_files, metadata, created_at, updated_at FROM tasks WHERE status = $1 AND user_id = $2 ORDER BY created_at DESC LIMIT $3"
        )
        .bind(status_filter)
        .bind(user_id.0)
        .bind(limit)
        .fetch_all(pool)
        .await
        .context("Failed to list tasks")?
    } else {
        sqlx::query_as(
            "SELECT id, slice_id, title, description, assigned_tier, assigned_agent, status, priority, context_files, metadata, created_at, updated_at FROM tasks WHERE user_id = $1 ORDER BY created_at DESC LIMIT $2"
        )
        .bind(user_id.0)
        .bind(limit)
        .fetch_all(pool)
        .await
        .context("Failed to list tasks")?
    };

    Ok(rows.into_iter().map(|r| r.into_task()).collect())
}

/// Get a task by UUID (string version for API)
pub async fn get_task_by_uuid(pool: &PgPool, user_id: UserId, id: Uuid) -> Result<Option<Task>> {
    let task_id = TaskId(id);
    get_task(pool, user_id, &task_id).await
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
    retry_count: Option<i32>,
    max_retries: Option<i32>,
    last_error: Option<String>,
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

        let context_files: Vec<std::path::PathBuf> = serde_json::from_value(self.context_files).unwrap_or_default();
        let metadata: Option<std::collections::HashMap<String, String>> = self.metadata.and_then(|m| serde_json::from_value(m).ok());

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
            retry_count: self.retry_count.unwrap_or(0) as u32,
            max_retries: self.max_retries.unwrap_or(3) as u32,
            last_error: self.last_error,
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
pub async fn insert_chat_message(pool: &PgPool, user_id: UserId, id: &Uuid, role: &str, content: &str) -> Result<()> {
    sqlx::query("INSERT INTO chat_messages (id, user_id, role, content, timestamp) VALUES ($1, $2, $3, $4, $5)")
        .bind(id)
        .bind(user_id.0)
        .bind(role)
        .bind(content)
        .bind(Utc::now())
        .execute(pool)
        .await
        .context("Failed to insert chat message")?;

    Ok(())
}

/// Get chat history with pagination
pub async fn get_chat_history(pool: &PgPool, user_id: UserId, limit: u32, offset: u32) -> Result<Vec<ChatMessageRow>> {
    let limit = limit.min(1000) as i64;
    let offset = offset as i64;

    let rows: Vec<ChatMessageRow> = sqlx::query_as("SELECT id, role, content, timestamp FROM chat_messages WHERE user_id = $1 ORDER BY timestamp ASC LIMIT $2 OFFSET $3")
        .bind(user_id.0)
        .bind(limit)
        .bind(offset)
        .fetch_all(pool)
        .await
        .context("Failed to get chat history")?;

    Ok(rows)
}

/// Clear all chat history
pub async fn clear_chat_history(pool: &PgPool, user_id: UserId) -> Result<()> {
    sqlx::query("DELETE FROM chat_messages WHERE user_id = $1")
        .bind(user_id.0)
        .execute(pool)
        .await
        .context("Failed to clear chat history")?;

    Ok(())
}

// ============================================================================
// Session Queries
// ============================================================================

/// A chat session row
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct SessionRow {
    pub id: Uuid,
    pub user_id: Uuid,
    pub mode_id: String,
    pub title: String,
    pub summary: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Create a new chat session
pub async fn create_session(pool: &PgPool, user_id: UserId, session_id: Uuid, mode_id: &str, title: &str) -> Result<()> {
    sqlx::query("INSERT INTO chat_sessions (id, user_id, mode_id, title) VALUES ($1, $2, $3, $4)")
        .bind(session_id)
        .bind(user_id.0)
        .bind(mode_id)
        .bind(title)
        .execute(pool)
        .await
        .context("Failed to create session")?;
    Ok(())
}

/// List sessions for a user
pub async fn list_sessions(pool: &PgPool, user_id: UserId) -> Result<Vec<SessionRow>> {
    let rows: Vec<SessionRow> = sqlx::query_as("SELECT id, user_id, mode_id, title, summary, created_at, updated_at FROM chat_sessions WHERE user_id = $1 ORDER BY updated_at DESC")
        .bind(user_id.0)
        .fetch_all(pool)
        .await
        .context("Failed to list sessions")?;
    Ok(rows)
}

/// Get a session by ID
pub async fn get_session(pool: &PgPool, session_id: Uuid) -> Result<Option<SessionRow>> {
    let row: Option<SessionRow> = sqlx::query_as("SELECT id, user_id, mode_id, title, summary, created_at, updated_at FROM chat_sessions WHERE id = $1")
        .bind(session_id)
        .fetch_optional(pool)
        .await
        .context("Failed to get session")?;
    Ok(row)
}

/// Delete a session and its messages
pub async fn delete_session(pool: &PgPool, session_id: Uuid) -> Result<()> {
    sqlx::query("DELETE FROM chat_messages WHERE session_id = $1")
        .bind(session_id)
        .execute(pool)
        .await
        .context("Failed to delete session messages")?;
    sqlx::query("DELETE FROM chat_sessions WHERE id = $1")
        .bind(session_id)
        .execute(pool)
        .await
        .context("Failed to delete session")?;
    Ok(())
}

/// Insert a chat message scoped to a session
pub async fn insert_session_message(pool: &PgPool, user_id: UserId, session_id: Uuid, id: &Uuid, role: &str, content: &str) -> Result<()> {
    sqlx::query("INSERT INTO chat_messages (id, user_id, session_id, role, content, timestamp) VALUES ($1, $2, $3, $4, $5, $6)")
        .bind(id)
        .bind(user_id.0)
        .bind(session_id)
        .bind(role)
        .bind(content)
        .bind(Utc::now())
        .execute(pool)
        .await
        .context("Failed to insert session message")?;
    Ok(())
}

/// Get chat history for a session
pub async fn get_session_history(pool: &PgPool, session_id: Uuid, limit: u32) -> Result<Vec<ChatMessageRow>> {
    let limit = limit.min(1000) as i64;
    let rows: Vec<ChatMessageRow> = sqlx::query_as("SELECT id, role, content, timestamp FROM chat_messages WHERE session_id = $1 ORDER BY timestamp ASC LIMIT $2")
        .bind(session_id)
        .bind(limit)
        .fetch_all(pool)
        .await
        .context("Failed to get session history")?;
    Ok(rows)
}

/// Update session title
pub async fn update_session_title(pool: &PgPool, session_id: Uuid, title: &str) -> Result<()> {
    sqlx::query("UPDATE chat_sessions SET title = $2, updated_at = NOW() WHERE id = $1")
        .bind(session_id)
        .bind(title)
        .execute(pool)
        .await
        .context("Failed to update session title")?;
    Ok(())
}

/// Update session updated_at timestamp
pub async fn touch_session(pool: &PgPool, session_id: Uuid) -> Result<()> {
    sqlx::query("UPDATE chat_sessions SET updated_at = NOW() WHERE id = $1")
        .bind(session_id)
        .execute(pool)
        .await
        .context("Failed to touch session")?;
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
    let row: Option<(String,)> = sqlx::query_as("SELECT password_hash FROM auth_config WHERE id = 1")
        .fetch_optional(pool)
        .await
        .context("Failed to get password")?;

    Ok(row.map(|r| r.0))
}

// ============================================================================
// Routing Event Queries
// ============================================================================

/// Insert a new routing event (when a tool call is routed to a cluster).
/// Captures user, session, task context and the raw parameters for analytics.
pub async fn insert_routing_event(
    pool: &PgPool,
    id: Uuid,
    user_id: Option<Uuid>,
    session_id: Option<Uuid>,
    task_id: Option<Uuid>,
    router_agent_id: Uuid,
    cluster_id: Option<Uuid>,
    cluster_name: &str,
    tool_name: &str,
    request: &str,
    parameters: &serde_json::Value,
) -> Result<()> {
    sqlx::query(
        r#"INSERT INTO routing_events
           (id, user_id, session_id, task_id, router_agent_id, cluster_id, cluster_name, tool_name, request, parameters, status)
           VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, 'pending')"#,
    )
    .bind(id)
    .bind(user_id)
    .bind(session_id)
    .bind(task_id)
    .bind(router_agent_id)
    .bind(cluster_id)
    .bind(cluster_name)
    .bind(tool_name)
    .bind(request)
    .bind(parameters)
    .execute(pool)
    .await
    .context("Failed to insert routing event")?;
    Ok(())
}

/// Complete a routing event with result, cost, and timing data.
pub async fn complete_routing_event(
    pool: &PgPool,
    id: Uuid,
    cluster_agent_id: Option<Uuid>,
    response: Option<&str>,
    error: Option<&str>,
    status: &str,
    agent_tier: Option<&str>,
    model_id: Option<&str>,
    input_tokens: i64,
    output_tokens: i64,
    duration_ms: i64,
) -> Result<()> {
    sqlx::query(
        r#"UPDATE routing_events
           SET cluster_agent_id = $2, response = $3, error = $4, status = $5,
               agent_tier = $6, model_id = $7,
               input_tokens = $8, output_tokens = $9, duration_ms = $10,
               completed_at = NOW()
           WHERE id = $1"#,
    )
    .bind(id)
    .bind(cluster_agent_id)
    .bind(response)
    .bind(error)
    .bind(status)
    .bind(agent_tier)
    .bind(model_id)
    .bind(input_tokens)
    .bind(output_tokens)
    .bind(duration_ms)
    .execute(pool)
    .await
    .context("Failed to complete routing event")?;
    Ok(())
}

/// List tools belonging to a cluster
pub async fn list_tools_by_cluster(pool: &PgPool, cluster_id: Uuid) -> Result<Vec<super::ToolRow>> {
    let rows: Vec<ToolRowDb> =
        sqlx::query_as("SELECT id, name, description, category, parameter_schema, output_schema, enabled, cluster_id, is_builtin FROM tools WHERE cluster_id = $1 AND enabled = true")
            .bind(cluster_id)
            .fetch_all(pool)
            .await
            .context("Failed to list tools by cluster")?;

    Ok(rows
        .into_iter()
        .map(|r| super::ToolRow {
            id: r.id,
            name: r.name,
            description: r.description,
            category: r.category,
            parameter_schema: r.parameter_schema,
            output_schema: r.output_schema,
            enabled: r.enabled,
            cluster_id: r.cluster_id,
            is_builtin: r.is_builtin,
        })
        .collect())
}

#[derive(sqlx::FromRow)]
struct ToolRowDb {
    id: Uuid,
    name: String,
    description: String,
    category: String,
    parameter_schema: serde_json::Value,
    output_schema: serde_json::Value,
    enabled: bool,
    cluster_id: Option<Uuid>,
    is_builtin: bool,
}

/// List all clusters with their tools (for building the ToolClusterIndex)
pub async fn list_clusters_with_tools(pool: &PgPool) -> Result<Vec<(super::ClusterRow, Vec<super::ToolRow>)>> {
    let cluster_rows: Vec<ClusterRowDb> = sqlx::query_as("SELECT id, name, description, conventions, shared_files FROM clusters")
        .fetch_all(pool)
        .await
        .context("Failed to list clusters")?;

    let mut results = Vec::new();
    for cr in cluster_rows {
        let tool_rows = list_tools_by_cluster(pool, cr.id).await?;
        let cluster = super::ClusterRow {
            id: cr.id,
            name: cr.name,
            description: cr.description,
            conventions: cr.conventions,
            shared_files: cr.shared_files,
        };
        results.push((cluster, tool_rows));
    }
    Ok(results)
}

#[derive(sqlx::FromRow)]
struct ClusterRowDb {
    id: Uuid,
    name: String,
    description: String,
    conventions: String,
    shared_files: serde_json::Value,
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::db::test_utils::TestDb;
    use crate::types::UserId;

    fn test_user_id() -> UserId {
        UserId(uuid::Uuid::parse_str("00000000-0000-0000-0000-000000000001").unwrap())
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
            retry_count: 0,
            max_retries: 3,
            last_error: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }

    #[tokio::test]
    #[ignore = "requires running Postgres"]
    async fn can_insert_and_get_task() {
        let db = TestDb::new().await;
        let task = create_test_task();

        insert_task(&db.pool, test_user_id(), &task).await.unwrap();

        let retrieved = get_task(&db.pool, test_user_id(), &task.id).await.unwrap();
        assert!(retrieved.is_some());
        let retrieved = retrieved.unwrap();
        assert_eq!(retrieved.title, task.title);

        db.cleanup().await;
    }

    #[tokio::test]
    #[ignore = "requires running Postgres"]
    async fn can_update_task_status() {
        let db = TestDb::new().await;
        let task = create_test_task();

        insert_task(&db.pool, test_user_id(), &task).await.unwrap();
        update_task_status(&db.pool, &task.id, TaskStatus::InProgress).await.unwrap();

        let retrieved = get_task(&db.pool, test_user_id(), &task.id).await.unwrap().unwrap();
        assert_eq!(retrieved.status, TaskStatus::InProgress);

        db.cleanup().await;
    }

    #[tokio::test]
    #[ignore = "requires running Postgres"]
    async fn can_list_tasks_by_status() {
        let db = TestDb::new().await;

        let task1 = create_test_task();
        let task2 = create_test_task();

        insert_task(&db.pool, test_user_id(), &task1).await.unwrap();
        insert_task(&db.pool, test_user_id(), &task2).await.unwrap();

        let pending = list_tasks_by_status(&db.pool, TaskStatus::Pending).await.unwrap();
        assert!(pending.len() >= 2);

        db.cleanup().await;
    }

    // Chat message tests

    #[tokio::test]
    #[ignore = "requires running Postgres"]
    async fn can_insert_and_get_chat_message() {
        let db = TestDb::new().await;
        let id = Uuid::new_v4();

        insert_chat_message(&db.pool, test_user_id(), &id, "user", "Hello, world!").await.unwrap();

        let history = get_chat_history(&db.pool, test_user_id(), 50, 0).await.unwrap();
        assert!(history.len() >= 1);

        db.cleanup().await;
    }

    #[tokio::test]
    #[ignore = "requires running Postgres"]
    async fn chat_history_pagination_works() {
        let db = TestDb::new().await;

        // Insert 5 messages
        for i in 0..5 {
            let id = Uuid::new_v4();
            insert_chat_message(&db.pool, test_user_id(), &id, "user", &format!("Message {}", i)).await.unwrap();
        }

        // Get first 2
        let history = get_chat_history(&db.pool, test_user_id(), 2, 0).await.unwrap();
        assert_eq!(history.len(), 2);

        db.cleanup().await;
    }

    #[tokio::test]
    #[ignore = "requires running Postgres"]
    async fn can_clear_chat_history() {
        let db = TestDb::new().await;

        for _ in 0..3 {
            let id = Uuid::new_v4();
            insert_chat_message(&db.pool, test_user_id(), &id, "user", "Test message").await.unwrap();
        }

        let history = get_chat_history(&db.pool, test_user_id(), 50, 0).await.unwrap();
        assert!(history.len() >= 3);

        clear_chat_history(&db.pool, test_user_id()).await.unwrap();

        let history = get_chat_history(&db.pool, test_user_id(), 50, 0).await.unwrap();
        assert_eq!(history.len(), 0);

        db.cleanup().await;
    }

    // Auth tests

    #[tokio::test]
    #[ignore = "requires running Postgres"]
    async fn test_password_flow() {
        let db = TestDb::new().await;

        // Initially no password
        assert!(!has_password(&db.pool).await.unwrap());
        assert!(get_password(&db.pool).await.unwrap().is_none());

        // Set password
        set_password(&db.pool, "test_hash_123").await.unwrap();

        // Now has password
        assert!(has_password(&db.pool).await.unwrap());
        let stored = get_password(&db.pool).await.unwrap();
        assert_eq!(stored, Some("test_hash_123".to_string()));

        db.cleanup().await;
    }

    #[tokio::test]
    #[ignore = "requires running Postgres"]
    async fn test_set_password_twice_fails() {
        let db = TestDb::new().await;

        // Set password first time
        set_password(&db.pool, "hash1").await.unwrap();

        // Setting again should fail (unique constraint on id=1)
        let result = set_password(&db.pool, "hash2").await;
        assert!(result.is_err());

        // Original password should still be there
        let stored = get_password(&db.pool).await.unwrap();
        assert_eq!(stored, Some("hash1".to_string()));

        db.cleanup().await;
    }
}
