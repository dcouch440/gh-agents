//! Tool execution handlers for the workforce archetype.
//!
//! Each agent becomes a pipeline step in a child workflow attached to the
//! workforce node. A Designer step is auto-managed via the pipeline service.
//! Workforce tools orchestrate: roster management (workforce-specific) +
//! pipeline CRUD (delegated to the pipeline service).

use std::collections::{HashMap, HashSet};

use serde_json::{json, Value};
use uuid::Uuid;

use crate::db::traits::WorkflowRepo;
use crate::db::TaskMissionBriefRow;
use crate::server::services::pipeline::PipelineContext;

mod agents;
mod configure;
mod dependencies;
mod tests;

/// Ambient context for workforce tool execution.
pub struct WorkforceToolContext {
    pub workflow_id: Uuid,
    pub step_id: Uuid,
}

/// Execute a workforce tool by name.
pub async fn execute_workforce_tool(
    name: &str,
    input: &Value,
    repo: &dyn WorkflowRepo,
    ctx: &WorkforceToolContext,
) -> Value {
    match name {
        "configure_team" => configure::execute_configure_team(input, repo, ctx).await,
        "set_task" => agents::execute_set_task(input, repo, ctx).await,
        "add_agent" => agents::execute_add_agent(input, repo, ctx).await,
        "update_agent" => agents::execute_update_agent(input, repo, ctx).await,
        "remove_agent" => agents::execute_remove_agent(input, repo, ctx).await,
        "set_capabilities" => agents::execute_set_capabilities(input, repo, ctx).await,
        "set_dependency" => dependencies::execute_set_dependency(input, repo, ctx).await,
        "remove_dependency" => dependencies::execute_remove_dependency(input, repo, ctx).await,
        _ => json!({ "error": format!("Unknown workforce tool: {}", name) }),
    }
}

// =========================================================================
// Helpers (shared by sub-modules)
// =========================================================================

/// Build a `PipelineContext` from the workforce tool context.
pub(super) fn pipeline_ctx(ctx: &WorkforceToolContext) -> PipelineContext {
    PipelineContext {
        parent_step_id: ctx.step_id,
        parent_workflow_id: ctx.workflow_id,
    }
}

/// Resolve user_id from the parent workflow.
pub(super) async fn resolve_user_id(
    repo: &dyn WorkflowRepo,
    ctx: &WorkforceToolContext,
) -> Result<Uuid, Value> {
    let workflow = repo
        .get_workflow(ctx.workflow_id)
        .await
        .map_err(|e| json!({ "error": format!("Failed to load workflow: {}", e) }))?
        .ok_or_else(|| json!({ "error": "Parent workflow not found" }))?;
    Ok(workflow.user_id)
}

/// Ensure a mission brief exists for this step, creating one if needed.
pub(super) async fn ensure_mission_brief(
    repo: &dyn WorkflowRepo,
    ctx: &WorkforceToolContext,
) -> Result<Uuid, Value> {
    match repo.get_mission_brief(ctx.step_id).await {
        Ok(Some(brief)) => Ok(brief.id),
        Ok(None) => match repo
            .upsert_mission_brief(ctx.step_id, "", &[], "fail_fast", None)
            .await
        {
            Ok(brief) => Ok(brief.id),
            Err(e) => Err(json!({ "error": format!("Failed to create mission brief: {}", e) })),
        },
        Err(e) => Err(json!({ "error": format!("Failed to load mission brief: {}", e) })),
    }
}

