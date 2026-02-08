//! YAML deserialization types for config files

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// ============================================================================
// Capabilities (capabilities.yaml)
// ============================================================================

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct CapabilitiesYaml {
    pub capabilities: Vec<Capability>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Capability {
    pub key: String,
    pub display_name: String,
    pub category: String,
    pub safety_level: String,
    pub description: String,
    #[serde(default)]
    pub examples: Vec<String>,
    #[serde(default)]
    pub notes: Option<String>,
    #[serde(default)]
    pub requires_approval: bool,
    #[serde(default)]
    pub default_enabled: bool,
}

// ============================================================================
// Tool Assignments (tool_assignments.yaml)
// ============================================================================

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ToolAssignmentsYaml {
    pub tool_assignments: HashMap<String, ToolAssignment>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ToolAssignment {
    pub capabilities: Vec<String>,
    #[serde(default)]
    pub priority: i32,
    #[serde(default)]
    pub notes: Option<String>,
    #[serde(default)]
    pub requires_approval: bool,
}

// ============================================================================
// Constraints (constraints.yaml)
// ============================================================================

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ConstraintsYaml {
    pub constraints: HashMap<String, ConstraintConfig>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ConstraintConfig {
    pub value: serde_json::Value,
    pub config_type: String, // "integer", "float", "boolean", "string"
    pub description: String,
    #[serde(default)]
    pub tenant_override: bool,
    #[serde(default)]
    pub recommended_range: Option<String>,
}

// ============================================================================
// System Agents (system_agents.yaml)
// ============================================================================

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct SystemAgentsYaml {
    pub system_agents: Vec<SystemAgent>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct SystemAgent {
    pub name: String,
    pub role: String,
    pub description: String,
    pub system_prompt: String,
    #[serde(default)]
    pub capabilities_required: Vec<String>,
    #[serde(default)]
    pub default_tools: Vec<String>,
    pub safety_level: String,
    #[serde(default)]
    pub is_system: bool,
    #[serde(default)]
    pub is_gatekeeper: bool,
}

// ============================================================================
// Routing Strategies (routing_strategies/*.yaml)
// ============================================================================

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct RoutingStrategyYaml {
    pub strategy_name: String,
    pub description: String,
    #[serde(default)]
    pub capabilities_required: Vec<String>,
    pub subtasks: Vec<Subtask>,
    pub aggregation_mode: String,
    #[serde(default)]
    pub max_parallel: usize,
    #[serde(default)]
    pub timeout_minutes: Option<i32>,
    #[serde(default)]
    pub cost_limit_usd: Option<f64>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Subtask {
    pub id: String,
    pub task_name: String,
    pub agent_role: String,
    pub agent_id: String,
    #[serde(default)]
    pub tools: Vec<String>,
    pub prompt_template: String,
    #[serde(default)]
    pub depends_on: Vec<String>,
    #[serde(default)]
    pub input_mapping: HashMap<String, String>,
    #[serde(default)]
    pub output_schema: Option<serde_json::Value>,
}

// ============================================================================
// Protocols (protocols.yaml)
// ============================================================================

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ProtocolsYaml {
    pub protocols: Vec<ProtocolDef>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ProtocolDef {
    pub name: String,
    pub description: String,
    pub protocol_type: String,
    #[serde(default)]
    pub config: Option<serde_json::Value>,
    pub agent: ProtocolAgentDef,
    pub output_schema: ProtocolSchemaDef,
    #[serde(default)]
    pub prompt_template: Option<ProtocolTemplateDef>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ProtocolAgentDef {
    pub name: String,
    pub system_prompt: String,
    #[serde(default = "default_model_provider")]
    pub model_provider: String,
    #[serde(default = "default_model_id")]
    pub model_id: String,
    #[serde(default = "default_max_tokens")]
    pub model_max_tokens: i32,
    #[serde(default = "default_temperature")]
    pub model_temperature: f32,
}

fn default_model_provider() -> String {
    "anthropic".to_string()
}
fn default_model_id() -> String {
    "claude-sonnet-4-20250514".to_string()
}
fn default_max_tokens() -> i32 {
    8192
}
fn default_temperature() -> f32 {
    0.7
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ProtocolSchemaDef {
    pub name: String,
    pub schema: serde_json::Value,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ProtocolTemplateDef {
    pub name: String,
    pub content: String,
}

// ============================================================================
// Sync Statistics
// ============================================================================

#[derive(Debug, Clone, Default, Serialize)]
pub struct SyncStats {
    pub capabilities_created: usize,
    pub capabilities_updated: usize,
    pub tool_assignments_updated: usize,
    pub constraints_created: usize,
    pub constraints_updated: usize,
    pub system_agents_created: usize,
    pub system_agents_updated: usize,
    pub routing_strategies_created: usize,
    pub routing_strategies_updated: usize,
    pub protocols_created: usize,
    pub protocols_updated: usize,
    pub errors: Vec<String>,
}

impl SyncStats {
    pub fn has_errors(&self) -> bool {
        !self.errors.is_empty()
    }

    pub fn add_error(&mut self, error: impl Into<String>) {
        self.errors.push(error.into());
    }
}
