//! Tool execution handlers for the workforce archetype.
//!
//! Each agent becomes a pipeline step in a child workflow attached to the
//! workforce node. A Designer step is auto-managed via the pipeline service.
//! Workforce tools orchestrate: roster management (workforce-specific) +
//! pipeline CRUD (delegated to the pipeline service).

use std::collections::{HashMap, HashSet};

use chrono::Utc;
use serde_json::{json, Value};
use uuid::Uuid;

use crate::db::traits::WorkflowRepo;
use crate::db::TaskMissionBriefRow;
use crate::server::services::pipeline::{self, AddStepInput, PipelineContext, UpdateStepInput};
use crate::server::tools::shared::{require_array, require_str, require_uuid};

mod tests;

/// Ambient context for workforce tool execution.
pub struct WorkforceToolContext {
    pub workflow_id: Uuid,
    pub step_id: Uuid,
}

/// Valid failure mode values.
const VALID_FAILURE_MODES: &[&str] = &["fail_fast", "skip_and_continue", "retry"];

/// Execute a workforce tool by name.
pub async fn execute_workforce_tool(
    name: &str,
    input: &Value,
    repo: &dyn WorkflowRepo,
    ctx: &WorkforceToolContext,
) -> Value {
    match name {
        "set_task" => execute_set_task(input, repo, ctx).await,
        "add_agent" => execute_add_agent(input, repo, ctx).await,
        "update_agent" => execute_update_agent(input, repo, ctx).await,
        "remove_agent" => execute_remove_agent(input, repo, ctx).await,
        "set_capabilities" => execute_set_capabilities(input, repo, ctx).await,
        "set_failure_mode" => execute_set_failure_mode(input, repo, ctx).await,
        "add_deliverable" => execute_add_deliverable(input, repo, ctx).await,
        "update_deliverable" => execute_update_deliverable(input, repo, ctx).await,
        "remove_deliverable" => execute_remove_deliverable(input, repo, ctx).await,
        "set_dependency" => execute_set_dependency(input, repo, ctx).await,
        "remove_dependency" => execute_remove_dependency(input, repo, ctx).await,
        _ => json!({ "error": format!("Unknown workforce tool: {}", name) }),
    }
}

// =========================================================================
// Helpers
// =========================================================================

/// Build a `PipelineContext` from the workforce tool context.
fn pipeline_ctx(ctx: &WorkforceToolContext) -> PipelineContext {
    PipelineContext {
        parent_step_id: ctx.step_id,
        parent_workflow_id: ctx.workflow_id,
    }
}

