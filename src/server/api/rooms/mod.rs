//! Multi-agent room orchestration endpoints

use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use serde::Deserialize;
use uuid::Uuid;

use serde::Serialize;

use super::AppError;
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
) -> Result<(StatusCode, Json<crate::db::RoomRow>), AppError> {
    if request.name.trim().is_empty() || request.name.len() > MAX_TITLE_LENGTH {
        return Err(AppError::bad_request(
            "Room name is empty or exceeds maximum length",
        ));
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
        .await?;
    Ok((StatusCode::CREATED, Json(row)))
}

/// GET /api/rooms/:id - Get a room.
pub async fn get_room(
    State(state): State<AppState>,
    auth: auth_utils::AuthUser,
    Path(id): Path<Uuid>,
) -> Result<Json<crate::db::RoomRow>, AppError> {
    let repo = &state.repos().rooms;
    let row = repo
        .get_room(id)
        .await?
        .ok_or(AppError::not_found("Room"))?;
    if row.user_id != auth.user_id.0 {
        return Err(AppError::not_found("Room"));
    }
    Ok(Json(row))
}

/// PUT /api/rooms/:id - Update a room.
pub async fn update_room(
    State(state): State<AppState>,
    auth: auth_utils::AuthUser,
    Path(id): Path<Uuid>,
    Json(request): Json<UpdateRoomRequest>,
) -> Result<Json<crate::db::RoomRow>, AppError> {
    let repo = &state.repos().rooms;
    let existing = repo
        .get_room(id)
        .await?
        .ok_or(AppError::not_found("Room"))?;
    if existing.user_id != auth.user_id.0 {
        return Err(AppError::not_found("Room"));
    }
    if let Some(ref name) = request.name {
        if name.trim().is_empty() || name.len() > MAX_TITLE_LENGTH {
            return Err(AppError::bad_request(
                "Room name is empty or exceeds maximum length",
            ));
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
        .await?;
    Ok(Json(row))
}

/// DELETE /api/rooms/:id - Delete a room.
pub async fn delete_room(
    State(state): State<AppState>,
    auth: auth_utils::AuthUser,
    Path(id): Path<Uuid>,
) -> Result<StatusCode, AppError> {
    let repo = &state.repos().rooms;
    let existing = repo
        .get_room(id)
        .await?
        .ok_or(AppError::not_found("Room"))?;
    if existing.user_id != auth.user_id.0 {
        return Err(AppError::not_found("Room"));
    }
    repo.delete_room(id).await?;
    Ok(StatusCode::NO_CONTENT)
}

/// GET /api/rooms/:id/members - List room members.
pub async fn list_room_members(
    State(state): State<AppState>,
    _auth: auth_utils::AuthUser,
    Path(room_id): Path<Uuid>,
) -> Result<Json<Vec<crate::db::RoomMemberRow>>, AppError> {
    let repo = &state.repos().rooms;
    let rows = repo.list_room_members(room_id).await?;
    Ok(Json(rows))
}

/// POST /api/rooms/:id/members - Add a room member.
pub async fn add_room_member(
    State(state): State<AppState>,
    _auth: auth_utils::AuthUser,
    Path(room_id): Path<Uuid>,
    Json(request): Json<AddRoomMemberRequest>,
) -> Result<StatusCode, AppError> {
    let repo = &state.repos().rooms;
    repo.add_room_member(
        room_id,
        request.agent_id,
        request.display_name,
        request.role_description,
        request.display_order,
    )
    .await?;
    Ok(StatusCode::CREATED)
}

/// DELETE /api/rooms/:id/members/:agent_id - Remove a room member.
pub async fn remove_room_member(
    State(state): State<AppState>,
    _auth: auth_utils::AuthUser,
    Path((room_id, agent_id)): Path<(Uuid, Uuid)>,
) -> Result<StatusCode, AppError> {
    let repo = &state.repos().rooms;
    repo.remove_room_member(room_id, agent_id).await?;
    Ok(StatusCode::NO_CONTENT)
}

/// PUT /api/rooms/:id/members - Set all room members (replace).
pub async fn set_room_members(
    State(state): State<AppState>,
    _auth: auth_utils::AuthUser,
    Path(room_id): Path<Uuid>,
    Json(request): Json<SetRoomMembersRequest>,
) -> Result<StatusCode, AppError> {
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
    repo.set_room_members(room_id, &members).await?;
    Ok(StatusCode::OK)
}

/// POST /api/rooms/:id/sessions - Start a room session.
pub async fn create_room_session(
    State(state): State<AppState>,
    _auth: auth_utils::AuthUser,
    Path(room_id): Path<Uuid>,
) -> Result<(StatusCode, Json<crate::db::RoomSessionRow>), AppError> {
    let repo = &state.repos().rooms;
    let row = repo.create_room_session(room_id).await?;
    Ok((StatusCode::CREATED, Json(row)))
}

/// GET /api/room-sessions/:id - Get room session.
pub async fn get_room_session(
    State(state): State<AppState>,
    _auth: auth_utils::AuthUser,
    Path(id): Path<Uuid>,
) -> Result<Json<crate::db::RoomSessionRow>, AppError> {
    let repo = &state.repos().rooms;
    let row = repo
        .get_room_session(id)
        .await?
        .ok_or(AppError::not_found("RoomSession"))?;
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
) -> Result<Json<serde_json::Value>, AppError> {
    if request.content.trim().is_empty() {
        return Err(AppError::bad_request("Message content cannot be empty"));
    }

    let room_repo = &state.repos().rooms;

    // Load session
    let session = room_repo
        .get_room_session(session_id)
        .await?
        .ok_or(AppError::not_found("RoomSession"))?;

    if session.status != "active" {
        return Err(AppError::Conflict("Room session is not active".to_string()));
    }

    // Load room
    let room = room_repo
        .get_room(session.room_id)
        .await?
        .ok_or(AppError::not_found("Room"))?;

    // Load members + agents
    let member_rows = room_repo.list_room_members(room.id).await?;

    let mut members = Vec::new();
    for m in member_rows {
        let agent =
            state
                .repo()
                .get_persisted_agent(m.agent_id)
                .await?
                .ok_or(AppError::Internal(format!(
                    "Agent {} not found for room member",
                    m.agent_id
                )))?;
        members.push(crate::server::executors::room::RoomMemberWithAgent { member: m, agent });
    }

    // Create LLM provider
    let provider: std::sync::Arc<dyn crate::llm::LLMProvider + Send + Sync> =
        match crate::llm::AnthropicClient::from_env() {
            Ok(p) => std::sync::Arc::new(p),
            Err(e) => {
                return Err(AppError::ServiceUnavailable(format!(
                    "LLM provider unavailable: {e}"
                )))
            }
        };

    // Spawn background task to execute the turn
    let room_clone = room.clone();
    let session_clone = session.clone();
    let state_clone = state.clone();
    let user_message = request.content.clone();
    let user_id = auth.user_id.0;
    tokio::spawn(async move {
        if let Err(e) = crate::server::executors::room::execute_room_turn(
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
) -> Result<Json<Vec<crate::db::RoomTranscriptEntry>>, AppError> {
    let repo = &state.repos().rooms;
    let entries = repo.get_room_transcript(session_id).await?;
    Ok(Json(entries))
}

/// POST /api/room-sessions/:id/close - Close a room session.
///
/// If the session is linked to a DAG workflow (via step→room), closing it
/// triggers DAG resume: per-agent outputs are extracted from the room transcript
/// and the workflow continues from the paused room step.
pub async fn close_room_session(
    State(state): State<AppState>,
    _auth: auth_utils::AuthUser,
    Path(session_id): Path<Uuid>,
) -> Result<StatusCode, AppError> {
    let repo = &state.repos().rooms;
    let session = repo
        .get_room_session(session_id)
        .await?
        .ok_or(AppError::not_found("RoomSession"))?;

    if session.status == "completed" {
        return Err(AppError::Conflict(
            "Room session is already completed".to_string(),
        ));
    }
    repo.update_room_session_status(session_id, "completed")
        .await?;

    state.broadcast_room(crate::server::ws::events::RoomEvent {
        room_session_id: session_id,
        run_id: None,
        user_id: None,
        kind: crate::server::ws::events::RoomEventKind::SessionComplete {
            turn_number: session.current_turn,
        },
    });

    // Check if this session is DAG-linked via step→room relationship
    {
        if let Some(wf_repo) = state.workflow_repo() {
            if let Ok(Some(step)) = wf_repo.find_step_by_room_id(session.room_id).await {
                // Extract per-agent outputs from the room transcript
                let transcript = repo
                    .get_room_transcript(session_id)
                    .await
                    .unwrap_or_default();

                let room_output = build_room_step_output(&transcript, &step);

                // Resume the DAG in the background
                let state_clone = state.clone();
                let step_id = step.id;
                tokio::spawn(async move {
                    if let Err(e) = crate::server::hub::dag::resume_dag_from_approval(
                        &state_clone,
                        step_id,
                        room_output,
                    )
                    .await
                    {
                        tracing::warn!(
                            step_id = %step_id,
                            session_id = %session_id,
                            "Failed to resume DAG after room close: {}",
                            e
                        );
                    }
                });
            }
        }
    }

    Ok(StatusCode::NO_CONTENT)
}

/// Build a StepOutput from a room transcript for DAG resume.
///
/// Groups transcript entries by agent, takes each agent's last message,
/// and builds a composite JSON object.
fn build_room_step_output(
    transcript: &[crate::db::RoomTranscriptEntry],
    step: &crate::db::WorkflowStepRow,
) -> crate::server::hub::dag::StepOutput {
    use std::collections::HashMap;

    let mut last_by_agent: HashMap<String, String> = HashMap::new();
    for entry in transcript {
        last_by_agent.insert(entry.agent_name.clone(), entry.content.clone());
    }

    let mut composite = serde_json::Map::new();
    for (agent_name, content) in &last_by_agent {
        let key = agent_name.to_lowercase().replace(' ', "_");
        let value: serde_json::Value = serde_json::from_str(content)
            .unwrap_or_else(|_| serde_json::Value::String(content.clone()));
        composite.insert(key, value);
    }
    let envelope_data = serde_json::Value::Object(composite);

    crate::server::hub::dag::StepOutput {
        variable_name: step.output_variable_name.clone().unwrap_or_default(),
        raw_output: serde_json::to_string(&envelope_data).unwrap_or_default(),
        structured_output: Some(envelope_data),
    }
}

/// Structured output from a room session member.
#[derive(Serialize, utoipa::ToSchema)]
pub struct RoomOutputResponse {
    pub id: Uuid,
    pub agent_id: Uuid,
    pub speaker_order: i32,
    pub turn_number: i32,
    pub output_name: String,
    pub structured_output: serde_json::Value,
    pub raw_output: String,
}

/// GET /api/room-sessions/:id/outputs - List structured outputs for a room session.
#[utoipa::path(
    get,
    path = "/api/room-sessions/{id}/outputs",
    tag = "Room Outputs",
    security(("bearer_auth" = [])),
    params(("id" = Uuid, Path, description = "Room session ID")),
    responses(
        (status = 200, description = "List of structured outputs", body = Vec<RoomOutputResponse>),
        (status = 404, description = "Session not found")
    )
)]
pub async fn list_room_outputs(
    State(state): State<AppState>,
    _auth: auth_utils::AuthUser,
    Path(session_id): Path<Uuid>,
) -> Result<Json<Vec<RoomOutputResponse>>, AppError> {
    let repo = &state.repos().rooms;
    // Verify session exists
    repo.get_room_session(session_id)
        .await?
        .ok_or(AppError::not_found("RoomSession"))?;
    let rows = repo.get_room_execution_outputs(session_id, None).await?;
    Ok(Json(
        rows.into_iter()
            .map(|r| RoomOutputResponse {
                id: r.id,
                agent_id: r.agent_id,
                speaker_order: r.speaker_order,
                turn_number: r.turn_number,
                output_name: r.output_name,
                structured_output: r.structured_output,
                raw_output: r.raw_output,
            })
            .collect(),
    ))
}

#[cfg(test)]
mod tests;
