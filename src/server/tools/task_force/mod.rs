//! Tool execution handlers for the task force archetype.
//!
//! These tools operate on a specific task force workflow step, managing
//! mission briefs and agent rosters. The chat strategy calls
//! `execute_task_force_tool` directly via dispatch.

use serde_json::{json, Value};
use uuid::Uuid;

use crate::db::traits::WorkflowRepo;

mod tests;

/// Ambient context for task force tool execution.
pub struct TaskForceToolContext {
    pub workflow_id: Uuid,
    pub step_id: Uuid,
}

/// Valid failure mode values.
const VALID_FAILURE_MODES: &[&str] = &["fail_fast", "skip_and_continue", "retry"];

/// Execute a task force tool by name.
pub async fn execute_task_force_tool(
    name: &str,
    input: &Value,
    repo: &dyn WorkflowRepo,
    ctx: &TaskForceToolContext,
) -> Value {
    match name {
        "set_task" => execute_set_task(input, repo, ctx).await,
        "add_agent" => execute_add_agent(input, repo, ctx).await,
        "update_agent" => execute_update_agent(input, repo).await,
        "remove_agent" => execute_remove_agent(input, repo).await,
        "set_capabilities" => execute_set_capabilities(input, repo, ctx).await,
        "set_failure_mode" => execute_set_failure_mode(input, repo, ctx).await,
        _ => json!({ "error": format!("Unknown task force tool: {}", name) }),
    }
}

/// Ensure a mission brief exists for this step, creating one if needed.
/// Returns the brief's ID.
async fn ensure_mission_brief(
    repo: &dyn WorkflowRepo,
    ctx: &TaskForceToolContext,
) -> Result<Uuid, Value> {
    match repo.get_mission_brief(ctx.step_id).await {
        Ok(Some(brief)) => Ok(brief.id),
        Ok(None) => {
            // Auto-create an empty brief
            match repo
                .upsert_mission_brief(ctx.step_id, "", &[], "fail_fast", None)
                .await
            {
                Ok(brief) => Ok(brief.id),
                Err(e) => Err(json!({ "error": format!("Failed to create mission brief: {}", e) })),
            }
        }
        Err(e) => Err(json!({ "error": format!("Failed to load mission brief: {}", e) })),
    }
}

async fn execute_set_task(
    input: &Value,
    repo: &dyn WorkflowRepo,
    ctx: &TaskForceToolContext,
) -> Value {
    let Some(description) = input["description"].as_str() else {
        return json!({ "error": "Missing required parameter: description" });
    };

    // Load existing brief to preserve other fields, or create new
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
    ctx: &TaskForceToolContext,
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

    // Compute next execution_order
    let roster = repo.list_agent_roster(brief_id).await.unwrap_or_default();
    let next_order = roster.iter().map(|a| a.execution_order).max().unwrap_or(-1) + 1;

    match repo
        .add_roster_agent(brief_id, name, role, &capabilities, next_order)
        .await
    {
        Ok(agent) => json!({
            "agent_id": agent.id.to_string(),
            "name": agent.name,
            "role": agent.role_description,
            "capabilities": agent.capabilities,
            "execution_order": agent.execution_order,
        }),
        Err(e) => json!({ "error": e.to_string() }),
    }
}

async fn execute_update_agent(input: &Value, repo: &dyn WorkflowRepo) -> Value {
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

    match repo
        .update_roster_agent(agent_id, name, role, capabilities)
        .await
    {
        Ok(agent) => json!({
            "agent_id": agent.id.to_string(),
            "name": agent.name,
            "role": agent.role_description,
            "capabilities": agent.capabilities,
        }),
        Err(e) => json!({ "error": e.to_string() }),
    }
}

async fn execute_remove_agent(input: &Value, repo: &dyn WorkflowRepo) -> Value {
    let Some(id_str) = input["agent_id"].as_str() else {
        return json!({ "error": "Missing required parameter: agent_id" });
    };
    let Ok(agent_id) = Uuid::parse_str(id_str) else {
        return json!({ "error": format!("Invalid UUID: {}", id_str) });
    };

    match repo.remove_roster_agent(agent_id).await {
        Ok(()) => json!({ "deleted": true }),
        Err(e) => json!({ "error": e.to_string() }),
    }
}

async fn execute_set_capabilities(
    input: &Value,
    repo: &dyn WorkflowRepo,
    ctx: &TaskForceToolContext,
) -> Value {
    let Some(caps_arr) = input["capabilities"].as_array() else {
        return json!({ "error": "Missing required parameter: capabilities (array)" });
    };
    let capabilities: Vec<String> = caps_arr
        .iter()
        .filter_map(|v| v.as_str().map(String::from))
        .collect();

    // Load existing brief to preserve other fields
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
    ctx: &TaskForceToolContext,
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

    // Load existing brief to preserve other fields
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
// Context Building (public helper for system prompt injection)
// =========================================================================

/// Build the config snapshot string for `{{.System.current_config}}` injection.
///
/// Called by the hub each turn to provide the assistant with the live state
/// of the task force step.
pub async fn build_config_snapshot(
    repo: &dyn WorkflowRepo,
    ctx: &TaskForceToolContext,
) -> Result<String, String> {
    // Load step
    let step = repo
        .get_step(ctx.step_id)
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "Step not found".to_string())?;

    // Load mission brief + roster
    let brief = repo
        .get_mission_brief(ctx.step_id)
        .await
        .map_err(|e| e.to_string())?;

    // Load edges to find upstream steps
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

        // Agent roster
        let roster = repo
            .list_agent_roster(brief.id)
            .await
            .map_err(|e| e.to_string())?;

        out.push_str("\nAgent Roster:\n");
        if roster.is_empty() {
            out.push_str("  (none)\n");
        } else {
            for agent in &roster {
                out.push_str(&format!(
                    "  - {} (id: {}){}{}\n",
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
            }
        }
    } else {
        out.push_str("\nTask: (not set)\n");
        out.push_str("\nAgent Roster:\n  (none)\n");
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
