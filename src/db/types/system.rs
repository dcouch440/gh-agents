use chrono::{DateTime, Utc};
use uuid::Uuid;

/// Row type for persisted output schema definitions.
#[derive(Debug, Clone, serde::Serialize, sqlx::FromRow)]
pub struct OutputSchemaRow {
    pub id: Uuid,
    pub user_id: Option<Uuid>,
    pub name: String,
    pub schema: serde_json::Value,
    pub created_at: DateTime<Utc>,
    pub version: i32,
}

/// Row type for persisted prompt template definitions.
#[derive(Debug, Clone, serde::Serialize, sqlx::FromRow)]
pub struct PromptTemplateRow {
    pub id: Uuid,
    pub user_id: Option<Uuid>,
    pub name: String,
    pub content: String,
    pub created_at: DateTime<Utc>,
    pub version: i32,
}

/// System configuration entry (admin-controlled)
#[derive(Debug, Clone, serde::Serialize, sqlx::FromRow)]
pub struct SystemConfigRow {
    pub id: Uuid,
    pub config_type: String,
    pub config_key: String,
    pub config_value: serde_json::Value,
    pub description: Option<String>,
    pub created_by: Option<Uuid>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Row type for saved structured results.
#[derive(Debug, Clone, serde::Serialize, sqlx::FromRow)]
pub struct ResultRow {
    pub id: Uuid,
    pub user_id: Uuid,
    pub agent_execution_id: Uuid,
    pub output_schema_id: Option<Uuid>,
    pub name: String,
    pub data: serde_json::Value,
    pub created_at: DateTime<Utc>,
}
