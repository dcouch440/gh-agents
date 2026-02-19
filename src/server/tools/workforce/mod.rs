//! Tool execution handlers for the workforce archetype.
//!
//! Unifies task force (agent roster) and documenter (deliverables) into
//! a single archetype. Each agent becomes a sub-workflow step in a child
//! workflow attached to the workforce node. A Designer step is auto-managed
//! (created with the first agent, removed with the last).

use std::collections::{HashMap, HashSet, VecDeque};

use chrono::Utc;
use serde_json::{json, Value};
use uuid::Uuid;

use crate::db::traits::{CreateWorkflowInput, WorkflowRepo};
use crate::db::WorkflowStepEdgeRow;

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

/// Ensure a child workflow exists for this workforce step.
/// Creates one if needed and updates the step's `child_workflow_id`.
async fn ensure_child_workflow(
    repo: &dyn WorkflowRepo,
    ctx: &WorkforceToolContext,
) -> Result<Uuid, Value> {
    let step = repo
        .get_step(ctx.step_id)
        .await
        .map_err(|e| json!({ "error": format!("Failed to load step: {}", e) }))?
        .ok_or_else(|| json!({ "error": "Step not found" }))?;

    if let Some(child_workflow_id) = step.child_workflow_id {
        return Ok(child_workflow_id);
    }

    let parent_workflow = repo
        .get_workflow(ctx.workflow_id)
        .await
        .map_err(|e| json!({ "error": format!("Failed to load workflow: {}", e) }))?
        .ok_or_else(|| json!({ "error": "Parent workflow not found" }))?;

    let step_name = step.name.clone().unwrap_or_else(|| "Workforce".to_string());
    let child_workflow = repo
        .create_workflow(CreateWorkflowInput {
            user_id: parent_workflow.user_id,
            name: format!("{} (child)", step_name),
            description: String::new(),
            container_enabled: false,
            target_repo_url: None,
            target_branch: None,
            vpn_enabled: false,
        })
        .await
        .map_err(|e| json!({ "error": format!("Failed to create child workflow: {}", e) }))?;

    let mut updated_step = step;
    updated_step.child_workflow_id = Some(child_workflow.id);
    repo.update_step(updated_step)
        .await
        .map_err(|e| json!({ "error": format!("Failed to link child workflow: {}", e) }))?;

    Ok(child_workflow.id)
}

/// Create the auto-managed Designer step in the child workflow.
async fn create_designer_step(
    repo: &dyn WorkflowRepo,
    child_workflow_id: Uuid,
) -> Result<crate::db::WorkflowStepRow, Value> {
    let step = crate::db::WorkflowStepRow {
        id: Uuid::new_v4(),
        workflow_id: child_workflow_id,
        agent_id: None,
        execution_mode: "single".to_string(),
        agent_execution_mode: None,
        for_each_ref: None,
        prompt_template_id: None,
        prompt_template: String::new(),
        output_schema_id: None,
        output_variable_name: Some("designer_output".to_string()),
        interactive_agent_id: None,
        for_each_label_field: None,
        room_id: None,
        routing_mode: None,
        routing_field: None,
        display_order: 0,
        version: 1,
        reasoning_trace: false,
        verification_agent_ids: None,
        position_x: Some(0.0),
        position_y: Some(0.0),
        width: None,
        height: None,
        name: Some("Designer".to_string()),
        system_prompt_suffix: None,
        visible: true,
        description: "Auto-managed Designer step".to_string(),
        board_context_cache: String::new(),
        board_context_updated_at: None,
        goal_summary: String::new(),
        goal_summary_updated_at: None,
        sub_workflow_template_id: None,
        child_workflow_id: None,
        is_designer_step: true,
        pinned: false,
        run_results_summary: String::new(),
    };

    repo.create_step(step)
        .await
        .map_err(|e| json!({ "error": format!("Failed to create Designer step: {}", e) }))
}

/// Remove the Designer step from the child workflow.
async fn remove_designer_step(
    repo: &dyn WorkflowRepo,
    child_workflow_id: Uuid,
) -> Result<(), Value> {
    let steps = repo
        .list_steps(child_workflow_id)
        .await
        .map_err(|e| json!({ "error": format!("Failed to list steps: {}", e) }))?;

    if let Some(designer) = steps.iter().find(|s| s.is_designer_step) {
        let edges = repo
            .list_edges(child_workflow_id)
            .await
            .map_err(|e| json!({ "error": format!("Failed to list edges: {}", e) }))?;

        for edge in &edges {
            if edge.from_step_id == designer.id || edge.to_step_id == designer.id {
                repo.remove_edge(edge.from_step_id, edge.to_step_id)
                    .await
                    .map_err(|e| json!({ "error": format!("Failed to remove edge: {}", e) }))?;
            }
        }

        repo.delete_step(designer.id)
            .await
            .map_err(|e| json!({ "error": format!("Failed to delete Designer step: {}", e) }))?;
    }

    Ok(())
}