/// Resolve user_id from the parent workflow.
async fn resolve_user_id(
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
async fn ensure_mission_brief(
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
async fn upsert_mission_brief_field(
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
fn normalize_name(name: &str) -> String {
    name.chars()
        .filter(|c| *c != ' ' && *c != '_' && *c != '-')
        .flat_map(|c| c.to_lowercase())
        .collect()
}

/// Find a roster agent by normalized name match.
fn find_agent_by_name<'a>(
    roster: &'a [crate::db::TaskAgentRosterRow],
    name: &str,
) -> Option<&'a crate::db::TaskAgentRosterRow> {
    let normalized = normalize_name(name);
    roster
        .iter()
        .find(|a| normalize_name(&a.name) == normalized)
}

/// Recompute execution_order for all roster agents from the dependency graph.
///
/// Uses Kahn's algorithm with a min-heap (tie-break by current execution_order
/// for stability). Updates the DB and returns the ordered agent list for
/// inclusion in tool responses.
async fn recompute_execution_order(
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

// =========================================================================
// Tool Handlers — Mission Brief & Roster
// =========================================================================

async fn execute_set_task(
    input: &Value,
    repo: &dyn WorkflowRepo,
    ctx: &WorkforceToolContext,
) -> Value {
    let description = match require_str(input, "description") {
        Ok(v) => v,
        Err(e) => return e,
    };

    match upsert_mission_brief_field(repo, ctx.step_id, Some(description), None, None, None).await {
        Ok(brief) => json!({
            "step_id": ctx.step_id.to_string(),
            "task_description": brief.task_description,
        }),
        Err(e) => json!({ "error": e }),
    }
}

async fn execute_add_agent(
    input: &Value,
    repo: &dyn WorkflowRepo,
    ctx: &WorkforceToolContext,
) -> Value {
    let name = match require_str(input, "name") {
        Ok(v) => v,
        Err(e) => return e,
    };

    let role = input["role"].as_str().unwrap_or("");
    let capabilities: Vec<String> = input["capabilities"]
        .as_array()
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str().map(String::from))
                .collect()
        })
        .unwrap_or_default();

    // 1. Ensure brief exists (workforce-specific)
    let brief_id = match ensure_mission_brief(repo, ctx).await {
        Ok(id) => id,
        Err(e) => return e,
    };

    // 2. Resolve user_id for pipeline creation
    let user_id = match resolve_user_id(repo, ctx).await {
        Ok(id) => id,
        Err(e) => return e,
    };

    // 3. Get existing roster for next_order
    let roster = repo.list_agent_roster(brief_id).await.unwrap_or_default();
    let next_order = roster.iter().map(|a| a.execution_order).max().unwrap_or(-1) + 1;

    // 4. Add step via pipeline service (handles: create pipeline + step)
    let pip_ctx = pipeline_ctx(ctx);
    let (step_added, _) = match pipeline::add_step(
        repo,
        &pip_ctx,
        user_id,
        AddStepInput {
            name: name.to_string(),
            description: role.to_string(),
            execution_mode: "single".to_string(),
            agent_id: None,
            prompt_template: None,
            output_variable_name: None,
            display_order: Some(next_order + 1),
        },
    )
    .await
    {
        Ok(result) => result,
        Err(e) => return json!({ "error": format!("Pipeline error: {}", e) }),
    };

    // 5. Create roster entry (workforce-specific)
    let roster_agent = match repo
        .add_roster_agent(brief_id, name, role, &capabilities, next_order)
        .await
    {
        Ok(agent) => agent,
        Err(e) => return json!({ "error": e.to_string() }),
    };

    // 6. Link roster entry to pipeline step (workforce-specific bridge)
    if let Err(e) = repo
        .link_roster_agent_to_child_step(roster_agent.id, Some(step_added.step_id))
        .await
    {
        return json!({ "error": format!("Failed to link roster to child step: {}", e) });
    }

    // 7. Recompute roster execution order (workforce-specific)
    let execution_sequence = recompute_execution_order(repo, ctx)
        .await
        .unwrap_or_default();

    json!({
        "agent_id": roster_agent.id.to_string(),
        "name": roster_agent.name,
        "role": roster_agent.role_description,
        "capabilities": roster_agent.capabilities,
        "child_step_id": step_added.step_id.to_string(),
        "execution_sequence": execution_sequence,
    })
}

async fn execute_update_agent(
    input: &Value,
    repo: &dyn WorkflowRepo,
    ctx: &WorkforceToolContext,
) -> Value {
    let agent_id = match require_uuid(input, "agent_id") {
        Ok(v) => v,
        Err(e) => return e,
    };

    let name = input["name"].as_str().map(String::from);
    let role = input["role"].as_str().map(String::from);
    let capabilities: Option<Vec<String>> = input["capabilities"].as_array().map(|arr| {
        arr.iter()
            .filter_map(|v| v.as_str().map(String::from))
            .collect()
    });

    let agent = match repo
        .update_roster_agent(agent_id, name.clone(), role.clone(), capabilities)
        .await
    {
        Ok(a) => a,
        Err(e) => return json!({ "error": e.to_string() }),
    };

    // Sync child step via pipeline service
    if let Some(child_step_id) = agent.child_step_id {
        if name.is_some() || role.is_some() {
            let pip_ctx = pipeline_ctx(ctx);
            let _ = pipeline::update_step(
                repo,
                &pip_ctx,
                child_step_id,
                UpdateStepInput {
                    name: name.clone(),
                    description: role.clone(),
                    ..Default::default()
                },
            )
            .await;
        }
    }

    json!({
        "agent_id": agent.id.to_string(),
        "name": agent.name,
        "role": agent.role_description,
        "capabilities": agent.capabilities,
    })
}

