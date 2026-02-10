//! Core types for the Protocol Layer.

use serde::{Deserialize, Serialize};
use uuid::Uuid;

// ============================================================================
// Protocol Configuration (input to expansion)
// ============================================================================

/// A fully-loaded protocol with its port assignments, ready for expansion.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProtocolConfig {
    /// The protocol type: "decomp", "transform", "review", "route".
    pub protocol_type: String,
    /// Type-specific configuration (content schema shape, review options, etc.).
    pub config: serde_json::Value,
    /// The configured port slots with agent assignments.
    pub ports: Vec<PortConfig>,
}

/// A single port slot within a protocol configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PortConfig {
    /// Port identifier (e.g., "frontend", "backend").
    pub port_name: String,
    /// Human-readable description, injected into the orchestrator prompt.
    pub description: String,
    /// The agent assigned to this port.
    pub agent_id: Uuid,
    /// Agent name (resolved from DB, used for prompt injection).
    pub agent_name: String,
    /// Tool names available to this port's agent (for prompt injection).
    pub agent_tools: Vec<String>,
    /// Display ordering.
    pub display_order: i32,
    /// Optional typed content schema from the agent's output_schema.
    /// When present, used to generate `oneOf` variants in the output schema.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub content_schema: Option<serde_json::Value>,
}

// ============================================================================
// Protocol Expansion (output of expansion — workflow primitives)
// ============================================================================

/// The result of expanding a protocol into concrete workflow primitives.
/// This is a pure data structure — no DB side effects.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProtocolExpansion {
    /// Auto-generated output schema for the orchestrator step.
    pub output_schema: serde_json::Value,
    /// Text to auto-inject into the orchestrator agent's prompt.
    pub prompt_injection: String,
    /// Downstream steps to create (one per port for decomp, etc.).
    pub steps: Vec<StepDefinition>,
    /// Edges to wire between orchestrator and downstream steps.
    pub edges: Vec<EdgeDefinition>,
    /// Output port definitions for the orchestrator step.
    pub output_ports: Vec<OutputPortDefinition>,
    /// Input port definitions for downstream steps.
    pub input_ports: Vec<InputPortDefinition>,
}

/// A step to be created as part of protocol expansion.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StepDefinition {
    /// Which protocol port this step fulfills.
    pub port_name: String,
    /// Assigned agent ID (fallback agent for label-routed steps). None for agent-less steps.
    pub agent_id: Option<Uuid>,
    /// Step execution mode ("single", "for_each", etc.).
    pub execution_mode: String,
    /// Optional prompt template for the step.
    pub prompt_template: Option<String>,
    /// Optional output schema for the step.
    pub output_schema: Option<serde_json::Value>,
    /// Routing mode for label-based dispatch (e.g., "label").
    pub routing_mode: Option<String>,
    /// Field name used for routing lookup on the step (e.g., "port").
    pub routing_field: Option<String>,
    /// For for_each steps: field in each element to extract the label from.
    pub for_each_label_field: Option<String>,
    /// Reference to the variable containing the array to iterate.
    /// Use `"{anchor_output}"` as sentinel; the apply handler resolves it.
    pub for_each_ref: Option<String>,
    /// Routing rules for label-based agent dispatch.
    pub routing_rules: Vec<RoutingRuleDefinition>,
}

/// A routing rule to create on a step during protocol application.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoutingRuleDefinition {
    /// Label value that triggers this rule (matches for_each_label_field).
    pub label_value: String,
    /// Human-readable description of what this route handles.
    pub description: Option<String>,
    /// Agent to dispatch to when this label matches.
    pub agent_id: Uuid,
    /// Display ordering.
    pub display_order: i32,
}

/// An edge to be created between the orchestrator and a downstream step.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EdgeDefinition {
    /// Port name on the source (orchestrator) step.
    pub from_output_port: String,
    /// Port name on the target (downstream) step.
    pub to_input_port: String,
    /// Which protocol port this edge serves.
    pub target_port_name: String,
    /// Optional condition for conditional edges (route, review).
    pub condition_type: Option<String>,
    /// Optional condition value.
    pub condition_value: Option<serde_json::Value>,
}

/// An output port to create on the orchestrator step.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OutputPortDefinition {
    pub port_name: String,
    pub port_type: String,
    pub json_path: String,
    pub description: Option<String>,
}

/// An input port to create on a downstream step.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InputPortDefinition {
    /// Which protocol port (downstream step) this belongs to.
    pub target_port_name: String,
    pub port_name: String,
    pub port_type: String,
    pub required: bool,
    pub description: Option<String>,
}

// ============================================================================
// Applied Protocol (result of apply — includes created DB entity IDs)
// ============================================================================

/// The result of applying a protocol expansion to a workflow.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppliedProtocol {
    /// The output schema ID that was created/assigned.
    pub output_schema_id: Uuid,
    /// Map of port_name → created step ID.
    pub created_steps: Vec<CreatedStep>,
    /// IDs of edges that were created.
    pub created_edge_ids: Vec<Uuid>,
}

/// A step created during protocol application.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreatedStep {
    pub port_name: String,
    pub step_id: Uuid,
    pub agent_id: Uuid,
}
