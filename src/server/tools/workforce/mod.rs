//! Tool execution handlers for the workforce archetype.
//!
//! Unifies task force (agent roster) and documenter (deliverables) into
//! a single archetype. Each agent becomes a sub-workflow step in a child
//! workflow attached to the workforce node. A Designer step is auto-managed
//! (created with the first agent, removed with the last).

use chrono::Utc;
use serde_json::{json, Value};
use uuid::Uuid;

use crate::db::traits::WorkflowRepo;

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
        .create_workflow(
            parent_workflow.user_id,
            format!("{} (child)", step_name),
            String::new(),
            false,
            None,
            None,
            false,
        )
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

    // Wire edges in the child workflow
    if let Some(ref designer) = designer_step {
        // First agent: Designer → agent
        if let Err(e) = repo
            .add_edge(child_workflow_id, designer.id, child_step.id)
            .await
        {
            return json!({ "error": format!("Failed to wire Designer edge: {}", e) });
        }
    } else {
        // Subsequent agent: wire from the last roster entry's child step
        let prev_child_step_id = roster
            .iter()
            .max_by_key(|a| a.execution_order)
            .and_then(|a| a.child_step_id);

        if let Some(prev_step_id) = prev_child_step_id {
            if let Err(e) = repo
                .add_edge(child_workflow_id, prev_step_id, child_step.id)
                .await
            {
                return json!({ "error": format!("Failed to wire agent edge: {}", e) });
            }
        }
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

    json!({
        "agent_id": roster_agent.id.to_string(),
        "name": roster_agent.name,
        "role": roster_agent.role_description,
        "capabilities": roster_agent.capabilities,
        "execution_order": roster_agent.execution_order,
        "child_step_id": child_step.id.to_string(),
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

    // Remove the child step and rewire edges
    if let Some(child_step_id) = target.child_step_id {
        let step = repo.get_step(ctx.step_id).await.ok().flatten();
        if let Some(ref step) = step {
            if let Some(child_workflow_id) = step.child_workflow_id {
                let edges = repo.list_edges(child_workflow_id).await.unwrap_or_default();

                let predecessor = edges
                    .iter()
                    .find(|e| e.to_step_id == child_step_id)
                    .map(|e| e.from_step_id);
                let successor = edges
                    .iter()
                    .find(|e| e.from_step_id == child_step_id)
                    .map(|e| e.to_step_id);

                // Remove edges involving this step
                for edge in &edges {
                    if edge.from_step_id == child_step_id || edge.to_step_id == child_step_id {
                        let _ = repo.remove_edge(edge.from_step_id, edge.to_step_id).await;
                    }
                }

                // Rewire: predecessor → successor
                if let (Some(pred), Some(succ)) = (predecessor, successor) {
                    let _ = repo.add_edge(child_workflow_id, pred, succ).await;
                }

                // Delete the child step
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

    json!({
        "deleted": true,
        "name": agent_name,
        "id": agent_id.to_string(),
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
// Context Building (public helper for system prompt injection)
// =========================================================================

/// Build the config snapshot string for `{{.System.current_config}}` injection.
///
/// Merges task force config (mission brief, roster) with documenter config
/// (deliverables), showing agents with their assigned deliverables.
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

    let doc_defs = repo
        .list_document_defs(ctx.step_id)
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

        // Agents with deliverables
        let roster = repo
            .list_agent_roster(brief.id)
            .await
            .map_err(|e| e.to_string())?;

        out.push_str("\nAgents:\n");
        if roster.is_empty() {
            out.push_str("  (none)\n");
        } else {
            for (i, agent) in roster.iter().enumerate() {
                out.push_str(&format!(
                    "  {}. {} (id: {}){}{}\n",
                    i + 1,
                    agent.name,
                    agent.id,
                    if agent.role_description.is_empty() {
                        String::new()
                    } else {
                        format!(" — {}", agent.role_description)
                    },
                    if agent.capabilities.is_empty() {
                        String::new()
                    } else {
                        format!(" [{}]", agent.capabilities.join(", "))
                    }
                ));

                let agent_defs: Vec<_> = doc_defs
                    .iter()
                    .filter(|d| d.agent_roster_entry_id == Some(agent.id))
                    .collect();
                if !agent_defs.is_empty() {
                    out.push_str("     Deliverables:\n");
                    for def in agent_defs {
                        out.push_str(&format!(
                            "       - {} (~{} words){}\n",
                            def.name,
                            def.target_length,
                            if def.description.is_empty() {
                                String::new()
                            } else {
                                format!(" — {}", def.description)
                            }
                        ));
                    }
                }
            }
        }
    } else {
        out.push_str("\nTask: (not set)\n");
        out.push_str("\nAgents:\n  (none)\n");
    }

    // Unassigned deliverables
    let unassigned: Vec<_> = doc_defs
        .iter()
        .filter(|d| d.agent_roster_entry_id.is_none())
        .collect();
    if !unassigned.is_empty() {
        out.push_str("\nUnassigned Deliverables:\n");
        for def in unassigned {
            out.push_str(&format!(
                "  - {} (id: {}, ~{} words){}\n",
                def.name,
                def.id,
                def.target_length,
                if def.description.is_empty() {
                    String::new()
                } else {
                    format!(" — {}", def.description)
                }
            ));
        }
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
