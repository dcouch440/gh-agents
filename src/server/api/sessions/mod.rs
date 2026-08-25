//! Session and agent mode management endpoints

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    Json,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::AppError;
use crate::server::auth as auth_utils;
use crate::server::services::sessions::{self, CreateSessionInput};
use crate::server::state::{AppState, ConsumerMessage};
use crate::server::ws::events::{SessionEvent, SessionEventKind};

use super::chat::{ChatMessage, ChatRequest, ChatResponse, HistoryQuery};

/// An available agent mode
#[derive(Serialize, utoipa::ToSchema)]
pub struct ModeInfo {
    pub id: String,
    pub name: String,
    pub description: String,
}

/// List available agents (replaces legacy list_modes)
#[utoipa::path(
    get,
    path = "/api/modes",
    tag = "Sessions",
    security(("bearer_auth" = [])),
    responses(
        (status = 200, description = "List of available modes", body = Vec<ModeInfo>)
    )
)]
pub async fn list_modes(
    State(state): State<AppState>,
    auth: auth_utils::AuthUser,
) -> Result<Json<Vec<ModeInfo>>, AppError> {
    let agents = state
        .repos()
        .agents
        .list_persisted_agents(auth.user_id)
        .await?;
    let modes: Vec<ModeInfo> = agents
        .into_iter()
        .map(|a| ModeInfo {
            id: a.id.to_string(),
            name: a.name,
            description: a.system_prompt.chars().take(120).collect(),
        })
        .collect();
    Ok(Json(modes))
}

// ============================================================================
// Session Endpoints
// ============================================================================

/// Request body for creating a session
#[derive(Deserialize, utoipa::ToSchema)]
pub struct CreateSessionRequest {
    #[serde(default)]
    pub mode_id: String,
    #[serde(default)]
    pub agent_id: Option<Uuid>,
    #[serde(default)]
    pub title: String,
    #[serde(default)]
    pub draft_config: Option<serde_json::Value>,
}

/// Request body for updating a session
#[derive(Deserialize, utoipa::ToSchema)]
pub struct UpdateSessionRequest {
    pub title: String,
}

/// Response for session creation
#[derive(Serialize, utoipa::ToSchema)]
pub struct SessionResponse {
    pub id: Uuid,
    pub mode_id: String,
    pub agent_id: Option<Uuid>,
    pub draft_config: Option<serde_json::Value>,
    pub title: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Create a new chat session
#[utoipa::path(
    post,
    path = "/api/sessions",
    tag = "Sessions",
    security(("bearer_auth" = [])),
    request_body = CreateSessionRequest,
    responses(
        (status = 201, description = "Session created", body = SessionResponse),
        (status = 400, description = "Invalid agent ID")
    )
)]
pub async fn create_session(
    State(state): State<AppState>,
    auth: auth_utils::AuthUser,
    Json(request): Json<CreateSessionRequest>,
) -> Result<(StatusCode, Json<SessionResponse>), AppError> {
    let session = sessions::create_session(
        state.repos().sessions.as_ref(),
        state.repos().agents.as_ref(),
        CreateSessionInput {
            user_id: auth.user_id,
            mode_id: request.mode_id,
            agent_id: request.agent_id,
            title: request.title,
            draft_config: request.draft_config,
        },
    )
    .await?;

    state.broadcast_session(SessionEvent {
        session_id: session.id,
        user_id: Some(auth.user_id.0),
        kind: SessionEventKind::Created {
            title: session.title.clone(),
            mode_id: session.mode_id.clone(),
        },
    });

    Ok((
        StatusCode::CREATED,
        Json(SessionResponse {
            id: session.id,
            mode_id: session.mode_id,
            agent_id: session.agent_id,
            draft_config: session.draft_config,
            title: session.title,
            created_at: session.created_at,
            updated_at: session.updated_at,
        }),
    ))
}

