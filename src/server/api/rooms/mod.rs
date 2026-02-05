//! Multi-agent room orchestration endpoints

use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use chrono::Utc;
use serde::Deserialize;
use uuid::Uuid;

use crate::constants::MAX_TITLE_LENGTH;
use crate::server::auth as auth_utils;
use crate::server::state::AppState;

/// Request body for creating a room.
#[derive(Deserialize, utoipa::ToSchema)]
pub struct CreateRoomRequest {
    pub collection_id: Option<Uuid>,
    pub name: String,
    #[serde(default)]
    pub gatekeeper_enabled: bool,
    #[serde(default = "default_gatekeeper_model")]
    pub gatekeeper_model_id: String,
    #[serde(default = "default_max_speakers")]
    pub max_speakers_per_turn: i32,
    #[serde(default = "default_max_turns")]
    pub max_turns: i32,
    #[serde(default)]
    pub tools_enabled: bool,
}

fn default_gatekeeper_model() -> String {
    "claude-haiku-4-20250414".to_string()
}
fn default_max_speakers() -> i32 {
    4
}
fn default_max_turns() -> i32 {
    20
}

/// Request body for updating a room.
#[derive(Deserialize, utoipa::ToSchema)]
pub struct UpdateRoomRequest {
    pub name: Option<String>,
    pub gatekeeper_enabled: Option<bool>,
    pub gatekeeper_model_id: Option<String>,
    pub max_speakers_per_turn: Option<i32>,
    pub max_turns: Option<i32>,
    pub tools_enabled: Option<bool>,
}

/// Request body for adding a room member.
#[derive(Deserialize, utoipa::ToSchema)]
pub struct AddRoomMemberRequest {
    pub agent_id: Uuid,
    pub display_name: Option<String>,
    pub role_description: String,
    #[serde(default)]
    pub display_order: i32,
}

/// Request body for setting all room members at once.
#[derive(Deserialize, utoipa::ToSchema)]
pub struct SetRoomMembersRequest {
    pub members: Vec<AddRoomMemberRequest>,
}

/// Request body for sending a message to a room session.
#[derive(Deserialize, utoipa::ToSchema)]
pub struct RoomMessageRequest {
    pub content: String,
}

/// POST /api/rooms - Create a room.
pub async fn create_room(
    State(state): State<AppState>,
    auth: auth_utils::AuthUser,
    Json(request): Json<CreateRoomRequest>,
) -> Result<(StatusCode, Json<crate::db::RoomRow>), StatusCode> {
    if request.name.trim().is_empty() || request.name.len() > MAX_TITLE_LENGTH {
        return Err(StatusCode::BAD_REQUEST);
    }
    let repo = &state.repos().rooms;
    let row = repo
        .create_room(
            auth.user_id.0,
            request.collection_id,
            &request.name,
            request.gatekeeper_enabled,
            &request.gatekeeper_model_id,
            request.max_speakers_per_turn,
            request.max_turns,
            request.tools_enabled,
        )
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok((StatusCode::CREATED, Json(row)))
}

/// GET /api/rooms/:id - Get a room.
pub async fn get_room(
    State(state): State<AppState>,
    auth: auth_utils::AuthUser,
    Path(id): Path<Uuid>,
) -> Result<Json<crate::db::RoomRow>, StatusCode> {
    let repo = &state.repos().rooms;
    let row = repo
        .get_room(id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::NOT_FOUND)?;
    if row.user_id != auth.user_id.0 {
        return Err(StatusCode::NOT_FOUND);
    }
    Ok(Json(row))
}

