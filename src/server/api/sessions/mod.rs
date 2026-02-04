//! Session and agent mode management endpoints

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    Json,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::server::auth as auth_utils;
use crate::server::state::{AppState, ConsumerMessage};
use crate::server::ws::SessionUpdate;

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
) -> Result<Json<Vec<ModeInfo>>, StatusCode> {
    let agents = state
        .repo
        .list_persisted_agents(auth.user_id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
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
// Agent Mode Endpoints
// ============================================================================

/// Response for an agent mode
#[derive(Serialize, utoipa::ToSchema)]
pub struct AgentModeResponse {
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

impl From<crate::db::AgentModeRow> for AgentModeResponse {
    fn from(r: crate::db::AgentModeRow) -> Self {
        Self {
            id: r.id,
            agent_id: r.agent_id,
            name: r.name,
            system_prompt_suffix: r.system_prompt_suffix,
            temperature_override: r.temperature_override,
            model_override: r.model_override,
            tool_overrides: r.tool_overrides,
            classifier_hint: r.classifier_hint,
            created_at: r.created_at,
            version: r.version,
        }
    }
}

/// Request body for creating an agent mode
#[derive(Deserialize, utoipa::ToSchema)]
pub struct CreateAgentModeRequest {
    pub name: String,
    #[serde(default)]
    pub system_prompt_suffix: Option<String>,
    #[serde(default)]
    pub temperature_override: Option<f64>,
    #[serde(default)]
    pub model_override: Option<String>,
    #[serde(default)]
    pub tool_overrides: Option<Vec<String>>,
    pub classifier_hint: String,
}

/// List all modes for an agent
#[utoipa::path(
    get,
    path = "/api/agents/{agent_id}/modes",
    tag = "Agent Modes",
    security(("bearer_auth" = [])),
    params(("agent_id" = Uuid, Path, description = "Agent ID")),
    responses(
        (status = 200, description = "List of agent modes", body = Vec<AgentModeResponse>)
    )
)]
pub async fn list_agent_modes(
    State(state): State<AppState>,
    _auth: auth_utils::AuthUser,
    Path(agent_id): Path<Uuid>,
) -> Result<Json<Vec<AgentModeResponse>>, StatusCode> {
    let modes = state
        .repo
        .get_agent_modes(agent_id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Json(
        modes.into_iter().map(AgentModeResponse::from).collect(),
    ))
}

/// Create a new mode for an agent
#[utoipa::path(
    post,
    path = "/api/agents/{agent_id}/modes",
    tag = "Agent Modes",
    security(("bearer_auth" = [])),
    params(("agent_id" = Uuid, Path, description = "Agent ID")),
    request_body = CreateAgentModeRequest,
    responses(
        (status = 201, description = "Mode created", body = AgentModeResponse)
    )
)]
pub async fn create_agent_mode(
    State(state): State<AppState>,
    _auth: auth_utils::AuthUser,
    Path(agent_id): Path<Uuid>,
    Json(req): Json<CreateAgentModeRequest>,
) -> Result<(StatusCode, Json<AgentModeResponse>), StatusCode> {
    let mode = crate::db::AgentModeRow {
        id: Uuid::new_v4(),
        agent_id,
        name: req.name,
        system_prompt_suffix: req.system_prompt_suffix,
        temperature_override: req.temperature_override,
        model_override: req.model_override,
        tool_overrides: req.tool_overrides,
        classifier_hint: req.classifier_hint,
        created_at: Utc::now(),
        version: 1,
    };

    state
        .repo
        .create_agent_mode(&mode)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok((StatusCode::CREATED, Json(AgentModeResponse::from(mode))))
}

