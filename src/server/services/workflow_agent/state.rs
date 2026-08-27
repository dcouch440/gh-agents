//! Current state builder — produces `<current_state>` XML that is attached to
//! the workflow agent's user message, and only when the board has changed.
//!
//! Reads from the DB (not filesystem) because status and agent info are
//! DB-derived. The filesystem only has topology and descriptions that the
//! agent already knows about.
//!
//! Status resolution itself lives in `services::workflow_state`, shared with the
//! live-state endpoint so the agent and the UI never disagree about what a node
//! is doing.

use std::collections::HashMap;

use uuid::Uuid;

use crate::db::traits::WorkflowRepo;
use crate::db::WorkflowStepRow;
use crate::server::services::workflow_state;
use crate::server::services::ServiceError;
use crate::server::state::AppState;

// Re-exported so `state_tests.rs` keeps its import path after the extraction.
pub(crate) use crate::server::services::workflow_state::resolve_node_status;

#[cfg(test)]
#[path = "state_tests.rs"]
mod tests;

// ── Public API ─────────────────────────────────────────────────────────────

/// Build the `<current_state>` XML from the workflow's DB state.
///
/// Produces a compact summary of the board topology with per-node metadata.
/// Attached to the turn's user message when it differs from the last one sent,
/// so the agent sees fresh board state regardless of conversation history
/// compression while the system prompt stays static and cacheable.
pub(crate) async fn build_current_state(
    workflow_id: Uuid,
    state: &AppState,
) -> Result<String, ServiceError> {
    let inputs = workflow_state::collect(state, workflow_id).await?;

    // The XML only describes workforce nodes; the collector deliberately holds
    // every step so the live-state endpoint can describe the whole board.
    let workforce_steps: Vec<&WorkflowStepRow> = inputs
        .steps
        .iter()
        .filter(|s| s.execution_mode == "workforce")
        .collect();

    if workforce_steps.is_empty() {
        return Ok(
            "<current_state refresh=\"sent with your message when the board changed\">\n  \
             <topology status=\"empty\" />\n\
             </current_state>"
                .to_string(),
        );
    }

    // Build step_id → slug lookup
    let id_to_slug: HashMap<Uuid, &str> = workforce_steps
        .iter()
        .filter_map(|s| s.ref_id.as_deref().map(|r| (s.id, r)))
        .collect();

    // Build per-node depends_on from edges
    let mut depends_on_map: HashMap<Uuid, Vec<&str>> = HashMap::new();
    for edge in &inputs.edges {
        if let Some(&from_slug) = id_to_slug.get(&edge.from_step_id) {
            depends_on_map
                .entry(edge.to_step_id)
                .or_default()
                .push(from_slug);
        }
    }

    let wf_repo = &*state.repos().workflows;

    // Build roster summaries for configured steps
    let agent_summaries = build_agent_summaries(&workforce_steps, wf_repo).await;

    // Render XML
    let mut lines = Vec::new();
    lines.push(
        "<current_state refresh=\"sent with your message when the board changed\">".to_string(),
    );
    lines.push("  <topology>".to_string());

    // Sort by display_order for deterministic output
    let mut sorted_steps = workforce_steps.clone();
    sorted_steps.sort_by_key(|s| s.display_order);

    for step in &sorted_steps {
        let slug = step.ref_id.as_deref().unwrap_or("unknown");

        // Resolve real-time status from the batched collector — no per-step query.
        let active_tasks = inputs
            .registry_tasks_by_step
            .get(&step.id)
            .map(|v| v.as_slice())
            .unwrap_or(&[]);
        let latest_dispatch = inputs.latest_dispatch_by_step.get(&step.id);
        let is_running = inputs.running_step_ids.contains(&step.id);
        let status = resolve_node_status(step, active_tasks, latest_dispatch, is_running);

        let deps = depends_on_map
            .get(&step.id)
            .map(|d| d.join(", "))
            .unwrap_or_default();

        let mut attrs = format!("slug=\"{slug}\"");

        if let Some(name) = &step.name {
            if !name.is_empty() {
                attrs.push_str(&format!(" name=\"{name}\""));
            }
        }

        attrs.push_str(&format!(" depends_on=\"{deps}\""));
        attrs.push_str(&format!(" status=\"{status}\""));

        if let Some(summary) = agent_summaries.get(&step.id) {
            attrs.push_str(&format!(" agents=\"{summary}\""));
        }

        lines.push(format!("    <node {attrs} />"));
    }

    lines.push("  </topology>".to_string());
    lines.push("</current_state>".to_string());

    Ok(lines.join("\n"))
}

