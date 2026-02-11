//! Port-based data flow: resolve input ports from upstream envelopes.

use std::collections::HashMap;

use serde_json::Value as JsonValue;
use uuid::Uuid;

use crate::db::{StepInputRow, StepOutputRow, WorkflowStepEdgeRow, WorkflowStepRow};
use crate::types::StepExecutionEnvelope;

use super::conditions::evaluate_edge_condition;

/// Error types for port resolution failures.
#[derive(Debug)]
pub enum PortResolutionError {
    /// A required input port has no incoming edge and no default value.
    MissingRequiredInput { port_name: String, step_id: Uuid },
    /// The source step hasn't completed yet (shouldn't happen in topo order).
    SourceStepNotCompleted {
        from_step_id: Uuid,
        port_name: String,
    },
    /// The output port definition was not found on the source step.
    OutputPortNotFound { step_id: Uuid, port_name: String },
    /// Data extraction from the envelope failed (json_path didn't match).
    DataExtractionFailed {
        step_id: Uuid,
        port_name: String,
        json_path: String,
    },
}

impl std::fmt::Display for PortResolutionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::MissingRequiredInput { port_name, step_id } => {
                write!(
                    f,
                    "required input '{}' missing for step {}",
                    port_name, step_id
                )
            }
            Self::SourceStepNotCompleted {
                from_step_id,
                port_name,
            } => {
                write!(
                    f,
                    "source step {} not completed for port '{}'",
                    from_step_id, port_name
                )
            }
            Self::OutputPortNotFound { step_id, port_name } => {
                write!(
                    f,
                    "output port '{}' not found on step {}",
                    port_name, step_id
                )
            }
            Self::DataExtractionFailed {
                step_id,
                port_name,
                json_path,
            } => {
                write!(
                    f,
                    "data extraction failed for port '{}' on step {} (json_path: {})",
                    port_name, step_id, json_path
                )
            }
        }
    }
}

impl std::error::Error for PortResolutionError {}

/// Navigate a JSON value using dot-path notation.
///
/// Supports field access (`"name"`), array index (`"0"`), and nesting (`"data.items.0.name"`).
/// Returns `None` if any segment fails to resolve or the final value is null.
pub fn resolve_dot_path(value: &JsonValue, path: &str) -> Option<JsonValue> {
    if path.is_empty() {
        return Some(value.clone());
    }
    let parts: Vec<&str> = path.split('.').collect();
    let mut current = value.clone();
    for part in &parts {
        current = if let Ok(idx) = part.parse::<usize>() {
            current.get(idx).cloned()?
        } else {
            current.get(*part).cloned()?
        };
    }
    if current.is_null() {
        None
    } else {
        Some(current)
    }
}

