//! Database query helpers

use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use sqlx::PgPool;
use uuid::Uuid;

use crate::types::{Priority, Task, TaskId, TaskStatus, UserId};

/// Insert a new task into the database
pub async fn insert_task(pool: &PgPool, user_id: UserId, task: &Task) -> Result<()> {
    let agent_id = task.assigned_agent.as_ref().map(|a| a.0);
    let status = format!("{:?}", task.status).to_lowercase();
    let priority = format!("{:?}", task.priority).to_lowercase();
    let context_files = serde_json::to_value(&task.context_files)?;
    let metadata = task
        .metadata
        .as_ref()
        .map(serde_json::to_value)
        .transpose()?;

    sqlx::query(
        r#"
        INSERT INTO tasks (id, user_id, slice_id, title, description, assigned_agent, status, priority, context_files, metadata, created_at, updated_at)
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12)
        "#,
    )
    .bind(task.id.0)
    .bind(user_id.0)
    .bind(task.slice_id.as_ref().map(|s| s.0))
    .bind(&task.title)
    .bind(&task.description)
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
    let row: Option<TaskRow> =
        sqlx::query_as("SELECT id, slice_id, title, description, assigned_agent, status, priority, context_files, metadata, created_at, updated_at FROM tasks WHERE id = $1 AND user_id = $2")
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
        "SELECT id, slice_id, title, description, assigned_agent, status, priority, context_files, metadata, created_at, updated_at FROM tasks WHERE status = $1 ORDER BY created_at DESC",
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
    user_id: UserId,
    status: Option<&str>,
    limit: Option<u32>,
) -> Result<Vec<Task>> {
    let limit = limit
        .unwrap_or(crate::constants::DEFAULT_QUERY_LIMIT as u32)
        .min(crate::constants::MAX_QUERY_LIMIT as u32) as i64;

    let rows: Vec<TaskRow> = if let Some(status_filter) = status {
        sqlx::query_as(
            "SELECT id, slice_id, title, description, assigned_agent, status, priority, context_files, metadata, created_at, updated_at FROM tasks WHERE status = $1 AND user_id = $2 ORDER BY created_at DESC LIMIT $3"
        )
        .bind(status_filter)
        .bind(user_id.0)
        .bind(limit)
        .fetch_all(pool)
        .await
        .context("Failed to list tasks")?
    } else {
        sqlx::query_as(
            "SELECT id, slice_id, title, description, assigned_agent, status, priority, context_files, metadata, created_at, updated_at FROM tasks WHERE user_id = $1 ORDER BY created_at DESC LIMIT $2"
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
pub async fn insert_chat_message(
    pool: &PgPool,
    user_id: UserId,
    id: &Uuid,
    role: &str,
    content: &str,
) -> Result<()> {
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
pub async fn get_chat_history(
    pool: &PgPool,
    user_id: UserId,
    limit: u32,
    offset: u32,
) -> Result<Vec<ChatMessageRow>> {
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
    pub agent_id: Option<Uuid>,
    pub draft_config: Option<serde_json::Value>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Create a new chat session
pub async fn create_session(
    pool: &PgPool,
    user_id: UserId,
    session_id: Uuid,
    mode_id: &str,
    title: &str,
    agent_id: Option<Uuid>,
    draft_config: Option<serde_json::Value>,
) -> Result<()> {
    sqlx::query("INSERT INTO chat_sessions (id, user_id, mode_id, title, agent_id, draft_config) VALUES ($1, $2, $3, $4, $5, $6)")
        .bind(session_id)
        .bind(user_id.0)
        .bind(mode_id)
        .bind(title)
        .bind(agent_id)
        .bind(draft_config)
        .execute(pool)
        .await
        .context("Failed to create session")?;
    Ok(())
}

/// List sessions for a user
pub async fn list_sessions(pool: &PgPool, user_id: UserId) -> Result<Vec<SessionRow>> {
    let rows: Vec<SessionRow> =
        sqlx::query_as("SELECT id, user_id, mode_id, title, summary, agent_id, draft_config, created_at, updated_at FROM chat_sessions WHERE user_id = $1 ORDER BY updated_at DESC")
            .bind(user_id.0)
            .fetch_all(pool)
            .await
            .context("Failed to list sessions")?;
    Ok(rows)
}

/// Get a session by ID
pub async fn get_session(pool: &PgPool, session_id: Uuid) -> Result<Option<SessionRow>> {
    let row: Option<SessionRow> = sqlx::query_as("SELECT id, user_id, mode_id, title, summary, agent_id, draft_config, created_at, updated_at FROM chat_sessions WHERE id = $1")
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
pub async fn insert_session_message(
    pool: &PgPool,
    user_id: UserId,
    session_id: Uuid,
    id: &Uuid,
    role: &str,
    content: &str,
) -> Result<()> {
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
pub async fn get_session_history(
    pool: &PgPool,
    session_id: Uuid,
    limit: u32,
) -> Result<Vec<ChatMessageRow>> {
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

/// Update session draft_config
pub async fn update_session_draft_config(
    pool: &PgPool,
    session_id: Uuid,
    draft_config: Option<serde_json::Value>,
) -> Result<()> {
    sqlx::query("UPDATE chat_sessions SET draft_config = $2, updated_at = NOW() WHERE id = $1")
        .bind(session_id)
        .bind(draft_config)
        .execute(pool)
        .await
        .context("Failed to update session draft_config")?;
    Ok(())
}

/// Clear all messages for a session
pub async fn clear_session_messages(pool: &PgPool, session_id: Uuid) -> Result<()> {
    sqlx::query("DELETE FROM chat_messages WHERE session_id = $1")
        .bind(session_id)
        .execute(pool)
        .await
        .context("Failed to clear session messages")?;
    Ok(())
}

/// Link an agent to a session (and clear draft_config)
pub async fn link_session_agent(pool: &PgPool, session_id: Uuid, agent_id: Uuid) -> Result<()> {
    sqlx::query(
        "UPDATE chat_sessions SET agent_id = $2, draft_config = NULL, updated_at = NOW() WHERE id = $1",
    )
    .bind(session_id)
    .bind(agent_id)
    .execute(pool)
    .await
    .context("Failed to link agent to session")?;
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

/// List tools belonging to a user.
///
/// Note: This replaces the old `list_tools_by_cluster` function.
/// Cluster-based tool lookup is no longer supported.
pub async fn list_tools_for_user(pool: &PgPool, user_id: Uuid) -> Result<Vec<super::ToolRow>> {
    let rows: Vec<ToolRowDb> = sqlx::query_as("SELECT id, user_id, name, display_name, description, parameters, created_at, version FROM tools WHERE user_id = $1 ORDER BY name")
        .bind(user_id)
        .fetch_all(pool)
        .await
        .context("Failed to list tools for user")?;

    Ok(rows
        .into_iter()
        .map(|r| super::ToolRow {
            id: r.id,
            user_id: r.user_id,
            name: r.name,
            display_name: r.display_name,
            description: r.description,
            parameters: r.parameters,
            created_at: r.created_at,
            version: r.version,
        })
        .collect())
}

#[derive(sqlx::FromRow)]
struct ToolRowDb {
    id: Uuid,
    user_id: Uuid,
    name: String,
    display_name: String,
    description: String,
    parameters: serde_json::Value,
    created_at: chrono::DateTime<chrono::Utc>,
    version: i32,
}

// ============================================================================
// Agent Mode Queries
// ============================================================================

/// An agent mode row — config overlay for dynamic LLM-driven mode switching.
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct AgentModeRow {
    pub id: Uuid,
    pub agent_id: Uuid,
    pub name: String,
    pub system_prompt_suffix: Option<String>,
    pub temperature_override: Option<f64>,
    pub model_override: Option<String>,
    pub tool_overrides: Option<Vec<String>>,
    pub classifier_hint: String,
    pub created_at: DateTime<Utc>,
    pub version: i32,
}

/// List all modes for an agent.
pub async fn list_agent_modes(pool: &PgPool, agent_id: Uuid) -> Result<Vec<AgentModeRow>> {
    let rows: Vec<AgentModeRow> = sqlx::query_as(
        "SELECT id, agent_id, name, system_prompt_suffix, temperature_override, model_override, tool_overrides, classifier_hint, created_at, version FROM agent_modes WHERE agent_id = $1 ORDER BY name",
    )
    .bind(agent_id)
    .fetch_all(pool)
    .await
    .context("Failed to list agent modes")?;
    Ok(rows)
}

/// Create an agent mode.
pub async fn create_agent_mode(pool: &PgPool, mode: &AgentModeRow) -> Result<()> {
    sqlx::query("INSERT INTO agent_modes (id, agent_id, name, system_prompt_suffix, temperature_override, model_override, tool_overrides, classifier_hint) VALUES ($1, $2, $3, $4, $5, $6, $7, $8)")
        .bind(mode.id)
        .bind(mode.agent_id)
        .bind(&mode.name)
        .bind(&mode.system_prompt_suffix)
        .bind(mode.temperature_override)
        .bind(&mode.model_override)
        .bind(&mode.tool_overrides)
        .bind(&mode.classifier_hint)
        .execute(pool)
        .await
        .context("Failed to create agent mode")?;
    Ok(())
}

/// Delete an agent mode by ID.
pub async fn delete_agent_mode(pool: &PgPool, mode_id: Uuid) -> Result<()> {
    sqlx::query("DELETE FROM agent_modes WHERE id = $1")
        .bind(mode_id)
        .execute(pool)
        .await
        .context("Failed to delete agent mode")?;
    Ok(())
}

#[cfg(test)]
mod tests;
