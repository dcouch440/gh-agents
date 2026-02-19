//! Execution envelope types for standardized output wrapping
//!
//! All step executions return StepExecutionEnvelope with consistent structure.

use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Standard execution status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExecutionStatus {
    #[default]
    Success,
    Error,
    Partial,
    Skipped, // Step skipped due to unmatched conditional edges
}

/// Execution metadata (timing, costs, routing info)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionMetadata {
    pub execution_id: Uuid,
    pub execution_time_ms: u64,
    pub tokens_in: Option<i32>,
    pub tokens_out: Option<i32>,
    pub cost_usd: Option<f64>,
    pub model: Option<String>,
    pub agent_id: Option<Uuid>,

    pub iteration_index: Option<usize>,
    pub iteration_label: Option<String>,
    pub routing_label: Option<String>,
    pub upstream_agent_id: Option<Uuid>,
    pub upstream_routing_label: Option<String>,

    // For room step execution
    pub room_session_id: Option<Uuid>,
    pub room_id: Option<Uuid>,
    pub total_rounds: Option<i32>,

    // For sub-workflow steps
    pub child_workflow_execution_id: Option<Uuid>,
}

impl ExecutionMetadata {
    /// Create metadata with all optional fields set to None.
    pub fn new(execution_id: Uuid) -> Self {
        Self {
            execution_id,
            execution_time_ms: 0,
            tokens_in: None,
            tokens_out: None,
            cost_usd: None,
            model: None,
            agent_id: None,
            iteration_index: None,
            iteration_label: None,
            routing_label: None,
            upstream_agent_id: None,
            upstream_routing_label: None,
            room_session_id: None,
            room_id: None,
            total_rounds: None,
            child_workflow_execution_id: None,
        }
    }
}

impl Default for ExecutionMetadata {
    fn default() -> Self {
        Self::new(Uuid::nil())
    }
}

/// Execution error details
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionError {
    pub message: String,
    pub error_type: String,
    pub retryable: bool,
    pub details: Option<serde_json::Value>,
}

/// Standard execution envelope (single step)
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct StepExecutionEnvelope {
    pub status: ExecutionStatus,
    pub data: Option<serde_json::Value>,
    pub metadata: ExecutionMetadata,
    pub error: Option<ExecutionError>,
}

// ============================================================================
// Downstream Routing Context
// ============================================================================

/// Context about a downstream label-routing step, used to inject
/// routing instructions into the upstream planner's prompt.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DownstreamRoutingContext {
    pub downstream_step_id: Uuid,
    pub routing_field: String,
    pub routes: Vec<RouteDescription>,
}

/// Description of a single routing rule for prompt injection.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RouteDescription {
    pub label_value: String,
    pub description: Option<String>,
    pub agent_name: String,
    pub agent_tools: Vec<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn envelope_serde_roundtrip() {
        let envelope = StepExecutionEnvelope {
            status: ExecutionStatus::Success,
            data: Some(serde_json::json!({"result": "test"})),
            metadata: ExecutionMetadata {
                execution_time_ms: 100,
                tokens_in: Some(10),
                tokens_out: Some(20),
                cost_usd: Some(0.001),
                model: Some("claude-opus-4".into()),
                agent_id: Some(Uuid::new_v4()),
                ..ExecutionMetadata::new(Uuid::new_v4())
            },
            error: None,
        };

        let json = serde_json::to_string(&envelope).unwrap();
        let parsed: StepExecutionEnvelope = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.status, ExecutionStatus::Success);
    }
}
