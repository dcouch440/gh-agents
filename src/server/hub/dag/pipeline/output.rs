//! Pure helper functions for workforce output composition and agent scheduling.

use std::collections::{HashMap, HashSet};

use serde_json::Value as JsonValue;
use uuid::Uuid;

use crate::db::WorkflowStepRow;
use crate::types::StepExecutionEnvelope;

use super::super::agent_designer::normalize_agent_name;
use super::super::DagContext;
use super::types::DesignedAgentPrompt;

/// Compose workforce output: agent results keyed by normalized name.
pub(crate) fn compose_workforce_output(agent_outputs: &[(String, String)]) -> JsonValue {
    let mut composite = serde_json::Map::new();

    let mut agents = serde_json::Map::new();
    for (name, output) in agent_outputs {
        let key = name.to_lowercase().replace(' ', "_");
        let value: JsonValue =
            serde_json::from_str(output).unwrap_or_else(|_| JsonValue::String(output.clone()));
        agents.insert(key, value);
    }
    composite.insert("agents".to_string(), JsonValue::Object(agents));

    JsonValue::Object(composite)
}

/// Filter agent outputs based on receives_from routing.
pub(crate) fn filter_outputs_for_agent<'a>(
    agent_outputs: &'a [(String, String)],
    receives_from: &[String],
) -> Vec<&'a (String, String)> {
    if receives_from.is_empty() {
        agent_outputs.iter().collect()
    } else {
        let normalized_receives: std::collections::HashSet<String> = receives_from
            .iter()
            .map(|n| normalize_agent_name(n))
            .collect();
        agent_outputs
            .iter()
            .filter(|(name, _)| normalized_receives.contains(&normalize_agent_name(name)))
            .collect()
    }
}

/// Build filtered outputs block for injection.
pub(crate) fn build_filtered_outputs_block(outputs: &[&(String, String)]) -> String {
    if outputs.is_empty() {
        "No previous agent outputs yet. You are the first agent to execute.".to_string()
    } else {
        outputs
            .iter()
            .map(|(name, output)| format!("### {}\n{}", name, output))
            .collect::<Vec<_>>()
            .join("\n\n")
    }
}

/// Group designed prompts into execution levels based on `receives_from`.
///
/// Level 0 = agents with no `receives_from` (roots).
/// Level N = agents whose `receives_from` agents are all in levels < N.
///
/// Returns `Vec<Vec<usize>>` where each inner vec contains indices into `prompts`.
/// Agents within the same level can execute in parallel.
pub(crate) fn compute_execution_levels(prompts: &[DesignedAgentPrompt]) -> Vec<Vec<usize>> {
    if prompts.is_empty() {
        return vec![];
    }

    // Build name -> index lookup (normalized)
    let name_to_idx: HashMap<String, usize> = prompts
        .iter()
        .enumerate()
        .map(|(i, p)| (normalize_agent_name(&p.agent_name), i))
        .collect();

    // Build in-degree from receives_from
    let mut in_degree = vec![0usize; prompts.len()];
    let mut dependents: Vec<Vec<usize>> = vec![vec![]; prompts.len()];

    for (i, prompt) in prompts.iter().enumerate() {
        for dep_name in &prompt.receives_from {
            if let Some(&dep_idx) = name_to_idx.get(&normalize_agent_name(dep_name)) {
                in_degree[i] += 1;
                dependents[dep_idx].push(i);
            }
        }
    }

    // BFS by levels (Kahn's with level tracking)
    let mut levels: Vec<Vec<usize>> = Vec::new();
    let mut current_level: Vec<usize> = (0..prompts.len()).filter(|&i| in_degree[i] == 0).collect();
    current_level.sort_by_key(|&i| prompts[i].execution_order);

    while !current_level.is_empty() {
        let mut next_level: Vec<usize> = Vec::new();
        for &idx in &current_level {
            for &dep_idx in &dependents[idx] {
                in_degree[dep_idx] -= 1;
                if in_degree[dep_idx] == 0 {
                    next_level.push(dep_idx);
                }
            }
        }
        levels.push(current_level);
        next_level.sort_by_key(|&i| prompts[i].execution_order);
        current_level = next_level;
    }

    levels
}