async fn execute_remove_agent(
    input: &Value,
    repo: &dyn WorkflowRepo,
    ctx: &WorkforceToolContext,
) -> Value {
    let agent_id = match require_uuid(input, "agent_id") {
        Ok(v) => v,
        Err(e) => return e,
    };

    // Load brief + roster to find the agent
    let brief = match repo.get_mission_brief(ctx.step_id).await {
        Ok(Some(b)) => b,
        Ok(None) => return json!({ "error": "No mission brief found" }),
        Err(e) => return json!({ "error": e.to_string() }),
    };

    let roster = repo.list_agent_roster(brief.id).await.unwrap_or_default();
    let target = match roster.iter().find(|a| a.id == agent_id) {
        Some(a) => a.clone(),
        None => return json!({ "error": "Agent not found in roster" }),
    };
    let agent_name = target.name.clone();

    // Remove the child step via pipeline service
    if let Some(child_step_id) = target.child_step_id {
        let pip_ctx = pipeline_ctx(ctx);
        if let Err(e) = pipeline::remove_step(repo, &pip_ctx, child_step_id).await {
            return json!({ "error": format!("Pipeline error: {}", e) });
        }
    }

    // Remove from roster
    if let Err(e) = repo.remove_roster_agent(agent_id).await {
        return json!({ "error": e.to_string() });
    }

    // Recompute roster execution order
    let execution_sequence = recompute_execution_order(repo, ctx)
        .await
        .unwrap_or_default();

    json!({
        "deleted": true,
        "name": agent_name,
        "id": agent_id.to_string(),
        "execution_sequence": execution_sequence,
    })
}

async fn execute_set_capabilities(
    input: &Value,
    repo: &dyn WorkflowRepo,
    ctx: &WorkforceToolContext,
) -> Value {
    let caps_arr = match require_array(input, "capabilities") {
        Ok(v) => v,
        Err(e) => return e,
    };
    let capabilities: Vec<String> = caps_arr
        .iter()
        .filter_map(|v| v.as_str().map(String::from))
        .collect();

    match upsert_mission_brief_field(repo, ctx.step_id, None, Some(&capabilities), None, None).await
    {
        Ok(brief) => json!({
            "capabilities": brief.available_capabilities,
        }),
        Err(e) => json!({ "error": e }),
    }
}

async fn execute_set_failure_mode(
    input: &Value,
    repo: &dyn WorkflowRepo,
    ctx: &WorkforceToolContext,
) -> Value {
    let mode = match require_str(input, "mode") {
        Ok(v) => v,
        Err(e) => return e,
    };

    if !VALID_FAILURE_MODES.contains(&mode) {
        return json!({
            "error": format!(
                "Invalid failure mode '{}'. Must be one of: {}",
                mode,
                VALID_FAILURE_MODES.join(", ")
            )
        });
    }

    match upsert_mission_brief_field(repo, ctx.step_id, None, None, Some(mode), None).await {
        Ok(brief) => json!({
            "failure_mode": brief.failure_mode,
        }),
        Err(e) => json!({ "error": e }),
    }
}

// =========================================================================
// Tool Handlers — Deliverables
// =========================================================================

