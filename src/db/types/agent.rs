use chrono::{DateTime, Utc};
use uuid::Uuid;

/// Row type for persisted agent definitions.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, sqlx::FromRow)]
pub struct AgentRow {
    pub id: Uuid,
    pub user_id: Option<Uuid>,
    pub tier: Option<String>,
    pub name: String,
    pub system_prompt: String,
    pub persona_style: Option<String>,
    pub model_provider: String,
    pub model_id: String,
    pub model_max_tokens: i32,
    pub model_temperature: f32,
    pub status: Option<String>,
    pub output_schema_id: Option<Uuid>,
    pub version: i32,
    pub default_reasoning_trace: Option<bool>,
    pub is_system: bool,
}

/// Row type for agent guidance (distilled feedback / learned instructions).
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct AgentGuidanceRow {
    pub id: Uuid,
    pub agent_id: Uuid,
    pub workflow_step_id: Option<Uuid>,
    pub suggestions: serde_json::Value,
    pub source: String,
    pub version: i32,
    pub is_active: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl Default for AgentRow {
    fn default() -> Self {
        Self {
            id: Uuid::nil(),
            user_id: None,
            tier: None,
            name: String::new(),
            system_prompt: String::new(),
            persona_style: None,
            model_provider: crate::constants::ACTIVE_PROVIDER.to_string(),
            model_id: crate::constants::MODEL_TIER2.to_string(),
            model_max_tokens: 4096,
            model_temperature: 0.7,
            status: None,
            output_schema_id: None,
            version: 1,
            default_reasoning_trace: None,
            is_system: false,
        }
    }
}
