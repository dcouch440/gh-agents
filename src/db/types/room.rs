use chrono::{DateTime, Utc};
use uuid::Uuid;

/// Row type for room definitions (pipeline-scoped).
#[derive(Debug, Clone, serde::Serialize, sqlx::FromRow)]
pub struct RoomRow {
    pub id: Uuid,
    pub user_id: Uuid,
    pub collection_id: Option<Uuid>,
    pub name: String,
    pub gatekeeper_enabled: bool,
    pub gatekeeper_model_id: String,
    pub max_speakers_per_turn: i32,
    pub max_turns: i32,
    pub tools_enabled: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Row type for room membership (join table).
#[derive(Debug, Clone, serde::Serialize, sqlx::FromRow)]
pub struct RoomMemberRow {
    pub room_id: Uuid,
    pub agent_id: Uuid,
    pub display_name: Option<String>,
    pub role_description: String,
    pub display_order: i32,
}

/// Row type for room session records (runtime).
#[derive(Debug, Clone, serde::Serialize, sqlx::FromRow)]
pub struct RoomSessionRow {
    pub id: Uuid,
    pub room_id: Uuid,
    pub status: String,
    pub current_turn: i32,
    pub transcript_summary: Option<String>,
    pub started_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
    pub structured_outputs: Option<serde_json::Value>,
    pub final_decision: Option<serde_json::Value>,
}

/// Labeled entry from a room transcript (cross-execution join).
#[derive(Debug, Clone, serde::Serialize, sqlx::FromRow)]
pub struct RoomTranscriptEntry {
    pub agent_name: String,
    pub role_description: String,
    pub content: String,
    pub speaker_order: Option<i32>,
    pub created_at: DateTime<Utc>,
}

/// Structured output from a room member for agent-to-agent data passing
#[derive(Debug, Clone, serde::Serialize, sqlx::FromRow)]
pub struct RoomExecutionOutputRow {
    pub id: Uuid,
    pub room_session_id: Uuid,
    pub agent_execution_id: Uuid,
    pub agent_id: Uuid,
    pub speaker_order: i32,
    pub turn_number: i32,
    pub output_name: String,
    pub structured_output: serde_json::Value,
    pub raw_output: String,
    pub schema_id: Option<Uuid>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, sqlx::FromRow)]
pub struct RoomStepConfigRow {
    pub id: Uuid,
    pub step_id: Uuid,
    pub meeting_purpose: String,
    pub max_turns: i32,
    pub interaction_mode: String,
    pub gatekeeper_enabled: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, sqlx::FromRow)]
pub struct RoomStepMemberRow {
    pub id: Uuid,
    pub step_id: Uuid,
    pub name: String,
    pub role: String,
    pub perspective: String,
    pub display_order: i32,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}
