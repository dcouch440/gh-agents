//! Database query helpers

use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use sqlx::PgPool;
use uuid::Uuid;

use crate::types::UserId;

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
    /// Message source: None = human, Some("agent") = agent-injected, Some("system") = system notification.
    pub source_type: Option<String>,
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

    let rows: Vec<ChatMessageRow> = sqlx::query_as("SELECT id, role, content, timestamp, source_type FROM chat_messages WHERE user_id = $1 ORDER BY timestamp ASC LIMIT $2 OFFSET $3")
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

/// Find a chat session linked to a workflow step via draft_config->>'step_id'.
///
/// Excludes builder sessions (L4) by filtering on the `role` field — only
/// returns sessions where role is absent or equals 'assistant'.
pub async fn find_session_by_step_id(pool: &PgPool, step_id: Uuid) -> Result<Option<SessionRow>> {
    let row: Option<SessionRow> = sqlx::query_as(
        "SELECT id, user_id, mode_id, title, summary, agent_id, draft_config, created_at, updated_at \
         FROM chat_sessions \
         WHERE draft_config->>'step_id' = $1 \
           AND COALESCE(draft_config->>'role', 'assistant') = 'assistant'"
    )
        .bind(step_id.to_string())
        .fetch_optional(pool)
        .await
        .context("Failed to find session by step_id")?;
    Ok(row)
}

/// Find the L4 builder session for a workflow step.
///
/// Builder sessions have `draft_config->>'role' = 'builder'` to distinguish
/// them from the L3 assistant session on the same step.
pub async fn find_builder_session_by_step_id(
    pool: &PgPool,
    step_id: Uuid,
) -> Result<Option<SessionRow>> {
    find_session_by_step_id_and_role(pool, step_id, "builder").await
}

/// Find a dispatch session for a step by role.
///
/// Generalizes `find_builder_session_by_step_id` to support multiple roles
/// (e.g., `"builder"`, `"system_agent"`).
pub async fn find_session_by_step_id_and_role(
    pool: &PgPool,
    step_id: Uuid,
    role: &str,
) -> Result<Option<SessionRow>> {
    let row: Option<SessionRow> = sqlx::query_as(
        "SELECT id, user_id, mode_id, title, summary, agent_id, draft_config, created_at, updated_at \
         FROM chat_sessions \
         WHERE draft_config->>'step_id' = $1 \
           AND draft_config->>'role' = $2"
    )
        .bind(step_id.to_string())
        .bind(role)
        .fetch_optional(pool)
        .await
        .context("Failed to find session by step_id and role")?;
    Ok(row)
}

/// Find the L2 manager builder session for a workflow.
///
/// Manager builder sessions have `draft_config->>'role' = 'manager_builder'`
/// and `draft_config->>'workflow_id'` matching the given workflow.
pub async fn find_manager_builder_session(
    pool: &PgPool,
    workflow_id: Uuid,
) -> Result<Option<SessionRow>> {
    let row: Option<SessionRow> = sqlx::query_as(
        "SELECT id, user_id, mode_id, title, summary, agent_id, draft_config, created_at, updated_at \
         FROM chat_sessions \
         WHERE draft_config->>'workflow_id' = $1 \
           AND draft_config->>'role' = 'manager_builder'"
    )
        .bind(workflow_id.to_string())
        .fetch_optional(pool)
        .await
        .context("Failed to find manager builder session")?;
    Ok(row)
}

/// Batch-check which steps have received initial instructions.
///
/// A step is considered "instructed" if its L3 assistant session contains at
/// least one agent-sourced message with `type="initial_instruction"` in the
/// content XML.
pub async fn check_initial_instructions_sent(
    pool: &PgPool,
    step_ids: &[Uuid],
) -> Result<std::collections::HashSet<Uuid>> {
    if step_ids.is_empty() {
        return Ok(std::collections::HashSet::new());
    }

    let step_id_strings: Vec<String> = step_ids.iter().map(|id| id.to_string()).collect();

    let rows: Vec<(String,)> = sqlx::query_as(
        "SELECT DISTINCT cs.draft_config->>'step_id' \
         FROM chat_sessions cs \
         JOIN chat_messages cm ON cm.session_id = cs.id \
         WHERE cs.draft_config->>'step_id' = ANY($1) \
           AND COALESCE(cs.draft_config->>'role', 'assistant') = 'assistant' \
           AND cm.source_type = 'agent' \
           AND cm.content LIKE '%type=\"initial_instruction\"%'",
    )
    .bind(&step_id_strings)
    .fetch_all(pool)
    .await
    .context("Failed to check initial instructions")?;

    Ok(rows
        .into_iter()
        .filter_map(|(id_str,)| Uuid::parse_str(&id_str).ok())
        .collect())
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

/// Insert an agent-sourced message into a session.
pub async fn insert_agent_session_message(
    pool: &PgPool,
    user_id: UserId,
    session_id: Uuid,
    id: &Uuid,
    role: &str,
    content: &str,
    source_type: &str,
) -> Result<()> {
    sqlx::query("INSERT INTO chat_messages (id, user_id, session_id, role, content, timestamp, source_type) VALUES ($1, $2, $3, $4, $5, $6, $7)")
        .bind(id)
        .bind(user_id.0)
        .bind(session_id)
        .bind(role)
        .bind(content)
        .bind(Utc::now())
        .bind(source_type)
        .execute(pool)
        .await
        .context("Failed to insert agent session message")?;
    Ok(())
}

/// Get chat history for a session
pub async fn get_session_history(
    pool: &PgPool,
    session_id: Uuid,
    limit: u32,
) -> Result<Vec<ChatMessageRow>> {
    let limit = limit.min(1000) as i64;
    let rows: Vec<ChatMessageRow> = sqlx::query_as("SELECT id, role, content, timestamp, source_type FROM chat_messages WHERE session_id = $1 ORDER BY timestamp ASC LIMIT $2")
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
    name: String,
    display_name: String,
    description: String,
    parameters: serde_json::Value,
    created_at: chrono::DateTime<chrono::Utc>,
    version: i32,
}

// ============================================================================
#[cfg(test)]
mod tests;