/// List sessions for the current user
#[utoipa::path(
    get,
    path = "/api/sessions",
    tag = "Sessions",
    security(("bearer_auth" = [])),
    responses(
        (status = 200, description = "List of sessions", body = Vec<SessionResponse>)
    )
)]
pub async fn list_sessions(
    State(state): State<AppState>,
    auth: auth_utils::AuthUser,
) -> Result<Json<Vec<SessionResponse>>, AppError> {
    let rows = sessions::list_sessions(state.repos().sessions.as_ref(), auth.user_id).await?;

    let response: Vec<SessionResponse> = rows
        .into_iter()
        .map(|s| SessionResponse {
            id: s.id,
            mode_id: s.mode_id,
            agent_id: s.agent_id,
            draft_config: s.draft_config,
            title: s.title,
            created_at: s.created_at,
            updated_at: s.updated_at,
        })
        .collect();

    Ok(Json(response))
}

/// Get a specific session
#[utoipa::path(
    get,
    path = "/api/sessions/{session_id}",
    tag = "Sessions",
    security(("bearer_auth" = [])),
    params(("session_id" = Uuid, Path, description = "Session ID")),
    responses(
        (status = 200, description = "Session found", body = SessionResponse),
        (status = 404, description = "Not found")
    )
)]
pub async fn get_session(
    State(state): State<AppState>,
    auth: auth_utils::AuthUser,
    Path(session_id): Path<Uuid>,
) -> Result<Json<SessionResponse>, AppError> {
    let session =
        sessions::get_session(state.repos().sessions.as_ref(), auth.user_id.0, session_id).await?;

    Ok(Json(SessionResponse {
        id: session.id,
        mode_id: session.mode_id,
        agent_id: session.agent_id,
        draft_config: session.draft_config,
        title: session.title,
        created_at: session.created_at,
        updated_at: session.updated_at,
    }))
}

/// Delete a session
#[utoipa::path(
    delete,
    path = "/api/sessions/{session_id}",
    tag = "Sessions",
    security(("bearer_auth" = [])),
    params(("session_id" = Uuid, Path, description = "Session ID")),
    responses(
        (status = 204, description = "Deleted successfully"),
        (status = 404, description = "Not found")
    )
)]
pub async fn delete_session(
    State(state): State<AppState>,
    auth: auth_utils::AuthUser,
    Path(session_id): Path<Uuid>,
) -> Result<StatusCode, AppError> {
    sessions::delete_session(state.repos().sessions.as_ref(), auth.user_id.0, session_id).await?;

    state.broadcast_session(SessionEvent {
        session_id,
        user_id: Some(auth.user_id.0),
        kind: SessionEventKind::Deleted,
    });

    Ok(StatusCode::NO_CONTENT)
}

/// Update a session (rename)
#[utoipa::path(
    patch,
    path = "/api/sessions/{session_id}",
    tag = "Sessions",
    security(("bearer_auth" = [])),
    params(("session_id" = Uuid, Path, description = "Session ID")),
    request_body = UpdateSessionRequest,
    responses(
        (status = 200, description = "Session updated", body = SessionResponse),
        (status = 404, description = "Not found")
    )
)]
pub async fn update_session(
    State(state): State<AppState>,
    auth: auth_utils::AuthUser,
    Path(session_id): Path<Uuid>,
    Json(request): Json<UpdateSessionRequest>,
) -> Result<Json<SessionResponse>, AppError> {
    let updated = sessions::update_session(
        state.repos().sessions.as_ref(),
        auth.user_id.0,
        session_id,
        &request.title,
    )
    .await?;

    state.broadcast_session(SessionEvent {
        session_id: updated.id,
        user_id: Some(auth.user_id.0),
        kind: SessionEventKind::Updated {
            title: Some(updated.title.clone()),
            mode_id: Some(updated.mode_id.clone()),
        },
    });

    Ok(Json(SessionResponse {
        id: updated.id,
        mode_id: updated.mode_id,
        agent_id: updated.agent_id,
        draft_config: updated.draft_config,
        title: updated.title,
        created_at: updated.created_at,
        updated_at: updated.updated_at,
    }))
}