// ── Agent summary builder ──────────────────────────────────────────────────

/// Build pipeline summary strings for steps that have agent rosters.
///
/// Returns step_id → summary like "(Scanner, Crawler) → Analyzer".
async fn build_agent_summaries(
    steps: &[&WorkflowStepRow],
    repo: &dyn WorkflowRepo,
) -> HashMap<Uuid, String> {
    let mut summaries = HashMap::new();

    for step in steps {
        let child_wf_id = match step.child_workflow_id {
            Some(id) => id,
            None => continue,
        };

        // Load child steps and edges to determine execution order
        let child_steps = match repo.list_steps(child_wf_id).await {
            Ok(s) => s,
            Err(_) => continue,
        };
        let child_edges = match repo.list_edges(child_wf_id).await {
            Ok(e) => e,
            Err(_) => continue,
        };

        if child_steps.is_empty() {
            continue;
        }

        let summary = format_pipeline_summary(&child_steps, &child_edges);
        if !summary.is_empty() {
            summaries.insert(step.id, summary);
        }
    }

    summaries
}

/// Format child steps into a pipeline summary string.
///
/// Groups by topological level. Parallel agents in parens, sequential with arrows.
/// Example: "(Scanner, Crawler) → Analyzer → Reporter"
pub(crate) fn format_pipeline_summary(
    steps: &[WorkflowStepRow],
    edges: &[crate::db::WorkflowStepEdgeRow],
) -> String {
    use std::collections::HashSet;

    let step_ids: HashSet<Uuid> = steps.iter().map(|s| s.id).collect();

    // Build in-degree
    let mut in_degree: HashMap<Uuid, usize> = steps.iter().map(|s| (s.id, 0)).collect();
    let mut adjacency: HashMap<Uuid, Vec<Uuid>> =
        steps.iter().map(|s| (s.id, Vec::new())).collect();

    for edge in edges {
        if step_ids.contains(&edge.from_step_id) && step_ids.contains(&edge.to_step_id) {
            adjacency
                .entry(edge.from_step_id)
                .or_default()
                .push(edge.to_step_id);
            *in_degree.entry(edge.to_step_id).or_default() += 1;
        }
    }

    // BFS levels
    let mut levels: Vec<Vec<Uuid>> = Vec::new();
    let mut queue: Vec<Uuid> = in_degree
        .iter()
        .filter(|(_, &deg)| deg == 0)
        .map(|(&id, _)| id)
        .collect();
    queue.sort();

    while !queue.is_empty() {
        levels.push(queue.clone());
        let mut next = Vec::new();
        for &node in &queue {
            if let Some(neighbors) = adjacency.get(&node) {
                for &n in neighbors {
                    if let Some(deg) = in_degree.get_mut(&n) {
                        *deg -= 1;
                        if *deg == 0 {
                            next.push(n);
                        }
                    }
                }
            }
        }
        next.sort();
        queue = next;
    }

    // Build name lookup
    let id_to_name: HashMap<Uuid, &str> = steps
        .iter()
        .map(|s| {
            let name = s
                .name
                .as_deref()
                .unwrap_or_else(|| s.ref_id.as_deref().unwrap_or("?"));
            (s.id, name)
        })
        .collect();

    // Format levels
    let level_strings: Vec<String> = levels
        .iter()
        .map(|level| {
            let names: Vec<&str> = level
                .iter()
                .filter_map(|id| id_to_name.get(id).copied())
                .collect();
            if names.len() == 1 {
                names[0].to_string()
            } else {
                format!("({})", names.join(", "))
            }
        })
        .collect();

    level_strings.join(" \u{2192} ")
}
