//! Pure helper functions for workforce output composition and agent scheduling.

use std::collections::HashMap;

use serde_json::Value as JsonValue;
use uuid::Uuid;

use crate::db::traits::SystemFileRepo;
use crate::db::{SystemFileRow, WorkflowStepRow};

use super::super::agent_designer::normalize_agent_name;
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

/// Build a team roster string for fallback prompts.
pub(crate) fn build_team_roster_string(roster: &[crate::db::TaskAgentRosterRow]) -> String {
    roster
        .iter()
        .map(|a| {
            let caps = if a.capabilities.is_empty() {
                String::new()
            } else {
                format!(" [{}]", a.capabilities.join(", "))
            };

            format!(
                "- **{}** (order {}): {}{}",
                a.name, a.execution_order, a.role_description, caps
            )
        })
        .collect::<Vec<_>>()
        .join("\n")
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

/// Build `<upstream_artifacts>` XML manifest from store metadata.
///
/// Lists files scoped to the current run:
/// - Workforce-local: files from agents in the same step (this run only)
/// - Upstream DAG: files from upstream connected steps (this run only)
///
/// When `run_id` is `None`, returns all files (no run scoping).
/// Returns an empty string if no artifacts exist.
pub(crate) async fn build_upstream_artifacts_block(
    repo: &dyn SystemFileRepo,
    workflow_id: Uuid,
    step_id: Uuid,
    upstream_step_ids: &[Uuid],
    steps: &[WorkflowStepRow],
    run_id: Option<Uuid>,
) -> String {
    let step_name_map: HashMap<Uuid, &str> = steps
        .iter()
        .map(|s| (s.id, s.name.as_deref().unwrap_or("Unnamed")))
        .collect();

    let mut sections: Vec<String> = Vec::new();

    // Workforce-local files (agents in this step, this run only)
    if let Ok(local_files) = repo.list_by_producer(workflow_id, step_id, run_id).await {
        let local_xml = format_artifact_section(
            step_name_map
                .get(&step_id)
                .copied()
                .unwrap_or("Current Step"),
            &local_files,
        );
        if !local_xml.is_empty() {
            sections.push(local_xml);
        }
    }

    // Upstream DAG step files (direct edges, this run only)
    for &upstream_id in upstream_step_ids {
        if let Ok(files) = repo
            .list_by_producer(workflow_id, upstream_id, run_id)
            .await
        {
            let name = step_name_map
                .get(&upstream_id)
                .copied()
                .unwrap_or("Upstream Step");
            let section = format_artifact_section(name, &files);
            if !section.is_empty() {
                sections.push(section);
            }
        }
    }

    if sections.is_empty() {
        return String::new();
    }

    format!(
        "<upstream_artifacts>\n{}\n</upstream_artifacts>",
        sections.join("\n")
    )
}

/// Format a single step's artifacts as XML entries.
fn format_artifact_section(step_name: &str, files: &[SystemFileRow]) -> String {
    // Filter out design/ files — those are internal to the designer, not artifacts
    let artifact_files: Vec<&SystemFileRow> = files
        .iter()
        .filter(|f| !f.path.starts_with("design/"))
        .collect();

    if artifact_files.is_empty() {
        return String::new();
    }

    let mut out = format!("  <step name=\"{step_name}\">\n");
    for file in &artifact_files {
        let by = file.produced_by_agent.as_deref().unwrap_or("unknown");
        let editable = if file.sealed {
            " editable=\"false\""
        } else {
            ""
        };
        out.push_str(&format!(
            "    <file path=\".system/{}\" type=\"{}\" by=\"{}\"{editable}>\n      {}\n    </file>\n",
            file.path, file.media_type, by, file.description
        ));
    }
    out.push_str("  </step>");
    out
}