/// PUT /api/rooms/:id - Update a room.
pub async fn update_room(
    State(state): State<AppState>,
    auth: auth_utils::AuthUser,
    Path(id): Path<Uuid>,
    Json(request): Json<UpdateRoomRequest>,
) -> Result<Json<crate::db::RoomRow>, StatusCode> {
    let repo = &state.repos().rooms;
    let existing = repo
        .get_room(id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::NOT_FOUND)?;
    if existing.user_id != auth.user_id.0 {
        return Err(StatusCode::NOT_FOUND);
    }
    if let Some(ref name) = request.name {
        if name.trim().is_empty() || name.len() > MAX_TITLE_LENGTH {
            return Err(StatusCode::BAD_REQUEST);
        }
    }
    let row = repo
        .update_room(
            id,
            request.name,
            request.gatekeeper_enabled,
            request.gatekeeper_model_id,
            request.max_speakers_per_turn,
            request.max_turns,
            request.tools_enabled,
        )
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Json(row))
}

/// DELETE /api/rooms/:id - Delete a room.
pub async fn delete_room(
    State(state): State<AppState>,
    auth: auth_utils::AuthUser,
    Path(id): Path<Uuid>,
) -> Result<StatusCode, StatusCode> {
    let repo = &state.repos().rooms;
    let existing = repo
        .get_room(id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::NOT_FOUND)?;
    if existing.user_id != auth.user_id.0 {
        return Err(StatusCode::NOT_FOUND);
    }
    repo.delete_room(id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(StatusCode::NO_CONTENT)
}

/// GET /api/rooms/:id/members - List room members.
pub async fn list_room_members(
    State(state): State<AppState>,
    _auth: auth_utils::AuthUser,
    Path(room_id): Path<Uuid>,
) -> Result<Json<Vec<crate::db::RoomMemberRow>>, StatusCode> {
    let repo = &state.repos().rooms;
    let rows = repo
        .list_room_members(room_id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Json(rows))
}

/// POST /api/rooms/:id/members - Add a room member.
pub async fn add_room_member(
    State(state): State<AppState>,
    _auth: auth_utils::AuthUser,
    Path(room_id): Path<Uuid>,
    Json(request): Json<AddRoomMemberRequest>,
) -> Result<StatusCode, StatusCode> {
    let repo = &state.repos().rooms;
    repo.add_room_member(
        room_id,
        request.agent_id,
        request.display_name,
        request.role_description,
        request.display_order,
    )
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(StatusCode::CREATED)
}

/// DELETE /api/rooms/:id/members/:agent_id - Remove a room member.
pub async fn remove_room_member(
    State(state): State<AppState>,
    _auth: auth_utils::AuthUser,
    Path((room_id, agent_id)): Path<(Uuid, Uuid)>,
) -> Result<StatusCode, StatusCode> {
    let repo = &state.repos().rooms;
    repo.remove_room_member(room_id, agent_id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(StatusCode::NO_CONTENT)
}

/// PUT /api/rooms/:id/members - Set all room members (replace).
pub async fn set_room_members(
    State(state): State<AppState>,
    _auth: auth_utils::AuthUser,
    Path(room_id): Path<Uuid>,
    Json(request): Json<SetRoomMembersRequest>,
) -> Result<StatusCode, StatusCode> {
    let repo = &state.repos().rooms;
    let members: Vec<crate::db::traits::RoomMemberInput> = request
        .members
        .into_iter()
        .map(|m| crate::db::traits::RoomMemberInput {
            agent_id: m.agent_id,
            display_name: m.display_name,
            role_description: m.role_description,
            display_order: m.display_order,
        })
        .collect();
    repo.set_room_members(room_id, &members)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(StatusCode::OK)
}

/// POST /api/rooms/:id/sessions - Start a room session.
pub async fn create_room_session(
    State(state): State<AppState>,
    _auth: auth_utils::AuthUser,
    Path(room_id): Path<Uuid>,
) -> Result<(StatusCode, Json<crate::db::RoomSessionRow>), StatusCode> {
    let repo = &state.repos().rooms;
    let row = repo
        .create_room_session(room_id, None)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok((StatusCode::CREATED, Json(row)))
}

/// GET /api/room-sessions/:id - Get room session.
pub async fn get_room_session(
    State(state): State<AppState>,
    _auth: auth_utils::AuthUser,
    Path(id): Path<Uuid>,
) -> Result<Json<crate::db::RoomSessionRow>, StatusCode> {
    let repo = &state.repos().rooms;
    let row = repo
        .get_room_session(id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::NOT_FOUND)?;
    Ok(Json(row))
}

/// POST /api/room-sessions/:id/messages - Send a message to a room session.
///
/// Triggers a full room turn: gatekeeper (if enabled) selects speakers,
/// each speaker executes via the engine, responses stream via WebSocket.
/// Returns immediately with turn status.
pub async fn send_room_message(
    State(state): State<AppState>,
    auth: auth_utils::AuthUser,
    Path(session_id): Path<Uuid>,
    Json(request): Json<RoomMessageRequest>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    if request.content.trim().is_empty() {
        return Err(StatusCode::BAD_REQUEST);
    }

    let room_repo = &state.repos().rooms;

    // Load session
    let session = room_repo
        .get_room_session(session_id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::NOT_FOUND)?;

    if session.status != "active" {
        return Err(StatusCode::CONFLICT);
    }

    // Load room
    let room = room_repo
        .get_room(session.room_id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::NOT_FOUND)?;

    // Load members + agents
    let member_rows = room_repo
        .list_room_members(room.id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let mut members = Vec::new();
    for m in member_rows {
        let agent = state
            .repo()
            .get_persisted_agent(m.agent_id)
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
            .ok_or(StatusCode::INTERNAL_SERVER_ERROR)?;
        members.push(crate::server::room_executor::RoomMemberWithAgent { member: m, agent });
    }

    // Create LLM provider
    let provider: std::sync::Arc<dyn crate::llm::LLMProvider + Send + Sync> =
        match crate::llm::AnthropicClient::from_env() {
            Ok(p) => std::sync::Arc::new(p),
            Err(_) => return Err(StatusCode::SERVICE_UNAVAILABLE),
        };

    // Spawn background task to execute the turn
    let room_clone = room.clone();
    let session_clone = session.clone();
    let state_clone = state.clone();
    let user_message = request.content.clone();
    let user_id = auth.user_id.0;
    tokio::spawn(async move {
        if let Err(e) = crate::server::room_executor::execute_room_turn(
            &state_clone,
            provider,
            &room_clone,
            &session_clone,
            &members,
            &user_message,
            user_id,
            None,
        )
        .await
        {
            eprintln!("Room turn execution error: {}", e);
        }
    });

    Ok(Json(serde_json::json!({
        "status": "processing",
        "session_id": session_id,
    })))
}

/// GET /api/room-sessions/:id/transcript - Get room transcript.
pub async fn get_room_transcript(
    State(state): State<AppState>,
    _auth: auth_utils::AuthUser,
    Path(session_id): Path<Uuid>,
) -> Result<Json<Vec<crate::db::RoomTranscriptEntry>>, StatusCode> {
    let repo = &state.repos().rooms;
    let entries = repo
        .get_room_transcript(session_id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Json(entries))
}

/// POST /api/room-sessions/:id/close - Close a room session.
pub async fn close_room_session(
    State(state): State<AppState>,
    _auth: auth_utils::AuthUser,
    Path(session_id): Path<Uuid>,
) -> Result<StatusCode, StatusCode> {
    let repo = &state.repos().rooms;
    let session = repo
        .get_room_session(session_id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::NOT_FOUND)?;

    if session.status == "completed" {
        return Err(StatusCode::CONFLICT);
    }
    repo.update_room_session_status(session_id, "completed")
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    state.broadcast_room_update(crate::server::ws::RoomUpdateEvent {
        room_session_id: session_id,
        run_id: None,
        event: "session_complete".into(),
        agent_id: None,
        agent_name: None,
        content: None,
        speaker_order: None,
        turn_number: Some(session.current_turn),
        timestamp: Utc::now(),
        user_id: None,
    });

    Ok(StatusCode::NO_CONTENT)
}

#[cfg(test)]
mod tests;
