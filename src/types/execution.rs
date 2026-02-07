//! Execution envelope types for standardized output wrapping
//!
//! All step executions return StepExecutionEnvelope with consistent structure.
//! For-each steps return ForEachAggregateEnvelope containing iteration envelopes.

use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Standard execution status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExecutionStatus {
    Success,
    Error,
    Partial, // For for-each with some failures
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

    // For for-each iterations
    pub iteration_index: Option<usize>,
    pub iteration_label: Option<String>,

    // For label-based routing
    pub routing_label: Option<String>,

    // For cavernous routing
    pub selected_routing_document_id: Option<Uuid>,

    // For chained for-each pipeline (Phase 6B)
    pub upstream_agent_id: Option<Uuid>,
    pub upstream_routing_label: Option<String>,

    // For room step execution
    pub room_session_id: Option<Uuid>,
    pub room_id: Option<Uuid>,
    pub total_rounds: Option<i32>,
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
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StepExecutionEnvelope {
    pub status: ExecutionStatus,
    pub data: Option<serde_json::Value>,
    pub metadata: ExecutionMetadata,
    pub error: Option<ExecutionError>,
}

/// For-each iteration error summary
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IterationError {
    pub iteration_index: usize,
    pub iteration_label: Option<String>,
    pub message: String,
    pub error_type: String,
}

/// Aggregate metadata for for-each executions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ForEachMetadata {
    pub total_iterations: usize,
    pub successful_iterations: usize,
    pub failed_iterations: usize,
    pub execution_time_ms: u64,
    pub total_tokens_in: i32,
    pub total_tokens_out: i32,
    pub total_cost_usd: f64,

    // For label routing
    pub routing_mode: Option<String>,
    pub routing_distribution: Option<std::collections::HashMap<String, usize>>,
}

/// Aggregate envelope for for-each executions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ForEachAggregateEnvelope {
    pub status: ExecutionStatus,
    pub data: Vec<StepExecutionEnvelope>,
    pub metadata: ForEachMetadata,
    pub errors: Vec<IterationError>,
}

// ============================================================================
// Chained For-Each Pipeline (Phase 6B)
// ============================================================================

/// Result of one pipeline stage for one item in a chained for-each execution.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PipelineStageResult {
    pub step_id: Uuid,
    pub status: ExecutionStatus,
    pub execution_time_ms: u64,
    pub agent_id: Option<Uuid>,
    pub routing_label: Option<String>,
}

// ============================================================================
// Downstream Routing Context (Phase 6)
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
                execution_id: Uuid::new_v4(),
                execution_time_ms: 100,
                tokens_in: Some(10),
                tokens_out: Some(20),
                cost_usd: Some(0.001),
                model: Some("claude-opus-4".into()),
                agent_id: Some(Uuid::new_v4()),
                iteration_index: None,
                iteration_label: None,
                routing_label: None,
                selected_routing_document_id: None,
                upstream_agent_id: None,
                upstream_routing_label: None,
                room_session_id: None,
                room_id: None,
                total_rounds: None,
            },
            error: None,
        };

        let json = serde_json::to_string(&envelope).unwrap();
        let parsed: StepExecutionEnvelope = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.status, ExecutionStatus::Success);
    }

    #[test]
    fn for_each_aggregate_structure() {
        let agg = ForEachAggregateEnvelope {
            status: ExecutionStatus::Partial,
            data: vec![],
            metadata: ForEachMetadata {
                total_iterations: 3,
                successful_iterations: 2,
                failed_iterations: 1,
                execution_time_ms: 500,
                total_tokens_in: 100,
                total_tokens_out: 200,
                total_cost_usd: 0.01,
                routing_mode: Some("label".into()),
                routing_distribution: None,
            },
            errors: vec![IterationError {
                iteration_index: 1,
                iteration_label: Some("failed item".into()),
                message: "Test error".into(),
                error_type: "TestError".into(),
            }],
        };

        assert_eq!(agg.metadata.failed_iterations, 1);
        assert_eq!(agg.errors.len(), 1);
    }
}
