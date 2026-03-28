//! Current state builder — produces `<current_state>` XML for injection
//! into the workflow agent's system prompt between turns.
//!
//! Reads from the DB (not filesystem) because status and agent info are
//! DB-derived. The filesystem only has topology and descriptions that the
//! agent already knows about.
//!
//! Status derivation uses two sources:
//! - **TaskRegistry** (in-memory) — active dispatch tasks → "configuring"
//! - **agent_executions** (DB) — latest dispatch result → "error" on failure

use std::collections::HashMap;

use uuid::Uuid;

use crate::db::traits::WorkflowRepo;
use crate::db::{AgentExecutionRow, WorkflowStepRow};
use crate::server::services::ServiceError;
use crate::server::state::task_registry::{TaskEntry, TaskStatus};
use crate::server::state::AppState;

#[cfg(test)]
#[path = "state_tests.rs"]
mod tests;

// ── Public API ─────────────────────────────────────────────────────────────

/// Build the `<current_state>` XML from the workflow's DB state.
///
/// Produces a compact summary of the board topology with per-node metadata.
/// Injected into the system prompt between turns so the agent always sees
/// fresh board state regardless of conversation history compression.
pub(crate) async fn build_current_state(
    workflow_id: Uuid,
    state: &AppState,
) -> Result<String, ServiceError> {
    let wf_repo = &*state.repos().workflows;
    let ae_repo = &*state.repos().agent_executions;

    let steps = wf_repo
        .list_steps(workflow_id)
        .await
        .map_err(ServiceError::Internal)?;
    let edges = wf_repo
        .list_edges(workflow_id)
        .await
        .map_err(ServiceError::Internal)?;

    // Filter to workforce steps
    let workforce_steps: Vec<&WorkflowStepRow> = steps
        .iter()
        .filter(|s| s.execution_mode == "workforce")
        .collect();

    if workforce_steps.is_empty() {
        return Ok(
            "<current_state refresh=\"every turn — always reflects the current board\">\n  \
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
    for edge in &edges {
        if let Some(&from_slug) = id_to_slug.get(&edge.from_step_id) {
            depends_on_map
                .entry(edge.to_step_id)
                .or_default()
                .push(from_slug);
        }
    }

    // Batch: latest dispatch execution per step
    let step_ids: Vec<Uuid> = workforce_steps.iter().map(|s| s.id).collect();
    let latest_dispatches = ae_repo
        .get_latest_dispatch_executions_for_steps(&step_ids)
        .await
        .map_err(ServiceError::Internal)?;

    let dispatch_by_step: HashMap<Uuid, &AgentExecutionRow> = latest_dispatches
        .iter()
        .filter_map(|ae| ae.workflow_step_id.map(|sid| (sid, ae)))
        .collect();

    // Build roster summaries for configured steps
    let agent_summaries = build_agent_summaries(&workforce_steps, wf_repo).await;

    // Render XML
    let mut lines = Vec::new();
    lines.push(
        "<current_state refresh=\"every turn — always reflects the current board\">".to_string(),
    );
    lines.push("  <topology>".to_string());

    // Sort by display_order for deterministic output
    let mut sorted_steps = workforce_steps.clone();
    sorted_steps.sort_by_key(|s| s.display_order);

    for step in &sorted_steps {
        let slug = step.ref_id.as_deref().unwrap_or("unknown");

        // Resolve real-time status
        let active_tasks = state.task_registry().list_tasks_for_step(step.id);
        let latest_dispatch = dispatch_by_step.get(&step.id).copied();
        let status = resolve_node_status(step, &active_tasks, latest_dispatch);

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

// ── Status resolution ──────────────────────────────────────────────────────

/// Resolve real-time status for a workflow node.
///
/// Priority (highest wins):
/// 1. `configuring` — TaskRegistry has a Running task for this step
/// 2. `error` — latest dispatch execution failed
/// 3. `completed` — pinned or has run_results_summary
/// 4. `configured` — has child_workflow_id (system node agent completed)
/// 5. `described` — has description
/// 6. `idle` — default
///
/// Pure function — takes pre-fetched data, no DB access.
pub(crate) fn resolve_node_status(
    step: &WorkflowStepRow,
    active_tasks: &[TaskEntry],
    latest_dispatch: Option<&AgentExecutionRow>,
) -> &'static str {
    // 1. Active dispatch task → configuring
    if active_tasks.iter().any(|t| t.status == TaskStatus::Running) {
        return "configuring";
    }

    // 2. Latest dispatch failed → error
    if let Some(dispatch) = latest_dispatch {
        if dispatch.status == "failed" {
            return "error";
        }
    }

    // 3. Completed
    if step.pinned || !step.run_results_summary.is_empty() {
        return "completed";
    }

    // 4. Configured (system node agent ran successfully)
    if step.child_workflow_id.is_some() {
        return "configured";
    }

    // 5. Has description
    if !step.description.is_empty() {
        return "described";
    }

    // 6. Default
    "idle"
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
