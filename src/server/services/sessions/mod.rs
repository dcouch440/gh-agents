//! Session service: create, read, update, delete sessions and session chat.

use uuid::Uuid;

use crate::db::traits::{AgentRepo, SessionRepo};
use crate::db::{ChatMessageRow, SessionRow};
use crate::types::UserId;

use super::error::ServiceError;

/// Input for creating a new session.
pub struct CreateSessionInput {
    pub user_id: UserId,
    pub mode_id: String,
    pub agent_id: Option<Uuid>,
    pub title: String,
    pub draft_config: Option<serde_json::Value>,
}

/// Verify the caller owns this session. Returns the session row on success.
async fn verify_ownership(
    repo: &dyn SessionRepo,
    user_id: Uuid,
    session_id: Uuid,
) -> Result<SessionRow, ServiceError> {
    super::ownership::fetch_and_check_owner(
        || repo.get_session(session_id),
        user_id,
        |s| s.user_id,
        "Session",
    )
    .await
}

/// Create a new session. Validates the agent exists if provided, applies
/// defaults for mode_id and title, then persists and returns the row.
pub async fn create_session(
    repo: &dyn SessionRepo,
    agent_repo: &dyn AgentRepo,
    input: CreateSessionInput,
) -> Result<SessionRow, ServiceError> {
    // Validate agent exists if provided
    if let Some(aid) = input.agent_id {
        if agent_repo.get_persisted_agent(aid).await?.is_none() {
            return Err(ServiceError::validation("Agent not found"));
        }
    }

    let session_id = Uuid::new_v4();
    let mode_id = if input.mode_id.is_empty() {
        "home".to_string()
    } else {
        input.mode_id
    };
    let title = if input.title.is_empty() {
        "New session".to_string()
    } else {
        input.title
    };

    repo.create_session(
        input.user_id,
        session_id,
        &mode_id,
        &title,
        input.agent_id,
        input.draft_config,
    )
    .await?;

    let session = repo.get_session(session_id).await?.ok_or_else(|| {
        ServiceError::Internal(anyhow::anyhow!("Session not found after creation"))
    })?;

    Ok(session)
}

/// Get a single session by ID, verifying ownership.
pub async fn get_session(
    repo: &dyn SessionRepo,
    user_id: Uuid,
    session_id: Uuid,
) -> Result<SessionRow, ServiceError> {
    verify_ownership(repo, user_id, session_id).await
}

/// List sessions for a user.
pub async fn list_sessions(
    repo: &dyn SessionRepo,
    user_id: UserId,
) -> Result<Vec<SessionRow>, ServiceError> {
    let rows = repo.list_sessions(user_id).await?;
    Ok(rows)
}

/// Update a session title, verifying ownership. Returns the updated row.
pub async fn update_session(
    repo: &dyn SessionRepo,
    user_id: Uuid,
    session_id: Uuid,
    title: &str,
) -> Result<SessionRow, ServiceError> {
    verify_ownership(repo, user_id, session_id).await?;

    repo.update_session_title(session_id, title).await?;

    let updated = repo
        .get_session(session_id)
        .await?
        .ok_or_else(|| ServiceError::Internal(anyhow::anyhow!("Session not found after update")))?;

    Ok(updated)
}

/// Delete a session, verifying ownership.
pub async fn delete_session(
    repo: &dyn SessionRepo,
    user_id: Uuid,
    session_id: Uuid,
) -> Result<(), ServiceError> {
    verify_ownership(repo, user_id, session_id).await?;
    repo.delete_session(session_id).await?;
    Ok(())
}

/// Validate a chat message and verify session ownership. Returns the session
/// row so the handler can use it for stream/queue logic.
pub async fn verify_session_chat(
    repo: &dyn SessionRepo,
    user_id: Uuid,
    session_id: Uuid,
    message: &str,
) -> Result<SessionRow, ServiceError> {
    if message.trim().is_empty() {
        return Err(ServiceError::validation("Message cannot be empty"));
    }
    verify_ownership(repo, user_id, session_id).await
}

/// Get session chat history, verifying ownership.
pub async fn get_session_history(
    repo: &dyn SessionRepo,
    user_id: Uuid,
    session_id: Uuid,
    limit: u32,
) -> Result<Vec<ChatMessageRow>, ServiceError> {
    verify_ownership(repo, user_id, session_id).await?;
    let rows = repo.get_session_history(session_id, limit).await?;
    Ok(rows)
}

/// Clear all messages in a session, verifying ownership.
pub async fn clear_session_messages(
    repo: &dyn SessionRepo,
    user_id: Uuid,
    session_id: Uuid,
) -> Result<(), ServiceError> {
    verify_ownership(repo, user_id, session_id).await?;
    repo.clear_session_messages(session_id).await?;
    Ok(())
}

mod tests;
