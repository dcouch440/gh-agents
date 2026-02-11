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
// Sync Statistics
// ============================================================================

#[derive(Debug, Clone, Default, Serialize)]
pub struct SyncStats {
    pub capabilities_created: usize,
    pub capabilities_updated: usize,
    pub tool_assignments_updated: usize,
    pub constraints_created: usize,
    pub constraints_updated: usize,
    pub routing_strategies_created: usize,
    pub routing_strategies_updated: usize,
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