/// Build a child workflow step for an agent.
fn build_agent_child_step(
    child_workflow_id: Uuid,
    name: &str,
    role: &str,
    display_order: i32,
) -> crate::db::WorkflowStepRow {
    crate::db::WorkflowStepRow {
        id: Uuid::new_v4(),
        workflow_id: child_workflow_id,
        agent_id: None,
        execution_mode: "single".to_string(),
        agent_execution_mode: None,
        for_each_ref: None,
        prompt_template_id: None,
        prompt_template: String::new(),
        output_schema_id: None,
        output_variable_name: Some(crate::server::hub::dag::dag_state::to_snake_case(name)),
        interactive_agent_id: None,
        for_each_label_field: None,
        room_id: None,
        routing_mode: None,
        routing_field: None,
        display_order,
        version: 1,
        reasoning_trace: false,
        verification_agent_ids: None,
        position_x: Some(display_order as f64 * 200.0),
        position_y: Some(0.0),
        width: None,
        height: None,
        name: Some(name.to_string()),
        system_prompt_suffix: None,
        visible: true,
        description: role.to_string(),
        board_context_cache: String::new(),
        board_context_updated_at: None,
        goal_summary: String::new(),
        goal_summary_updated_at: None,
        sub_workflow_template_id: None,
        child_workflow_id: None,
        is_designer_step: false,
        pinned: false,
        run_results_summary: String::new(),
    }
}

// =========================================================================
// Tool Handlers — Mission Brief & Roster (shared with task_force pattern)
// =========================================================================

async fn execute_set_task(
    input: &Value,
    repo: &dyn WorkflowRepo,
    ctx: &WorkforceToolContext,
) -> Value {
    let Some(description) = input["description"].as_str() else {
        return json!({ "error": "Missing required parameter: description" });
    };

    let existing = repo.get_mission_brief(ctx.step_id).await.ok().flatten();
    let (capabilities, failure_mode, downstream_context) = match &existing {
        Some(brief) => (
            brief.available_capabilities.clone(),
            brief.failure_mode.clone(),
            brief.downstream_context.clone(),
        ),
        None => (vec![], "fail_fast".to_string(), None),
    };

    match repo
        .upsert_mission_brief(
            ctx.step_id,
            description,
            &capabilities,
            &failure_mode,
            downstream_context,
        )
        .await
    {
        Ok(brief) => json!({
            "step_id": ctx.step_id.to_string(),
            "task_description": brief.task_description,
        }),
        Err(e) => json!({ "error": e.to_string() }),
    }
}

