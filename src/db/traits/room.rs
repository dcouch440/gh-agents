use anyhow::Result;
use async_trait::async_trait;
use uuid::Uuid;

use crate::db::{
    RoomExecutionOutputRow, RoomMemberRow, RoomRow, RoomSessionRow, RoomTranscriptEntry,
};

// ============================================================================
// Room Repository
// ============================================================================

/// Input type for setting room members in bulk.
#[derive(Debug, Clone)]
pub struct RoomMemberInput {
    pub agent_id: Uuid,
    pub display_name: Option<String>,
    pub role_description: String,
    pub display_order: i32,
}

/// Input for creating a new room.
#[derive(Debug, Clone)]
pub struct CreateRoomInput {
    pub user_id: Uuid,
    pub collection_id: Option<Uuid>,
    pub name: String,
    pub gatekeeper_enabled: bool,
    pub gatekeeper_model_id: String,
    pub max_speakers_per_turn: i32,
    pub max_turns: i32,
    pub tools_enabled: bool,
}

/// Input for updating a room's configuration.
#[derive(Debug, Clone)]
pub struct UpdateRoomInput {
    pub id: Uuid,
    pub name: Option<String>,
    pub gatekeeper_enabled: Option<bool>,
    pub gatekeeper_model_id: Option<String>,
    pub max_speakers_per_turn: Option<i32>,
    pub max_turns: Option<i32>,
    pub tools_enabled: Option<bool>,
}

/// Input for saving a room execution output.
#[derive(Debug, Clone)]
pub struct SaveRoomExecutionOutputInput {
    pub room_session_id: Uuid,
    pub agent_execution_id: Uuid,
    pub agent_id: Uuid,
    pub speaker_order: i32,
    pub turn_number: i32,
    pub output_name: String,
    pub structured_output: serde_json::Value,
    pub raw_output: String,
    pub schema_id: Option<Uuid>,
}

/// Database operations for rooms, room members, and room sessions.
#[cfg_attr(test, mockall::automock)]
#[async_trait]
pub trait RoomRepo: Send + Sync {
    // --- Room CRUD ---

    /// Create a new room within a collection.
    async fn create_room(&self, input: CreateRoomInput) -> Result<RoomRow>;

    /// Get a room by ID.
    async fn get_room(&self, id: Uuid) -> Result<Option<RoomRow>>;

    /// Update a room's configuration.
    async fn update_room(&self, input: UpdateRoomInput) -> Result<RoomRow>;

    /// Delete a room by ID.
    async fn delete_room(&self, id: Uuid) -> Result<()>;

    // --- Room members (join table) ---

    /// List all members of a room, ordered by display_order.
    async fn list_room_members(&self, room_id: Uuid) -> Result<Vec<RoomMemberRow>>;

    /// Add a single member to a room.
    async fn add_room_member(
        &self,
        room_id: Uuid,
        agent_id: Uuid,
        display_name: Option<String>,
        role_description: String,
        display_order: i32,
    ) -> Result<()>;

    /// Remove a single member from a room.
    async fn remove_room_member(&self, room_id: Uuid, agent_id: Uuid) -> Result<()>;

    /// Replace all members of a room atomically.
    async fn set_room_members(&self, room_id: Uuid, members: &[RoomMemberInput]) -> Result<()>;

    // --- Room sessions (runtime) ---

    /// Start a new room session.
    async fn create_room_session(&self, room_id: Uuid) -> Result<RoomSessionRow>;

    /// Get a room session by ID.
    async fn get_room_session(&self, id: Uuid) -> Result<Option<RoomSessionRow>>;

    /// Update room session status.
    async fn update_room_session_status(&self, id: Uuid, status: &str) -> Result<()>;

    /// Increment turn counter and return new value.
    async fn increment_room_session_turn(&self, id: Uuid) -> Result<i32>;

    /// Set the compressed transcript summary for older turns.
    async fn set_transcript_summary(&self, id: Uuid, summary: &str) -> Result<()>;

    // --- Room transcript ---

    /// Load the full room transcript (cross-execution message join).
    async fn get_room_transcript(&self, room_session_id: Uuid) -> Result<Vec<RoomTranscriptEntry>>;

    // --- Room Execution Outputs (Phase 3) ---

    /// Save a structured output from a room speaker
    async fn save_room_execution_output(
        &self,
        input: SaveRoomExecutionOutputInput,
    ) -> Result<RoomExecutionOutputRow>;

    /// Get room execution outputs, optionally filtered by turn number
    async fn get_room_execution_outputs(
        &self,
        room_session_id: Uuid,
        turn_number: Option<i32>,
    ) -> Result<Vec<RoomExecutionOutputRow>>;

    /// Get room execution outputs by schema ID
    async fn get_room_outputs_by_schema(
        &self,
        room_session_id: Uuid,
        schema_id: Uuid,
    ) -> Result<Vec<RoomExecutionOutputRow>>;
}