/// Read-preserve-write helper for mission brief fields.
pub(super) async fn upsert_mission_brief_field(
    repo: &dyn WorkflowRepo,
    step_id: Uuid,
    task_description: Option<&str>,
    available_capabilities: Option<&[String]>,
    failure_mode: Option<&str>,
    downstream_context: Option<Option<String>>,
) -> Result<TaskMissionBriefRow, String> {
    let existing = repo.get_mission_brief(step_id).await.ok().flatten();

    let desc = task_description.map(String::from).unwrap_or_else(|| {
        existing
            .as_ref()
            .map_or(String::new(), |b| b.task_description.clone())
    });
    let caps = available_capabilities
        .map(|c| c.to_vec())
        .unwrap_or_else(|| {
            existing
                .as_ref()
                .map_or(vec![], |b| b.available_capabilities.clone())
        });
    let fm = failure_mode.map(String::from).unwrap_or_else(|| {
        existing
            .as_ref()
            .map_or("fail_fast".to_string(), |b| b.failure_mode.clone())
    });
    let dc = downstream_context
        .unwrap_or_else(|| existing.as_ref().and_then(|b| b.downstream_context.clone()));

    repo.upsert_mission_brief(step_id, &desc, &caps, &fm, dc)
        .await
        .map_err(|e| e.to_string())
}

/// Normalize an agent name for matching (case-insensitive, strip separators).
pub(super) fn normalize_name(name: &str) -> String {
    name.chars()
        .filter(|c| *c != ' ' && *c != '_' && *c != '-')
        .flat_map(|c| c.to_lowercase())
        .collect()
}

/// Find a roster agent by normalized name match.
pub(super) fn find_agent_by_name<'a>(
    roster: &'a [crate::db::TaskAgentRosterRow],
    name: &str,
) -> Option<&'a crate::db::TaskAgentRosterRow> {
    let normalized = normalize_name(name);
    roster
        .iter()
        .find(|a| normalize_name(&a.name) == normalized)
}

/// Build a detailed error when an agent name isn't found in the roster.
///
/// Lists available agents and reminds the builder of its scope — it can only
/// reference agents within this node, not other workflow nodes.
pub(super) fn agent_not_found_error(name: &str, roster: &[crate::db::TaskAgentRosterRow]) -> Value {
    let available: Vec<&str> = roster.iter().map(|a| a.name.as_str()).collect();
    let available_str = if available.is_empty() {
        "No agents configured yet — use configure_team or add_agent first.".to_string()
    } else {
        format!("Available agents in this node: {}", available.join(", "))
    };
    json!({
        "error": format!(
            "Agent '{}' not found in this node's roster. {} \
             You can only reference agents within this node — \
             other workflow nodes and the manager are outside your scope.",
            name, available_str
        )
    })
}

/// Resolve an agent by ID or name.
///
/// Tries `agent_id` (UUID) first, falls back to `name` lookup via
/// `find_agent_by_name`. Returns an actionable error if neither matches.
pub(super) async fn resolve_agent_id(
    input: &Value,
    repo: &dyn WorkflowRepo,
    ctx: &WorkforceToolContext,
) -> Result<Uuid, Value> {
    // Try UUID first
    if let Some(id_str) = input["agent_id"].as_str() {
        if let Ok(id) = Uuid::parse_str(id_str) {
            return Ok(id);
        }
    }

    // Fall back to name lookup
    if let Some(name) = input["name"].as_str() {
        let brief = repo
            .get_mission_brief(ctx.step_id)
            .await
            .map_err(|e| json!({ "error": e.to_string() }))?
            .ok_or_else(|| json!({ "error": "No mission brief found" }))?;
        let roster = repo.list_agent_roster(brief.id).await.unwrap_or_default();
        if let Some(agent) = find_agent_by_name(&roster, name) {
            return Ok(agent.id);
        }
        return Err(agent_not_found_error(name, &roster));
    }

    Err(json!({ "error": "Provide either agent_id or name to identify the agent" }))
}