async fn execute_add_deliverable(
    input: &Value,
    repo: &dyn WorkflowRepo,
    ctx: &WorkforceToolContext,
) -> Value {
    let name = match require_str(input, "name") {
        Ok(v) => v,
        Err(e) => return e,
    };

    let description = input["description"].as_str().unwrap_or("").to_string();
    let target_length = input["target_length"].as_i64().unwrap_or(1500) as i32;

    let agent_roster_entry_id = match input["agent_id"].as_str() {
        Some(id_str) => match Uuid::parse_str(id_str) {
            Ok(id) => Some(id),
            Err(_) => return json!({ "error": format!("Invalid agent UUID: {}", id_str) }),
        },
        None => None,
    };

    let def = crate::db::ProtocolDocumentDefRow {
        id: Uuid::new_v4(),
        step_id: Some(ctx.step_id),
        name: name.to_string(),
        description,
        target_length,
        display_order: 0,
        created_at: Utc::now(),
        protocol_id: None,
        document_id: None,
        agent_roster_entry_id,
    };

    match repo.create_document_def(def).await {
        Ok(row) => json!({
            "id": row.id.to_string(),
            "name": row.name,
            "description": row.description,
            "target_length": row.target_length,
            "agent_id": row.agent_roster_entry_id.map(|id| id.to_string()),
        }),
        Err(e) => json!({ "error": e.to_string() }),
    }
}

async fn execute_update_deliverable(
    input: &Value,
    repo: &dyn WorkflowRepo,
    ctx: &WorkforceToolContext,
) -> Value {
    let def_id = match require_uuid(input, "deliverable_id") {
        Ok(v) => v,
        Err(e) => return e,
    };

    let existing_defs = match repo.list_document_defs(ctx.step_id).await {
        Ok(defs) => defs,
        Err(e) => return json!({ "error": e.to_string() }),
    };

    let existing = match existing_defs.into_iter().find(|d| d.id == def_id) {
        Some(d) => d,
        None => return json!({ "error": "Deliverable not found" }),
    };

    let name = input["name"]
        .as_str()
        .map(String::from)
        .unwrap_or(existing.name);
    let description = input["description"]
        .as_str()
        .map(String::from)
        .unwrap_or(existing.description);
    let target_length = input["target_length"]
        .as_i64()
        .map(|v| v as i32)
        .unwrap_or(existing.target_length);

    match repo
        .update_document_def(def_id, name, description, target_length)
        .await
    {
        Ok(row) => json!({
            "id": row.id.to_string(),
            "name": row.name,
            "description": row.description,
            "target_length": row.target_length,
        }),
        Err(e) => json!({ "error": e.to_string() }),
    }
}

async fn execute_remove_deliverable(
    input: &Value,
    repo: &dyn WorkflowRepo,
    ctx: &WorkforceToolContext,
) -> Value {
    let def_id = match require_uuid(input, "deliverable_id") {
        Ok(v) => v,
        Err(e) => return e,
    };

    let def_name = repo
        .list_document_defs(ctx.step_id)
        .await
        .ok()
        .and_then(|defs| defs.into_iter().find(|d| d.id == def_id))
        .map(|d| d.name)
        .unwrap_or_default();

    match repo.delete_document_def(def_id).await {
        Ok(()) => json!({
            "deleted": true,
            "name": def_name,
            "id": def_id.to_string(),
        }),
        Err(e) => json!({ "error": e.to_string() }),
    }
}

// =========================================================================
// Tool Handlers — Agent Dependencies
// =========================================================================

