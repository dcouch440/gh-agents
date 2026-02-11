//! Conditional edge evaluation and step readiness determination.

use std::collections::HashMap;

use tracing::warn;
use uuid::Uuid;

use crate::db::WorkflowStepEdgeRow;
use crate::types::StepExecutionEnvelope;

use super::types::{StepOutput, StepReadiness};

/// Evaluate a single edge condition against the parent step's output envelope.
///
/// Supports:
/// - `"port_match"`: Checks `envelope.data[field] == value` (route protocol)
/// - `"equals"`: Checks `envelope.data[field] == value` (review protocol)
///
/// Returns `false` for unknown condition types (fail-closed), missing fields,
/// or null envelope data.
pub fn evaluate_edge_condition(
    edge: &WorkflowStepEdgeRow,
    parent_envelope: &StepExecutionEnvelope,
) -> bool {
    let condition_type = match &edge.condition_type {
        Some(ct) => ct.as_str(),
        None => return true, // No condition = unconditional edge
    };

    let condition_value = match &edge.condition_value {
        Some(cv) => cv,
        None => return false,
    };

    let data = match &parent_envelope.data {
        Some(d) => d,
        None => return false,
    };

    match condition_type {
        "equals" | "port_match" => {
            let field = match condition_value.get("field").and_then(|f| f.as_str()) {
                Some(f) => f,
                None => return false,
            };
            let expected = match condition_value.get("value") {
                Some(v) => v,
                None => return false,
            };
            let actual = match data.get(field) {
                Some(v) => v,
                None => return false,
            };
            actual == expected
        }
        _ => {
            warn!("Unknown edge condition type: {}", condition_type);
            false
        }
    }
}

/// Determine whether a step should execute based on its incoming edges.
///
/// Rules:
/// 1. Entry steps (no incoming edges) always execute.
/// 2. All unconditional parent steps must be completed.
/// 3. If conditional edges exist, at least one must have a completed parent
///    whose output matches the condition.
/// 4. If ONLY conditional edges exist and NONE match, the step is skipped.
pub fn check_step_readiness(
    step_id: Uuid,
    edges: &[WorkflowStepEdgeRow],
    completed: &HashMap<Uuid, StepOutput>,
    completed_envelopes: &HashMap<Uuid, StepExecutionEnvelope>,
) -> StepReadiness {
    let incoming: Vec<&WorkflowStepEdgeRow> =
        edges.iter().filter(|e| e.to_step_id == step_id).collect();

    if incoming.is_empty() {
        return StepReadiness::Ready;
    }

    // Partition into unconditional and conditional edges
    let unconditional: Vec<&&WorkflowStepEdgeRow> = incoming
        .iter()
        .filter(|e| e.condition_type.is_none())
        .collect();
    let conditional: Vec<&&WorkflowStepEdgeRow> = incoming
        .iter()
        .filter(|e| e.condition_type.is_some())
        .collect();

    // All unconditional parents must be completed
    for edge in &unconditional {
        if !completed.contains_key(&edge.from_step_id) {
            return StepReadiness::Waiting;
        }
    }

    // If there are conditional edges, at least one must match
    if !conditional.is_empty() {
        let mut any_parent_pending = false;
        let mut any_matched = false;

        for edge in &conditional {
            if let Some(envelope) = completed_envelopes.get(&edge.from_step_id) {
                if evaluate_edge_condition(edge, envelope) {
                    any_matched = true;
                    // Ensure the matched parent is actually in completed
                    if !completed.contains_key(&edge.from_step_id) {
                        return StepReadiness::Waiting;
                    }
                }
            } else {
                // Parent hasn't produced an envelope yet
                any_parent_pending = true;
            }
        }

        if !any_matched {
            // If some parents haven't run yet, we might match later
            if any_parent_pending {
                return StepReadiness::Waiting;
            }
            // All conditional parents ran, none matched => skip
            return StepReadiness::Skipped;
        }
    }

    StepReadiness::Ready
}