/// Send a message to a session
#[utoipa::path(
    post,
    path = "/api/sessions/{session_id}/chat",
    tag = "Sessions",
    security(("bearer_auth" = [])),
    params(("session_id" = Uuid, Path, description = "Session ID")),
    request_body = ChatRequest,
    responses(
        (status = 202, description = "Message queued", body = ChatResponse),
        (status = 400, description = "Empty message"),
        (status = 404, description = "Session not found")
    )
)]
pub async fn send_session_chat(
    State(state): State<AppState>,
    auth: auth_utils::AuthUser,
    Path(session_id): Path<Uuid>,
    Json(request): Json<ChatRequest>,
) -> Result<(StatusCode, Json<ChatResponse>), AppError> {
    let session = sessions::verify_session_chat(
        state.repos().sessions.as_ref(),
        auth.user_id.0,
        session_id,
        &request.message,
    )
    .await?;

    let message_id = Uuid::new_v4();

    state.ensure_response_stream(message_id);

    // Store user message scoped to session
    state
        .repos()
        .sessions
        .insert_session_message(
            auth.user_id,
            session_id,
            message_id,
            "user".to_string(),
            request.message.clone(),
        )
        .await?;

    // Queue to chat consumer with session context
    state
        .chat_tx()
        .send(ConsumerMessage {
            id: message_id,
            user_id: auth.user_id,
            session_id: Some(session_id),
            agent_id: session.agent_id,
            content: request.message,
            timestamp: Utc::now(),
        })
        .await
        .map_err(|e| AppError::Internal(format!("Failed to queue message: {e}")))?;

    Ok((
        StatusCode::ACCEPTED,
        Json(ChatResponse {
            message_id,
            status: "queued".to_string(),
        }),
    ))
}

/// Get session chat history
#[utoipa::path(
    get,
    path = "/api/sessions/{session_id}/history",
    tag = "Sessions",
    security(("bearer_auth" = [])),
    params(
        ("session_id" = Uuid, Path, description = "Session ID"),
        HistoryQuery
    ),
    responses(
        (status = 200, description = "Session chat history", body = Vec<ChatMessage>),
        (status = 404, description = "Session not found")
    )
)]
pub async fn get_session_history(
    State(state): State<AppState>,
    auth: auth_utils::AuthUser,
    Path(session_id): Path<Uuid>,
    Query(query): Query<HistoryQuery>,
) -> Result<Json<Vec<ChatMessage>>, AppError> {
    let limit = query.limit.unwrap_or(50);
    let rows = sessions::get_session_history(
        state.repos().sessions.as_ref(),
        auth.user_id.0,
        session_id,
        limit,
    )
    .await?;

    let messages: Vec<ChatMessage> = rows
        .into_iter()
        .map(|row| ChatMessage {
            id: row.id,
            role: row.role,
            content: row.content,
            timestamp: row.timestamp,
            source_type: row.source_type,
            error: row.error,
        })
        .collect();

    Ok(Json(messages))
}

/// Clear session messages
#[utoipa::path(
    delete,
    path = "/api/sessions/{session_id}/messages",
    tag = "Sessions",
    security(("bearer_auth" = [])),
    params(("session_id" = Uuid, Path, description = "Session ID")),
    responses(
        (status = 204, description = "Messages cleared"),
        (status = 404, description = "Not found")
    )
)]
pub async fn clear_session_messages(
    State(state): State<AppState>,
    auth: auth_utils::AuthUser,
    Path(session_id): Path<Uuid>,
) -> Result<StatusCode, AppError> {
    sessions::clear_session_messages(state.repos().sessions.as_ref(), auth.user_id.0, session_id)
        .await?;

    Ok(StatusCode::NO_CONTENT)
}

#[cfg(test)]
mod tests;