/// Recompute execution_order for all roster agents from the dependency graph.
///
/// Uses Kahn's algorithm with a min-heap (tie-break by current execution_order
/// for stability). Updates the DB and returns the ordered agent list for
/// inclusion in tool responses.
pub(super) async fn recompute_execution_order(
    repo: &dyn WorkflowRepo,
    ctx: &WorkforceToolContext,
) -> Result<Vec<Value>, Value> {
    use std::cmp::Reverse;
    use std::collections::BinaryHeap;

    let brief = match repo.get_mission_brief(ctx.step_id).await {
        Ok(Some(b)) => b,
        Ok(None) => return Ok(vec![]),
        Err(e) => return Err(json!({ "error": e.to_string() })),
    };
    let roster = repo
        .list_agent_roster(brief.id)
        .await
        .map_err(|e| json!({ "error": e.to_string() }))?;
    if roster.is_empty() {
        return Ok(vec![]);
    }

    let step = match repo.get_step(ctx.step_id).await {
        Ok(Some(s)) => s,
        Ok(None) => {
            return Ok(roster
                .iter()
                .map(|a| json!({"name": a.name, "order": a.execution_order}))
                .collect())
        }
        Err(e) => return Err(json!({ "error": e.to_string() })),
    };
    let child_workflow_id = match step.child_workflow_id {
        Some(id) => id,
        None => {
            return Ok(roster
                .iter()
                .map(|a| json!({"name": a.name, "order": a.execution_order}))
                .collect())
        }
    };

    let edges = repo.list_edges(child_workflow_id).await.unwrap_or_default();

    // Build child_step_id → roster index lookup
    let step_to_roster: HashMap<Uuid, usize> = roster
        .iter()
        .enumerate()
        .filter_map(|(i, a)| a.child_step_id.map(|sid| (sid, i)))
        .collect();

    // Filter to agent-to-agent edges only (exclude Designer → agent edges)
    let agent_step_ids: HashSet<Uuid> = step_to_roster.keys().copied().collect();
    let mut in_degree = vec![0usize; roster.len()];
    let mut dependents: Vec<Vec<usize>> = vec![vec![]; roster.len()];

    for edge in &edges {
        if let (Some(&from_ri), Some(&to_ri)) = (
            step_to_roster.get(&edge.from_step_id),
            step_to_roster.get(&edge.to_step_id),
        ) {
            if agent_step_ids.contains(&edge.from_step_id)
                && agent_step_ids.contains(&edge.to_step_id)
            {
                in_degree[to_ri] += 1;
                dependents[from_ri].push(to_ri);
            }
        }
    }

    // Kahn's with min-heap (tie-break by current execution_order for stability)
    let mut heap: BinaryHeap<Reverse<(i32, usize)>> = BinaryHeap::new();
    for (i, &deg) in in_degree.iter().enumerate() {
        if deg == 0 {
            heap.push(Reverse((roster[i].execution_order, i)));
        }
    }

    let mut sorted: Vec<usize> = Vec::with_capacity(roster.len());
    while let Some(Reverse((_, ri))) = heap.pop() {
        sorted.push(ri);
        for &dep_ri in &dependents[ri] {
            in_degree[dep_ri] -= 1;
            if in_degree[dep_ri] == 0 {
                heap.push(Reverse((roster[dep_ri].execution_order, dep_ri)));
            }
        }
    }

    // Update DB for agents whose order changed
    let mut result = Vec::with_capacity(sorted.len());
    for (new_order, &ri) in sorted.iter().enumerate() {
        let agent = &roster[ri];
        let new_order_i32 = new_order as i32;
        if agent.execution_order != new_order_i32 {
            let _ = repo
                .update_roster_agent_order(agent.id, new_order_i32)
                .await;
        }
        result.push(json!({"name": agent.name, "order": new_order_i32}));
    }

    // Include any agents not in sorted (shouldn't happen, but safety)
    let sorted_set: HashSet<usize> = sorted.iter().copied().collect();
    for (i, agent) in roster.iter().enumerate() {
        if !sorted_set.contains(&i) {
            result.push(json!({"name": agent.name, "order": agent.execution_order}));
        }
    }

    Ok(result)
}
