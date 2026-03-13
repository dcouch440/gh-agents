use chrono::{DateTime, Utc};
use uuid::Uuid;

/// Row type for persisted canvas snapshots (one per workflow, upserted on board submit).
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct CanvasSnapshotRow {
    pub workflow_id: Uuid,
    pub snapshot_json: String,
    pub elements_json: String,
    pub last_response_json: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Maps Excalidraw element IDs to workflow step or edge UUIDs.
/// Exactly one of `step_id` or `edge_id` is populated (XOR constraint in DB).
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct CanvasElementMapRow {
    pub workflow_id: Uuid,
    pub element_id: String,
    pub step_id: Option<Uuid>,
    pub edge_id: Option<Uuid>,
    pub created_at: DateTime<Utc>,
}
