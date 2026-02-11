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
    let agents = state.repo().list_persisted_agents(auth.user_id).await?;
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
    auth: auth_utils::AuthUser,
    Path(agent_id): Path<Uuid>,
) -> Result<Json<Vec<AgentModeResponse>>, AppError> {
    super::ownership::verify_agent_ownership(state.repo().as_ref(), &auth, agent_id).await?;

    let modes = state.repo().get_agent_modes(agent_id).await?;
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
    auth: auth_utils::AuthUser,
    Path(agent_id): Path<Uuid>,
    Json(req): Json<CreateAgentModeRequest>,
) -> Result<(StatusCode, Json<AgentModeResponse>), AppError> {
    super::ownership::verify_agent_ownership(state.repo().as_ref(), &auth, agent_id).await?;

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

    state.repo().create_agent_mode(&mode).await?;

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
    auth: auth_utils::AuthUser,
    Path(mode_id): Path<Uuid>,
) -> Result<StatusCode, AppError> {
    let mode = state
        .repo()
        .get_agent_mode(mode_id)
        .await?
        .ok_or(AppError::not_found("Agent mode"))?;

    super::ownership::verify_agent_ownership(state.repo().as_ref(), &auth, mode.agent_id).await?;

    state.repo().delete_agent_mode(mode_id).await?;
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
    #[serde(default)]
    pub draft_config: Option<serde_json::Value>,
}

/// Request body for updating a session
#[derive(Deserialize, utoipa::ToSchema)]
pub struct UpdateSessionRequest {
    pub title: String,
}

/// Request body for updating session draft config
#[derive(Deserialize, utoipa::ToSchema)]
pub struct UpdateDraftConfigRequest {
    pub draft_config: serde_json::Value,
}

/// Request body for saving an agent from draft config
#[derive(Deserialize, utoipa::ToSchema)]
pub struct SaveAgentRequest {
    pub name: String,
    #[serde(default)]
    pub context_document_ids: Vec<Uuid>,
}

