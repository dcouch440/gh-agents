use chrono::{DateTime, Utc};
use uuid::Uuid;

/// Row type for persisted tool definitions.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, sqlx::FromRow)]
pub struct ToolRow {
    pub id: Uuid,
    pub name: String,
    pub display_name: String,
    pub description: String,
    pub parameters: serde_json::Value,
    pub created_at: DateTime<Utc>,
    pub version: i32,
}

/// Tool capability taxonomy (semantic capabilities)
#[derive(Debug, Clone, serde::Serialize, sqlx::FromRow, utoipa::ToSchema)]
pub struct ToolCapabilityRow {
    pub id: Uuid,
    pub capability_key: String,
    pub display_name: String,
    pub category: String,
    pub safety_level: String,
    pub description: String,
    pub created_at: DateTime<Utc>,
}

/// Tool-to-capability assignment (which capabilities each tool provides)
#[derive(Debug, Clone, serde::Serialize, sqlx::FromRow)]
pub struct ToolCapabilityAssignmentRow {
    pub tool_id: Uuid,
    pub capability_id: Uuid,
}

/// Mode-to-capability requirement (which capabilities each mode requires)
#[derive(Debug, Clone, serde::Serialize, sqlx::FromRow)]
pub struct ModeRequiredCapabilityRow {
    pub mode_id: Uuid,
    pub capability_id: Uuid,
    pub is_required: bool,
}
