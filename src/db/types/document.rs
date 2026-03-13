use chrono::{DateTime, Utc};
use uuid::Uuid;

/// Row type for persisted documents.
#[derive(Debug, Clone, serde::Serialize, sqlx::FromRow)]
pub struct DocumentRow {
    pub id: Uuid,
    pub user_id: Uuid,
    pub session_id: Option<Uuid>,
    pub title: String,
    pub content: String,
    pub summary: Option<String>,
    pub doc_type: Option<String>,
    pub ref_tag: Option<String>,
    pub tags: Option<Vec<String>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub workflow_id: Option<Uuid>,
    pub target_length: Option<i32>,
    pub is_static: Option<bool>,
    pub source_protocol_step_id: Option<Uuid>,
}

/// Search result for documents (no full content).
#[derive(Debug, Clone, serde::Serialize, sqlx::FromRow, utoipa::ToSchema)]
pub struct DocumentSearchResult {
    pub id: Uuid,
    pub title: String,
    pub summary: Option<String>,
    pub ref_tag: Option<String>,
    pub snippet: String,
}

/// Row type for immutable content version snapshots.
#[derive(Debug, Clone, serde::Serialize, sqlx::FromRow)]
pub struct ContentVersionRow {
    pub id: Uuid,
    pub source_id: Uuid,
    pub content_type: String,
    pub content_hash: String,
    pub content: String,
    pub version_number: i32,
    pub byte_size: i32,
    pub created_at: DateTime<Utc>,
}

/// Row type for run snapshot linkage (run → content version).
#[derive(Debug, Clone, serde::Serialize, sqlx::FromRow)]
pub struct RunSnapshotRow {
    pub id: Uuid,
    pub run_id: Uuid,
    pub step_id: Uuid,
    pub content_type: String,
    pub role: String,
    pub content_version_id: Uuid,
    pub source_id: Uuid,
    pub created_at: DateTime<Utc>,
}

/// Lightweight row for reconstructing envelopes from snapshots (JOIN result).
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct EnvelopeSnapshotRow {
    pub step_id: Uuid,
    pub content: String,
    pub source_id: Uuid,
}

/// Row type for run templates (frozen workflow snapshots).
#[derive(Debug, Clone, serde::Serialize, sqlx::FromRow)]
pub struct RunTemplateRow {
    pub id: Uuid,
    pub workflow_id: Uuid,
    pub user_id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub snapshot: serde_json::Value,
    pub created_at: DateTime<Utc>,
}