/// Resolve all input port values for a step from upstream envelopes.
///
/// For each incoming edge with `from_output_port` and `to_input_port` set:
/// 1. Gets the source step's completed envelope
/// 2. Finds the output port definition to get its `json_path`
/// 3. Extracts data from `envelope.data` via the json_path
/// 4. Optionally applies `transform_jsonpath` from the edge
/// 5. Maps to the `to_input_port` key in the result
///
/// Missing optional inputs are filled from `StepInputRow.default_value`.
/// Missing required inputs with no default produce an error.
pub fn resolve_port_inputs(
    step_id: Uuid,
    edges: &[WorkflowStepEdgeRow],
    step_inputs: &[StepInputRow],
    source_outputs_map: &HashMap<Uuid, Vec<StepOutputRow>>,
    completed_envelopes: &HashMap<Uuid, StepExecutionEnvelope>,
) -> std::result::Result<HashMap<String, JsonValue>, PortResolutionError> {
    let mut resolved: HashMap<String, JsonValue> = HashMap::new();

    // Find all incoming edges with port wiring, filtering out non-matching conditional edges
    let incoming_edges: Vec<&WorkflowStepEdgeRow> = edges
        .iter()
        .filter(|e| {
            e.to_step_id == step_id && e.from_output_port.is_some() && e.to_input_port.is_some()
        })
        .filter(|e| {
            // Skip conditional edges whose condition didn't match
            if e.condition_type.is_some() {
                completed_envelopes
                    .get(&e.from_step_id)
                    .map(|env| evaluate_edge_condition(e, env))
                    .unwrap_or(false)
            } else {
                true
            }
        })
        .collect();

    for edge in &incoming_edges {
        let from_port = edge.from_output_port.as_ref().unwrap();
        let to_port = edge.to_input_port.as_ref().unwrap();

        // Get source envelope
        let envelope = completed_envelopes.get(&edge.from_step_id).ok_or_else(|| {
            PortResolutionError::SourceStepNotCompleted {
                from_step_id: edge.from_step_id,
                port_name: from_port.clone(),
            }
        })?;

        // Find output port definition for json_path
        let source_outputs = source_outputs_map.get(&edge.from_step_id);
        let output_port =
            source_outputs.and_then(|outputs| outputs.iter().find(|o| o.port_name == *from_port));

        // Extract data from envelope
        let extracted = if let Some(port_def) = output_port {
            // Use the output port's json_path to extract from envelope.data
            if let Some(ref data) = envelope.data {
                resolve_dot_path(data, &port_def.json_path)
            } else {
                None
            }
        } else {
            // No output port definition — try using the port name as json_path
            if let Some(ref data) = envelope.data {
                resolve_dot_path(data, from_port)
            } else {
                None
            }
        };

        let mut value = extracted.ok_or_else(|| {
            let json_path = output_port
                .map(|p| p.json_path.clone())
                .unwrap_or_else(|| from_port.clone());
            PortResolutionError::DataExtractionFailed {
                step_id: edge.from_step_id,
                port_name: from_port.clone(),
                json_path,
            }
        })?;

        // Apply edge transform if present
        if let Some(ref transform) = edge.transform_jsonpath {
            if let Some(transformed) = resolve_dot_path(&value, transform) {
                value = transformed;
            }
        }

        resolved.insert(to_port.clone(), value);
    }

    // Fill defaults for missing optional inputs, error on missing required
    for input in step_inputs {
        if resolved.contains_key(&input.port_name) {
            continue;
        }
        if let Some(ref default_val) = input.default_value {
            resolved.insert(input.port_name.clone(), default_val.clone());
        } else if input.required {
            return Err(PortResolutionError::MissingRequiredInput {
                port_name: input.port_name.clone(),
                step_id,
            });
        }
    }

    Ok(resolved)
}

/// Collect data from upstream context-mode steps connected by bare (portless) edges.
///
/// Context steps store their `prompt_template` as a `JsonValue::String` in the
/// envelope's `data` field. This function finds all incoming bare edges (no port
/// names) from completed context steps and returns their data as `(title, content)`.
///
/// Complements `resolve_port_inputs()` which handles port-wired edges.
pub fn collect_upstream_context_data(
    step_id: Uuid,
    edges: &[WorkflowStepEdgeRow],
    steps: &[WorkflowStepRow],
    completed_envelopes: &HashMap<Uuid, StepExecutionEnvelope>,
) -> Vec<(String, String)> {
    let step_map: HashMap<Uuid, &WorkflowStepRow> = steps.iter().map(|s| (s.id, s)).collect();
    let mut results = Vec::new();

    for edge in edges {
        if edge.to_step_id != step_id {
            continue;
        }
        // Skip port-wired edges — those are handled by resolve_port_inputs
        if edge.from_output_port.is_some() || edge.to_input_port.is_some() {
            continue;
        }
        // Only collect from context-mode steps
        let source_step = match step_map.get(&edge.from_step_id) {
            Some(s) if s.execution_mode == "context" => s,
            _ => continue,
        };
        // Extract string data from the completed envelope
        if let Some(envelope) = completed_envelopes.get(&edge.from_step_id) {
            if let Some(JsonValue::String(content)) = &envelope.data {
                let title = source_step
                    .name
                    .clone()
                    .unwrap_or_else(|| "Upstream Context".to_string());
                results.push((title, content.clone()));
            }
        }
    }

    results
}