/// Build a formatted block of upstream DAG step outputs for injection into
/// workforce agent task prompts.
///
/// Filters out context-mode and input-mode steps (context nodes are already
/// handled by `user_notes_block`). For workforce envelopes, extracts individual
/// agent outputs from the `{"agents": {...}}` structure. For other step types,
/// renders the data as a string.
///
/// Returns an empty string if no qualifying upstream outputs exist.
pub(crate) fn build_upstream_outputs_block(
    envelopes: &HashMap<Uuid, StepExecutionEnvelope>,
    steps: &[WorkflowStepRow],
) -> String {
    if envelopes.is_empty() {
        return String::new();
    }

    let step_map: HashMap<Uuid, &WorkflowStepRow> = steps.iter().map(|s| (s.id, s)).collect();

    let mut sections: Vec<String> = Vec::new();

    for (step_id, env) in envelopes {
        // Skip context and input steps — already handled by user_notes_block
        if let Some(step) = step_map.get(step_id) {
            if step.execution_mode == "context" || step.execution_mode == "input" {
                continue;
            }
        }

        let data = match &env.data {
            Some(d) => d,
            None => continue,
        };

        // Use human-readable step name, falling back to output_variable_name
        let name = step_map
            .get(step_id)
            .and_then(|s| s.name.as_deref().or(s.output_variable_name.as_deref()))
            .unwrap_or("Upstream Step");

        let content = format_envelope_data(data);
        if content.is_empty() {
            continue;
        }

        let truncated = super::super::designer_input::truncate_for_context(&content, 4000);
        sections.push(format!("### {}\n{}", name, truncated));
    }

    if sections.is_empty() {
        return String::new();
    }

    sections.join("\n\n")
}

/// Compute upstream step output text for workforce agent `<previous_step>` injection.
///
/// Filters completed envelopes to only include steps with edges into the
/// target step (not the full DAG state), then formats them via
/// `build_upstream_outputs_block`. This ensures workshop reruns don't
/// accidentally include this step's own prior output.
pub(crate) fn build_upstream_step_output(
    dag: &DagContext<'_>,
    step: &WorkflowStepRow,
    completed_envelopes: &HashMap<Uuid, StepExecutionEnvelope>,
) -> String {
    let incoming = dag
        .port_meta
        .incoming_edges
        .get(&step.id)
        .map(|v| v.as_slice())
        .unwrap_or(&[]);
    let upstream_step_ids: HashSet<Uuid> = incoming.iter().map(|e| e.from_step_id).collect();
    let upstream_envelopes: HashMap<Uuid, StepExecutionEnvelope> = completed_envelopes
        .iter()
        .filter(|(id, _)| upstream_step_ids.contains(id))
        .map(|(id, env)| (*id, env.clone()))
        .collect();

    build_upstream_outputs_block(&upstream_envelopes, dag.steps)
}

/// Format envelope data for human-readable injection.
///
/// Workforce envelopes have `{"agents": {"name": "output"}}` — extract each
/// agent's output as a labeled section. Other formats are rendered as strings.
fn format_envelope_data(data: &JsonValue) -> String {
    // Workforce envelope: extract individual agent outputs
    if let Some(agents) = data.get("agents").and_then(|a| a.as_object()) {
        let mut parts: Vec<String> = Vec::new();
        for (name, value) in agents {
            let content = match value {
                JsonValue::String(s) => s.clone(),
                other => serde_json::to_string_pretty(other).unwrap_or_default(),
            };
            parts.push(format!("**{}**:\n{}", name, content));
        }
        return parts.join("\n\n");
    }

    // Non-workforce: render as string
    match data {
        JsonValue::String(s) => s.clone(),
        other => serde_json::to_string_pretty(other).unwrap_or_default(),
    }
}