async fn execute_set_dependency(
    input: &Value,
    repo: &dyn WorkflowRepo,
    ctx: &WorkforceToolContext,
) -> Value {
    let from_name = match require_str(input, "from_agent") {
        Ok(v) => v,
        Err(e) => return e,
    };
    let to_name = match require_str(input, "to_agent") {
        Ok(v) => v,
        Err(e) => return e,
    };

    // Load brief + roster
    let brief = match repo.get_mission_brief(ctx.step_id).await {
        Ok(Some(b)) => b,
        Ok(None) => return json!({ "error": "No mission brief found" }),
        Err(e) => return json!({ "error": e.to_string() }),
    };
    let roster = repo.list_agent_roster(brief.id).await.unwrap_or_default();

    // Resolve agent names → child step IDs
    let from_agent = match find_agent_by_name(&roster, from_name) {
        Some(a) => a,
        None => return json!({ "error": format!("Agent '{}' not found in roster", from_name) }),
    };
    let to_agent = match find_agent_by_name(&roster, to_name) {
        Some(a) => a,
        None => return json!({ "error": format!("Agent '{}' not found in roster", to_name) }),
    };

    if from_agent.id == to_agent.id {
        return json!({ "error": "Cannot create a dependency from an agent to itself" });
    }

    let from_child_step_id = match from_agent.child_step_id {
        Some(id) => id,
        None => return json!({ "error": format!("Agent '{}' has no child step", from_agent.name) }),
    };
    let to_child_step_id = match to_agent.child_step_id {
        Some(id) => id,
        None => return json!({ "error": format!("Agent '{}' has no child step", to_agent.name) }),
    };

    // Add edge via pipeline service (handles duplicate + cycle checks)
    let pip_ctx = pipeline_ctx(ctx);
    match pipeline::add_edge(repo, &pip_ctx, from_child_step_id, to_child_step_id).await {
        Ok(_) => {}
        Err(crate::server::services::ServiceError::Conflict(_)) => {
            return json!({
                "already_exists": true,
                "from": from_agent.name,
                "to": to_agent.name,
            });
        }
        Err(crate::server::services::ServiceError::Validation(msg)) => {
            return json!({
                "error": format!(
                    "Adding dependency {} \u{2192} {} would create a cycle: {}",
                    from_agent.name, to_agent.name, msg
                )
            });
        }
        Err(e) => {
            return json!({ "error": format!("Failed to create dependency: {}", e) });
        }
    }

    // Recompute roster execution order
    let execution_sequence = recompute_execution_order(repo, ctx)
        .await
        .unwrap_or_default();

    json!({
        "created": true,
        "from": from_agent.name,
        "to": to_agent.name,
        "execution_sequence": execution_sequence,
    })
}

async fn execute_remove_dependency(
    input: &Value,
    repo: &dyn WorkflowRepo,
    ctx: &WorkforceToolContext,
) -> Value {
    let from_name = match require_str(input, "from_agent") {
        Ok(v) => v,
        Err(e) => return e,
    };
    let to_name = match require_str(input, "to_agent") {
        Ok(v) => v,
        Err(e) => return e,
    };

    // Load brief + roster
    let brief = match repo.get_mission_brief(ctx.step_id).await {
        Ok(Some(b)) => b,
        Ok(None) => return json!({ "error": "No mission brief found" }),
        Err(e) => return json!({ "error": e.to_string() }),
    };
    let roster = repo.list_agent_roster(brief.id).await.unwrap_or_default();

    let from_agent = match find_agent_by_name(&roster, from_name) {
        Some(a) => a,
        None => return json!({ "error": format!("Agent '{}' not found in roster", from_name) }),
    };
    let to_agent = match find_agent_by_name(&roster, to_name) {
        Some(a) => a,
        None => return json!({ "error": format!("Agent '{}' not found in roster", to_name) }),
    };

    let from_child_step_id = match from_agent.child_step_id {
        Some(id) => id,
        None => return json!({ "error": format!("Agent '{}' has no child step", from_agent.name) }),
    };
    let to_child_step_id = match to_agent.child_step_id {
        Some(id) => id,
        None => return json!({ "error": format!("Agent '{}' has no child step", to_agent.name) }),
    };

    // Remove edge via pipeline service
    let pip_ctx = pipeline_ctx(ctx);
    if let Err(e) =
        pipeline::remove_edge(repo, &pip_ctx, from_child_step_id, to_child_step_id).await
    {
        return json!({ "error": format!("Failed to remove dependency: {}", e) });
    }

    // Recompute roster execution order
    let execution_sequence = recompute_execution_order(repo, ctx)
        .await
        .unwrap_or_default();

    json!({
        "removed": true,
        "from": from_agent.name,
        "to": to_agent.name,
        "execution_sequence": execution_sequence,
    })
}

