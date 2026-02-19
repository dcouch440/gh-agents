//! Room service: create, read, update, delete rooms, manage members and sessions.

use std::collections::HashMap;

use uuid::Uuid;

use crate::db::traits::{CreateRoomInput, RoomMemberInput, RoomRepo, UpdateRoomInput};
use crate::db::{
    RoomExecutionOutputRow, RoomMemberRow, RoomRow, RoomSessionRow, RoomTranscriptEntry,
    WorkflowStepRow,
};
use crate::server::hub::dag::StepOutput;

use super::error::ServiceError;
use super::validation;

/// Verify the caller owns this room.
async fn verify_ownership(
    repo: &dyn RoomRepo,
    user_id: Uuid,
    room_id: Uuid,
) -> Result<RoomRow, ServiceError> {
    let room = repo
        .get_room(room_id)
        .await?
        .ok_or_else(|| ServiceError::not_found("Room"))?;
    super::ownership::check_direct_owner(room.user_id, user_id, "Room")?;
    Ok(room)
}

/// Create a new room.
pub async fn create_room(
    repo: &dyn RoomRepo,
    _user_id: Uuid,
    input: CreateRoomInput,
) -> Result<RoomRow, ServiceError> {
    validation::validate_name(&input.name, "Room name")?;
    let row = repo.create_room(input).await?;
    Ok(row)
}

/// Get a single room by ID, verifying ownership.
pub async fn get_room(
    repo: &dyn RoomRepo,
    user_id: Uuid,
    room_id: Uuid,
) -> Result<RoomRow, ServiceError> {
    verify_ownership(repo, user_id, room_id).await
}

/// Update an existing room (partial update).
pub async fn update_room(
    repo: &dyn RoomRepo,
    user_id: Uuid,
    room_id: Uuid,
    input: UpdateRoomInput,
) -> Result<RoomRow, ServiceError> {
    verify_ownership(repo, user_id, room_id).await?;
    if let Some(ref name) = input.name {
        validation::validate_name(name, "Room name")?;
    }
    let row = repo.update_room(input).await?;
    Ok(row)
}

/// Delete a room by ID, verifying ownership.
pub async fn delete_room(
    repo: &dyn RoomRepo,
    user_id: Uuid,
    room_id: Uuid,
) -> Result<(), ServiceError> {
    verify_ownership(repo, user_id, room_id).await?;
    repo.delete_room(room_id).await?;
    Ok(())
}

/// List all members of a room.
pub async fn list_room_members(
    repo: &dyn RoomRepo,
    room_id: Uuid,
) -> Result<Vec<RoomMemberRow>, ServiceError> {
    let rows = repo.list_room_members(room_id).await?;
    Ok(rows)
}

/// Add a single member to a room.
pub async fn add_room_member(
    repo: &dyn RoomRepo,
    room_id: Uuid,
    agent_id: Uuid,
    display_name: Option<String>,
    role_description: String,
    display_order: i32,
) -> Result<(), ServiceError> {
    repo.add_room_member(
        room_id,
        agent_id,
        display_name,
        role_description,
        display_order,
    )
    .await?;
    Ok(())
}

/// Remove a single member from a room.
pub async fn remove_room_member(
    repo: &dyn RoomRepo,
    room_id: Uuid,
    agent_id: Uuid,
) -> Result<(), ServiceError> {
    repo.remove_room_member(room_id, agent_id).await?;
    Ok(())
}

/// Replace all members of a room atomically.
pub async fn set_room_members(
    repo: &dyn RoomRepo,
    room_id: Uuid,
    members: &[RoomMemberInput],
) -> Result<(), ServiceError> {
    repo.set_room_members(room_id, members).await?;
    Ok(())
}

/// Start a new room session.
pub async fn create_room_session(
    repo: &dyn RoomRepo,
    room_id: Uuid,
) -> Result<RoomSessionRow, ServiceError> {
    let row = repo.create_room_session(room_id).await?;
    Ok(row)
}

/// Get a room session by ID.
pub async fn get_room_session(
    repo: &dyn RoomRepo,
    session_id: Uuid,
) -> Result<RoomSessionRow, ServiceError> {
    let row = repo
        .get_room_session(session_id)
        .await?
        .ok_or_else(|| ServiceError::not_found("RoomSession"))?;
    Ok(row)
}

/// Get the full transcript for a room session.
pub async fn get_room_transcript(
    repo: &dyn RoomRepo,
    session_id: Uuid,
) -> Result<Vec<RoomTranscriptEntry>, ServiceError> {
    let entries = repo.get_room_transcript(session_id).await?;
    Ok(entries)
}

/// List structured outputs for a room session.
pub async fn list_room_outputs(
    repo: &dyn RoomRepo,
    session_id: Uuid,
) -> Result<Vec<RoomExecutionOutputRow>, ServiceError> {
    // Verify session exists before fetching outputs.
    get_room_session(repo, session_id).await?;
    let rows = repo.get_room_execution_outputs(session_id, None).await?;
    Ok(rows)
}

/// Build a [`StepOutput`] from a room transcript for DAG resume.
///
/// Groups transcript entries by agent, takes each agent's last message,
/// and builds a composite JSON object. This is a pure function with no
/// async or repo dependencies.
pub(crate) fn build_room_step_output(
    transcript: &[RoomTranscriptEntry],
    step: &WorkflowStepRow,
) -> StepOutput {
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

    StepOutput {
        variable_name: step.output_variable_name.clone().unwrap_or_default(),
        raw_output: serde_json::to_string(&envelope_data).unwrap_or_default(),
        structured_output: Some(envelope_data),
    }
}

mod tests;