async fn execute_add_agent(
    input: &Value,
    repo: &dyn WorkflowRepo,
    ctx: &WorkforceToolContext,
) -> Value {
    let Some(name) = input["name"].as_str() else {
        return json!({ "error": "Missing required parameter: name" });
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

    // Ensure brief exists
    let brief_id = match ensure_mission_brief(repo, ctx).await {
        Ok(id) => id,
        Err(e) => return e,
    };

    // Ensure child workflow exists
    let child_workflow_id = match ensure_child_workflow(repo, ctx).await {
        Ok(id) => id,
        Err(e) => return e,
    };

    // Get existing roster
    let roster = repo.list_agent_roster(brief_id).await.unwrap_or_default();
    let next_order = roster.iter().map(|a| a.execution_order).max().unwrap_or(-1) + 1;
    let is_first_agent = roster.is_empty();

    // Create Designer if first agent
    let designer_step = if is_first_agent {
        match create_designer_step(repo, child_workflow_id).await {
            Ok(s) => Some(s),
            Err(e) => return e,
        }
    } else {
        None
    };

    // Create child workflow step for the agent
    let child_step = build_agent_child_step(child_workflow_id, name, role, next_order + 1);
    let child_step = match repo.create_step(child_step).await {
        Ok(s) => s,
        Err(e) => return json!({ "error": format!("Failed to create child step: {}", e) }),
    };

    // Wire edge: Designer → new agent (fan-out, not linear chain)
    let designer_id = if let Some(ref designer) = designer_step {
        designer.id
    } else {
        // Find existing Designer step in child workflow
        let child_steps = repo.list_steps(child_workflow_id).await.unwrap_or_default();
        match child_steps.iter().find(|s| s.is_designer_step) {
            Some(s) => s.id,
            None => {
                return json!({ "error": "Designer step not found in child workflow" });
            }
        }
    };
    if let Err(e) = repo
        .add_edge(child_workflow_id, designer_id, child_step.id)
        .await
    {
        return json!({ "error": format!("Failed to wire Designer edge: {}", e) });
    }

    // Create roster entry
    let roster_agent = match repo
        .add_roster_agent(brief_id, name, role, &capabilities, next_order)
        .await
    {
        Ok(agent) => agent,
        Err(e) => return json!({ "error": e.to_string() }),
    };

    // Link roster entry to child step
    if let Err(e) = repo
        .link_roster_agent_to_child_step(roster_agent.id, Some(child_step.id))
        .await
    {
        return json!({ "error": format!("Failed to link roster to child step: {}", e) });
    }

    // Recompute execution order from dependency graph
    let execution_sequence = recompute_execution_order(repo, ctx).await.unwrap_or_default();

    json!({
        "agent_id": roster_agent.id.to_string(),
        "name": roster_agent.name,
        "role": roster_agent.role_description,
        "capabilities": roster_agent.capabilities,
        "child_step_id": child_step.id.to_string(),
        "execution_sequence": execution_sequence,
    })
}

async fn execute_update_agent(
    input: &Value,
    repo: &dyn WorkflowRepo,
    ctx: &WorkforceToolContext,
) -> Value {
    let _ = ctx; // Used in the future for context; suppress unused warning

    let Some(id_str) = input["agent_id"].as_str() else {
        return json!({ "error": "Missing required parameter: agent_id" });
    };
    let Ok(agent_id) = Uuid::parse_str(id_str) else {
        return json!({ "error": format!("Invalid UUID: {}", id_str) });
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

    // Also update the child step name/description if they changed
    if let Some(child_step_id) = agent.child_step_id {
        if name.is_some() || role.is_some() {
            if let Ok(Some(mut child_step)) = repo.get_step(child_step_id).await {
                if let Some(ref new_name) = name {
                    child_step.name = Some(new_name.clone());
                    child_step.output_variable_name =
                        Some(crate::server::hub::dag::dag_state::to_snake_case(new_name));
                }
                if let Some(ref new_role) = role {
                    child_step.description = new_role.clone();
                }
                let _ = repo.update_step(child_step).await;
            }
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
    let Some(id_str) = input["agent_id"].as_str() else {
        return json!({ "error": "Missing required parameter: agent_id" });
    };
    let Ok(agent_id) = Uuid::parse_str(id_str) else {
        return json!({ "error": format!("Invalid UUID: {}", id_str) });
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

    // Remove the child step and all its edges (no bridging — fan-in/fan-out topology)
    if let Some(child_step_id) = target.child_step_id {
        let step = repo.get_step(ctx.step_id).await.ok().flatten();
        if let Some(ref step) = step {
            if let Some(child_workflow_id) = step.child_workflow_id {
                let edges = repo.list_edges(child_workflow_id).await.unwrap_or_default();

                for edge in &edges {
                    if edge.from_step_id == child_step_id || edge.to_step_id == child_step_id {
                        let _ = repo.remove_edge(edge.from_step_id, edge.to_step_id).await;
                    }
                }

                let _ = repo.delete_step(child_step_id).await;
            }
        }
    }

    // Remove from roster
    if let Err(e) = repo.remove_roster_agent(agent_id).await {
        return json!({ "error": e.to_string() });
    }

    // If roster is now empty, remove Designer and clear child_workflow_id
    let remaining = roster.len() - 1; // we already know we removed one
    if remaining == 0 {
        let step = repo.get_step(ctx.step_id).await.ok().flatten();
        if let Some(ref step) = step {
            if let Some(child_workflow_id) = step.child_workflow_id {
                let _ = remove_designer_step(repo, child_workflow_id).await;
            }
        }
        // Clear child_workflow_id on parent step
        if let Some(mut step) = step {
            step.child_workflow_id = None;
            let _ = repo.update_step(step).await;
        }
    }

    // Recompute execution order from dependency graph (closes gaps)
    let execution_sequence = recompute_execution_order(repo, ctx).await.unwrap_or_default();

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
    let Some(caps_arr) = input["capabilities"].as_array() else {
        return json!({ "error": "Missing required parameter: capabilities (array)" });
    };
    let capabilities: Vec<String> = caps_arr
        .iter()
        .filter_map(|v| v.as_str().map(String::from))
        .collect();

    let existing = repo.get_mission_brief(ctx.step_id).await.ok().flatten();
    let (task_description, failure_mode, downstream_context) = match &existing {
        Some(brief) => (
            brief.task_description.clone(),
            brief.failure_mode.clone(),
            brief.downstream_context.clone(),
        ),
        None => (String::new(), "fail_fast".to_string(), None),
    };

    match repo
        .upsert_mission_brief(
            ctx.step_id,
            &task_description,
            &capabilities,
            &failure_mode,
            downstream_context,
        )
        .await
    {
        Ok(brief) => json!({
            "capabilities": brief.available_capabilities,
        }),
        Err(e) => json!({ "error": e.to_string() }),
    }
}

async fn execute_set_failure_mode(
    input: &Value,
    repo: &dyn WorkflowRepo,
    ctx: &WorkforceToolContext,
) -> Value {
    let Some(mode) = input["mode"].as_str() else {
        return json!({ "error": "Missing required parameter: mode" });
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

    let existing = repo.get_mission_brief(ctx.step_id).await.ok().flatten();
    let (task_description, capabilities, downstream_context) = match &existing {
        Some(brief) => (
            brief.task_description.clone(),
            brief.available_capabilities.clone(),
            brief.downstream_context.clone(),
        ),
        None => (String::new(), vec![], None),
    };

    match repo
        .upsert_mission_brief(
            ctx.step_id,
            &task_description,
            &capabilities,
            mode,
            downstream_context,
        )
        .await
    {
        Ok(brief) => json!({
            "failure_mode": brief.failure_mode,
        }),
        Err(e) => json!({ "error": e.to_string() }),
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
    let Some(name) = input["name"].as_str() else {
        return json!({ "error": "Missing required parameter: name" });
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
    let Some(id_str) = input["deliverable_id"].as_str() else {
        return json!({ "error": "Missing required parameter: deliverable_id" });
    };
    let Ok(def_id) = Uuid::parse_str(id_str) else {
        return json!({ "error": format!("Invalid UUID: {}", id_str) });
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
    let Some(id_str) = input["deliverable_id"].as_str() else {
        return json!({ "error": "Missing required parameter: deliverable_id" });
    };
    let Ok(def_id) = Uuid::parse_str(id_str) else {
        return json!({ "error": format!("Invalid UUID: {}", id_str) });
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

/// Check if adding an edge from `from_step_id` to `to_step_id` would create a cycle.
///
/// Performs BFS from `to_step_id` following existing outgoing edges. If
/// `from_step_id` is reachable, adding the proposed edge creates a cycle.
fn would_create_cycle(
    from_step_id: Uuid,
    to_step_id: Uuid,
    edges: &[WorkflowStepEdgeRow],
) -> bool {
    let mut adjacency: HashMap<Uuid, Vec<Uuid>> = HashMap::new();
    for edge in edges {
        adjacency
            .entry(edge.from_step_id)
            .or_default()
            .push(edge.to_step_id);
    }

    let mut visited = HashSet::new();
    let mut queue = VecDeque::new();
    queue.push_back(to_step_id);

    while let Some(current) = queue.pop_front() {
        if current == from_step_id {
            return true;
        }
        if !visited.insert(current) {
            continue;
        }
        if let Some(neighbors) = adjacency.get(&current) {
            queue.extend(neighbors);
        }
    }

    false
}

async fn execute_set_dependency(
    input: &Value,
    repo: &dyn WorkflowRepo,
    ctx: &WorkforceToolContext,
) -> Value {
    let Some(from_name) = input["from_agent"].as_str() else {
        return json!({ "error": "Missing required parameter: from_agent" });
    };
    let Some(to_name) = input["to_agent"].as_str() else {
        return json!({ "error": "Missing required parameter: to_agent" });
    };

    // Load brief + roster
    let brief = match repo.get_mission_brief(ctx.step_id).await {
        Ok(Some(b)) => b,
        Ok(None) => return json!({ "error": "No mission brief found" }),
        Err(e) => return json!({ "error": e.to_string() }),
    };
    let roster = repo.list_agent_roster(brief.id).await.unwrap_or_default();

    // Resolve agent names
    let from_agent = match find_agent_by_name(&roster, from_name) {
        Some(a) => a,
        None => return json!({ "error": format!("Agent '{}' not found in roster", from_name) }),
    };
    let to_agent = match find_agent_by_name(&roster, to_name) {
        Some(a) => a,
        None => return json!({ "error": format!("Agent '{}' not found in roster", to_name) }),
    };

    // Validate: no self-edges
    if from_agent.id == to_agent.id {
        return json!({ "error": "Cannot create a dependency from an agent to itself" });
    }

    // Both must have child_step_id
    let from_child_step_id = match from_agent.child_step_id {
        Some(id) => id,
        None => return json!({ "error": format!("Agent '{}' has no child step", from_agent.name) }),
    };
    let to_child_step_id = match to_agent.child_step_id {
        Some(id) => id,
        None => return json!({ "error": format!("Agent '{}' has no child step", to_agent.name) }),
    };

    // Get child workflow
    let step = match repo.get_step(ctx.step_id).await {
        Ok(Some(s)) => s,
        Ok(None) => return json!({ "error": "Step not found" }),
        Err(e) => return json!({ "error": e.to_string() }),
    };
    let child_workflow_id = match step.child_workflow_id {
        Some(id) => id,
        None => return json!({ "error": "No child workflow exists" }),
    };

    // Check for duplicate edge
    let edges = repo.list_edges(child_workflow_id).await.unwrap_or_default();
    if edges
        .iter()
        .any(|e| e.from_step_id == from_child_step_id && e.to_step_id == to_child_step_id)
    {
        return json!({
            "already_exists": true,
            "from": from_agent.name,
            "to": to_agent.name,
        });
    }

    // Check for cycles
    if would_create_cycle(from_child_step_id, to_child_step_id, &edges) {
        return json!({
            "error": format!(
                "Adding dependency {} \u{2192} {} would create a cycle (there is already a path from {} to {})",
                from_agent.name, to_agent.name, to_agent.name, from_agent.name
            )
        });
    }

    // Create the edge
    if let Err(e) = repo
        .add_edge(child_workflow_id, from_child_step_id, to_child_step_id)
        .await
    {
        return json!({ "error": format!("Failed to create dependency: {}", e) });
    }

    // Recompute execution order from dependency graph
    let execution_sequence = recompute_execution_order(repo, ctx).await.unwrap_or_default();

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
    let Some(from_name) = input["from_agent"].as_str() else {
        return json!({ "error": "Missing required parameter: from_agent" });
    };
    let Some(to_name) = input["to_agent"].as_str() else {
        return json!({ "error": "Missing required parameter: to_agent" });
    };

    // Load brief + roster
    let brief = match repo.get_mission_brief(ctx.step_id).await {
        Ok(Some(b)) => b,
        Ok(None) => return json!({ "error": "No mission brief found" }),
        Err(e) => return json!({ "error": e.to_string() }),
    };
    let roster = repo.list_agent_roster(brief.id).await.unwrap_or_default();

    // Resolve agent names
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

    // Remove the edge
    if let Err(e) = repo.remove_edge(from_child_step_id, to_child_step_id).await {
        return json!({ "error": format!("Failed to remove dependency: {}", e) });
    }

    // Recompute execution order from dependency graph
    let execution_sequence = recompute_execution_order(repo, ctx).await.unwrap_or_default();

    json!({
        "removed": true,
        "from": from_agent.name,
        "to": to_agent.name,
        "execution_sequence": execution_sequence,
    })
}

// =========================================================================
// Topology Recomputation
// =========================================================================

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
            // Both endpoints are roster agents (not Designer)
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
// Context Building (public helper for system prompt injection)
// =========================================================================

/// Build the config snapshot string for `{{.System.current_config}}` injection.
///
/// Shows the current step configuration: mission brief, agent roster,
/// and incoming context from upstream steps.
pub async fn build_config_snapshot(
    repo: &dyn WorkflowRepo,
    ctx: &WorkforceToolContext,
) -> Result<String, String> {
    let step = repo
        .get_step(ctx.step_id)
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "Step not found".to_string())?;

    let brief = repo
        .get_mission_brief(ctx.step_id)
        .await
        .map_err(|e| e.to_string())?;

    let edges = repo
        .list_edges(ctx.workflow_id)
        .await
        .map_err(|e| e.to_string())?;

    let upstream_step_ids: Vec<Uuid> = edges
        .iter()
        .filter(|e| e.to_step_id == ctx.step_id)
        .map(|e| e.from_step_id)
        .collect();

    let mut out = String::new();

    // Step config
    out.push_str(&format!(
        "Name: {}\n",
        step.name.as_deref().unwrap_or("(not set)")
    ));
    out.push_str(&format!(
        "Description: {}\n",
        if step.description.is_empty() {
            "(not set)"
        } else {
            &step.description
        }
    ));

    // Mission brief
    if let Some(ref brief) = brief {
        out.push_str(&format!(
            "\nTask: {}\n",
            if brief.task_description.is_empty() {
                "(not set)"
            } else {
                &brief.task_description
            }
        ));
        out.push_str(&format!("Failure Mode: {}\n", brief.failure_mode));

        if !brief.available_capabilities.is_empty() {
            out.push_str(&format!(
                "Available Capabilities: {}\n",
                brief.available_capabilities.join(", ")
            ));
        }

        // Agents + Dependencies (topology-aware view)
        let roster = repo
            .list_agent_roster(brief.id)
            .await
            .map_err(|e| e.to_string())?;

        // Load child workflow edges for dependency info
        let child_edges = if let Some(child_wf_id) = step.child_workflow_id {
            repo.list_edges(child_wf_id).await.unwrap_or_default()
        } else {
            vec![]
        };

        let child_step_to_name: HashMap<Uuid, &str> = roster
            .iter()
            .filter_map(|a| a.child_step_id.map(|csid| (csid, a.name.as_str())))
            .collect();

        // Build agent-to-agent edges and receives_from lookup
        let agent_step_ids: HashSet<&Uuid> = child_step_to_name.keys().collect();
        let mut receives_from_map: HashMap<&str, Vec<&str>> = HashMap::new();
        let mut agent_edges: Vec<(&str, &str)> = Vec::new();

        for edge in &child_edges {
            if agent_step_ids.contains(&edge.from_step_id)
                && agent_step_ids.contains(&edge.to_step_id)
            {
                if let (Some(&from_name), Some(&to_name)) = (
                    child_step_to_name.get(&edge.from_step_id),
                    child_step_to_name.get(&edge.to_step_id),
                ) {
                    agent_edges.push((from_name, to_name));
                    receives_from_map
                        .entry(to_name)
                        .or_default()
                        .push(from_name);
                }
            }
        }

        out.push_str("\nExecution sequence (derived from dependency graph):\n");
        if roster.is_empty() {
            out.push_str("  (none)\n");
        } else {
            for (i, agent) in roster.iter().enumerate() {
                let receives = receives_from_map.get(agent.name.as_str());
                let routing = if let Some(froms) = receives {
                    format!(" \u{2190} receives from: {}", froms.join(", "))
                } else {
                    String::new()
                };
                out.push_str(&format!(
                    "  {}. {} (id: {}){}{}{}\n",
                    i + 1,
                    agent.name,
                    agent.id,
                    if agent.capabilities.is_empty() {
                        String::new()
                    } else {
                        format!(" [{}]", agent.capabilities.join(", "))
                    },
                    if agent.role_description.is_empty() {
                        String::new()
                    } else {
                        format!(" — {}", agent.role_description)
                    },
                    routing,
                ));
            }
        }

        out.push_str("\nDependencies:\n");
        if agent_edges.is_empty() {
            out.push_str("  (none)\n");
        } else {
            for (from, to) in &agent_edges {
                out.push_str(&format!("  {} \u{2192} {}\n", from, to));
            }
        }
    } else {
        out.push_str("\nTask: (not set)\n");
        out.push_str("\nAgents:\n  (none)\n");
    }

    // Incoming context
    out.push_str("\nIncoming Context:\n");
    if upstream_step_ids.is_empty() {
        out.push_str("  (no connected sources)\n");
    } else {
        for upstream_id in upstream_step_ids {
            let upstream = match repo.get_step(upstream_id).await {
                Ok(Some(s)) => s,
                _ => continue,
            };

            let (status, preview, word_count) =
                crate::server::tools::shared::classify_content_status(&upstream);
            let name = upstream
                .name
                .unwrap_or_else(|| format!("Step {}", upstream.id));

            out.push_str(&format!(
                "  - {} ({}) — {}\n",
                name, upstream.execution_mode, status
            ));
            if !upstream.description.is_empty() {
                out.push_str(&format!("    Description: {}\n", upstream.description));
            }
            if let Some(preview) = preview {
                out.push_str(&format!(
                    "    Preview ({} words): {}\n",
                    word_count.unwrap_or(0),
                    preview
                ));
            }
        }
    }

    Ok(out)
}
