use chrono::{DateTime, Utc};
use uuid::Uuid;

/// Row type for protocol definitions (reusable execution recipes).
#[derive(Debug, Clone, serde::Serialize, sqlx::FromRow)]
pub struct ProtocolRow {
    pub id: Uuid,
    pub name: String,
    pub description: String,
    pub protocol_type: String, // e.g. "workforce"
    pub config: serde_json::Value,
    pub version: i32,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub agent_id: Option<Uuid>,
    pub output_schema_id: Option<Uuid>,
    pub prompt_template_id: Option<Uuid>,
}

/// Row type for protocol port slots (agent assignments within a protocol).
#[derive(Debug, Clone, serde::Serialize, sqlx::FromRow)]
pub struct ProtocolPortRow {
    pub id: Uuid,
    pub protocol_id: Uuid,
    pub port_name: String,
    pub description: String,
    pub agent_id: Uuid,
    pub display_order: i32,
}

/// Row type for protocol document definitions (workforce step config).
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, sqlx::FromRow)]
pub struct ProtocolDocumentDefRow {
    pub id: Uuid,
    pub step_id: Option<Uuid>,
    pub name: String,
    pub description: String,
    pub target_length: i32,
    pub display_order: i32,
    pub created_at: DateTime<Utc>,
    pub protocol_id: Option<Uuid>,
    pub document_id: Option<Uuid>,
    /// Agent roster entry that produces this deliverable (workforce archetype).
    pub agent_roster_entry_id: Option<Uuid>,
}

/// Row type for protocol execution audit trail (protocol hidden phases).
#[derive(Debug, Clone, serde::Serialize, sqlx::FromRow)]
pub struct ProtocolExecutionRow {
    pub id: Uuid,
    pub protocol_step_id: Uuid,
    pub workflow_run_id: Option<Uuid>,
    pub phase: String,
    pub document_def_id: Option<Uuid>,
    pub agent_id: Option<Uuid>,
    pub input_prompt: Option<String>,
    pub output_content: Option<String>,
    pub status: String,
    pub error_message: Option<String>,
    pub tokens_in: Option<i32>,
    pub tokens_out: Option<i32>,
    pub cost_usd: Option<f64>,
    pub model: Option<String>,
    pub capabilities_used: Option<Vec<String>>,
    pub created_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
    /// Human-readable agent name (e.g. "Scanner") for workforce agent phases.
    pub agent_name: Option<String>,
    /// Protocol archetype that produced this phase (e.g. "workforce").
    pub archetype: Option<String>,
    /// Links agent phases back to the designer run that created them.
    pub designer_run_id: Option<Uuid>,
}

/// Row type for workflow step ↔ protocol linkage.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, sqlx::FromRow)]
pub struct WorkflowStepProtocolRow {
    pub id: Uuid,
    pub workflow_step_id: Uuid,
    pub protocol_id: Uuid,
    pub applied_expansion: serde_json::Value,
    pub created_at: DateTime<Utc>,
}