/// Response for saving an agent from draft
#[derive(Serialize, utoipa::ToSchema)]
pub struct SaveAgentResponse {
    pub agent_id: Uuid,
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
    // Validate agent exists if provided
    if let Some(aid) = request.agent_id {
        if state.repo().get_persisted_agent(aid).await?.is_none() {
            return Err(AppError::bad_request("Agent not found"));
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
        .repo()
        .create_session(
            auth.user_id,
            session_id,
            &mode_id,
            &title,
            request.agent_id,
            request.draft_config,
        )
        .await?;

    let session = state
        .repo()
        .get_session(session_id)
        .await?
        .ok_or(AppError::Internal(
            "Session not found after creation".into(),
        ))?;

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
    let sessions = state.repo().list_sessions(auth.user_id).await?;

    let response: Vec<SessionResponse> = sessions
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
    let session = state
        .repo()
        .get_session(session_id)
        .await?
        .ok_or(AppError::not_found("Session"))?;

    // Verify ownership
    if session.user_id != auth.user_id.0 {
        return Err(AppError::not_found("Session"));
    }

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
    // Verify ownership
    let session = state
        .repo()
        .get_session(session_id)
        .await?
        .ok_or(AppError::not_found("Session"))?;

    if session.user_id != auth.user_id.0 {
        return Err(AppError::not_found("Session"));
    }

    state.repo().delete_session(session_id).await?;

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
    let session = state
        .repo()
        .get_session(session_id)
        .await?
        .ok_or(AppError::not_found("Session"))?;

    if session.user_id != auth.user_id.0 {
        return Err(AppError::not_found("Session"));
    }

    state
        .repo()
        .update_session_title(session_id, &request.title)
        .await?;

    let updated = state
        .repo()
        .get_session(session_id)
        .await?
        .ok_or(AppError::Internal("Session not found after update".into()))?;

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
    if request.message.trim().is_empty() {
        return Err(AppError::bad_request("Message cannot be empty"));
    }

    // Verify session exists and belongs to user
    let session = state
        .repo()
        .get_session(session_id)
        .await?
        .ok_or(AppError::not_found("Session"))?;

    if session.user_id != auth.user_id.0 {
        return Err(AppError::not_found("Session"));
    }

    let message_id = Uuid::new_v4();

    state.ensure_response_stream(message_id);

    // Store user message scoped to session
    state
        .repo()
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
    // Verify session ownership
    let session = state
        .repo()
        .get_session(session_id)
        .await?
        .ok_or(AppError::not_found("Session"))?;

    if session.user_id != auth.user_id.0 {
        return Err(AppError::not_found("Session"));
    }

    let limit = query.limit.unwrap_or(50);
    let rows = state.repo().get_session_history(session_id, limit).await?;

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

/// Update session draft config
#[utoipa::path(
    patch,
    path = "/api/sessions/{session_id}/config",
    tag = "Sessions",
    security(("bearer_auth" = [])),
    params(("session_id" = Uuid, Path, description = "Session ID")),
    request_body = UpdateDraftConfigRequest,
    responses(
        (status = 200, description = "Config updated", body = SessionResponse),
        (status = 404, description = "Not found")
    )
)]
pub async fn update_session_config(
    State(state): State<AppState>,
    auth: auth_utils::AuthUser,
    Path(session_id): Path<Uuid>,
    Json(request): Json<UpdateDraftConfigRequest>,
) -> Result<Json<SessionResponse>, AppError> {
    // Verify session ownership
    let session = state
        .repo()
        .get_session(session_id)
        .await?
        .ok_or(AppError::not_found("Session"))?;

    if session.user_id != auth.user_id.0 {
        return Err(AppError::not_found("Session"));
    }

    state
        .repo()
        .update_session_draft_config(session_id, Some(request.draft_config))
        .await?;

    let updated = state
        .repo()
        .get_session(session_id)
        .await?
        .ok_or(AppError::Internal("Session not found after update".into()))?;

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
    // Verify session ownership
    let session = state
        .repo()
        .get_session(session_id)
        .await?
        .ok_or(AppError::not_found("Session"))?;

    if session.user_id != auth.user_id.0 {
        return Err(AppError::not_found("Session"));
    }

    state.repo().clear_session_messages(session_id).await?;

    Ok(StatusCode::NO_CONTENT)
}

/// Save agent from session draft config
#[utoipa::path(
    post,
    path = "/api/sessions/{session_id}/save-agent",
    tag = "Sessions",
    security(("bearer_auth" = [])),
    params(("session_id" = Uuid, Path, description = "Session ID")),
    request_body = SaveAgentRequest,
    responses(
        (status = 201, description = "Agent created and linked", body = SaveAgentResponse),
        (status = 400, description = "Session has no draft config"),
        (status = 404, description = "Not found")
    )
)]
pub async fn save_session_agent(
    State(state): State<AppState>,
    auth: auth_utils::AuthUser,
    Path(session_id): Path<Uuid>,
    Json(request): Json<SaveAgentRequest>,
) -> Result<(StatusCode, Json<SaveAgentResponse>), AppError> {
    // Verify session ownership
    let session = state
        .repo()
        .get_session(session_id)
        .await?
        .ok_or(AppError::not_found("Session"))?;

    if session.user_id != auth.user_id.0 {
        return Err(AppError::not_found("Session"));
    }

    // Parse draft_config
    let draft_config: crate::server::hub::DraftConfig = session
        .draft_config
        .ok_or(AppError::bad_request("Session has no draft config"))?
        .try_into()
        .map_err(|e: serde_json::Error| {
            AppError::bad_request(format!("Invalid draft config: {e}"))
        })?;

    // Create agent from draft config
    let agent_id = Uuid::new_v4();
    let agent = crate::db::AgentRow {
        id: agent_id,
        user_id: Some(auth.user_id.0),
        tier: None,
        name: request.name,
        system_prompt: draft_config.system_prompt,
        persona_style: None,
        model_provider: "anthropic".to_string(),
        model_id: draft_config.model_id,
        model_max_tokens: draft_config.model_max_tokens,
        model_temperature: draft_config.model_temperature,
        status: None,
        router_mode: None,
        router_id: None,
        output_schema_id: None,
        version: 1,
        default_reasoning_trace: None,
        is_system: false,
    };

    state.repo().upsert_agent(agent).await?;

    // Set context documents if provided
    if !request.context_document_ids.is_empty() {
        state
            .repo()
            .set_agent_context(agent_id, request.context_document_ids)
            .await?;
    }

    // Link agent to session (clears draft_config)
    state
        .repo()
        .link_session_agent(session_id, agent_id)
        .await?;

    Ok((StatusCode::CREATED, Json(SaveAgentResponse { agent_id })))
}

#[cfg(test)]
mod tests;