/// Delete an agent mode
#[utoipa::path(
    delete,
    path = "/api/agent-modes/{mode_id}",
    tag = "Agent Modes",
    security(("bearer_auth" = [])),
    params(("mode_id" = Uuid, Path, description = "Mode ID")),
    responses(
        (status = 204, description = "Mode deleted")
    )
)]
pub async fn delete_agent_mode(
    State(state): State<AppState>,
    _auth: auth_utils::AuthUser,
    Path(mode_id): Path<Uuid>,
) -> Result<StatusCode, StatusCode> {
    state
        .repo
        .delete_agent_mode(mode_id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(StatusCode::NO_CONTENT)
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
) -> Result<(StatusCode, Json<SessionResponse>), StatusCode> {
    // Validate agent exists if provided
    if let Some(aid) = request.agent_id {
        if state
            .repo
            .get_persisted_agent(aid)
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
            .is_none()
        {
            return Err(StatusCode::BAD_REQUEST);
        }
    }

    let session_id = Uuid::new_v4();
    let mode_id = if request.mode_id.is_empty() {
        "home".to_string()
    } else {
        request.mode_id
    };
    let title = if request.title.is_empty() {
        "New session".to_string()
    } else {
        request.title
    };

    state
        .repo
        .create_session(auth.user_id, session_id, &mode_id, &title, request.agent_id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let session = state
        .repo
        .get_session(session_id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::INTERNAL_SERVER_ERROR)?;

    state.broadcast_session(SessionUpdate {
        id: session.id,
        action: "created".to_string(),
        title: Some(session.title.clone()),
        mode_id: Some(session.mode_id.clone()),
        user_id: Some(auth.user_id.0),
    });

    Ok((
        StatusCode::CREATED,
        Json(SessionResponse {
            id: session.id,
            mode_id: session.mode_id,
            agent_id: session.agent_id,
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
) -> Result<Json<Vec<SessionResponse>>, StatusCode> {
    let sessions = state
        .repo
        .list_sessions(auth.user_id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let response: Vec<SessionResponse> = sessions
        .into_iter()
        .map(|s| SessionResponse {
            id: s.id,
            mode_id: s.mode_id,
            agent_id: s.agent_id,
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
) -> Result<Json<SessionResponse>, StatusCode> {
    let session = state
        .repo
        .get_session(session_id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::NOT_FOUND)?;

    // Verify ownership
    if session.user_id != auth.user_id.0 {
        return Err(StatusCode::NOT_FOUND);
    }

    Ok(Json(SessionResponse {
        id: session.id,
        mode_id: session.mode_id,
        agent_id: session.agent_id,
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
) -> Result<StatusCode, StatusCode> {
    // Verify ownership
    let session = state
        .repo
        .get_session(session_id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::NOT_FOUND)?;

    if session.user_id != auth.user_id.0 {
        return Err(StatusCode::NOT_FOUND);
    }

    state
        .repo
        .delete_session(session_id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    state.broadcast_session(SessionUpdate {
        id: session_id,
        action: "deleted".to_string(),
        title: None,
        mode_id: None,
        user_id: Some(auth.user_id.0),
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
) -> Result<Json<SessionResponse>, StatusCode> {
    let session = state
        .repo
        .get_session(session_id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::NOT_FOUND)?;

    if session.user_id != auth.user_id.0 {
        return Err(StatusCode::NOT_FOUND);
    }

    state
        .repo
        .update_session_title(session_id, &request.title)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let updated = state
        .repo
        .get_session(session_id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::INTERNAL_SERVER_ERROR)?;

    state.broadcast_session(SessionUpdate {
        id: updated.id,
        action: "updated".to_string(),
        title: Some(updated.title.clone()),
        mode_id: Some(updated.mode_id.clone()),
        user_id: Some(auth.user_id.0),
    });

    Ok(Json(SessionResponse {
        id: updated.id,
        mode_id: updated.mode_id,
        agent_id: updated.agent_id,
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
) -> Result<(StatusCode, Json<ChatResponse>), StatusCode> {
    if request.message.trim().is_empty() {
        return Err(StatusCode::BAD_REQUEST);
    }

    // Verify session exists and belongs to user
    let session = state
        .repo
        .get_session(session_id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::NOT_FOUND)?;

    if session.user_id != auth.user_id.0 {
        return Err(StatusCode::NOT_FOUND);
    }

    let message_id = Uuid::new_v4();

    state.ensure_response_stream(message_id).await;

    // Store user message scoped to session
    state
        .repo
        .insert_session_message(
            auth.user_id,
            session_id,
            message_id,
            "user".to_string(),
            request.message.clone(),
        )
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    // Queue to chat consumer with session context
    state
        .chat_tx
        .send(ConsumerMessage {
            id: message_id,
            user_id: auth.user_id,
            session_id: Some(session_id),
            agent_id: session.agent_id,
            content: request.message,
            timestamp: Utc::now(),
        })
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

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
) -> Result<Json<Vec<ChatMessage>>, StatusCode> {
    // Verify session ownership
    let session = state
        .repo
        .get_session(session_id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::NOT_FOUND)?;

    if session.user_id != auth.user_id.0 {
        return Err(StatusCode::NOT_FOUND);
    }

    let limit = query.limit.unwrap_or(50);
    let rows = state
        .repo
        .get_session_history(session_id, limit)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let messages: Vec<ChatMessage> = rows
        .into_iter()
        .map(|row| ChatMessage {
            id: row.id,
            role: row.role,
            content: row.content,
            timestamp: row.timestamp,
        })
        .collect();

    Ok(Json(messages))
}
#[cfg(test)]
mod tests;
