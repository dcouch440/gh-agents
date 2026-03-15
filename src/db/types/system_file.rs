use chrono::{DateTime, Utc};
use uuid::Uuid;

/// Row type for system store file metadata.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, sqlx::FromRow)]
pub struct SystemFileRow {
    pub id: Uuid,
    pub workflow_id: Uuid,
    pub path: String,
    pub media_type: String,
    pub description: String,
    pub tags: Vec<String>,
    pub produced_by: Option<Uuid>,
    pub produced_by_agent: Option<String>,
    pub version: i32,
    pub size_bytes: i64,
    /// The workflow run that produced this file. NULL for design-time configs.
    pub workflow_run_id: Option<Uuid>,
    /// Whether this file is sealed (immutable). Set when the producing step is pinned.
    pub sealed: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}
