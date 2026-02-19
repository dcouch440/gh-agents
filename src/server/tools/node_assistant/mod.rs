//! Universal tool handlers for the node assistant.
//!
//! These tools are available to all archetypes and handle node-level
//! configuration: setting the archetype (execution_mode), name, and
//! description.

use serde_json::{json, Value};
use uuid::Uuid;

use crate::db::traits::WorkflowRepo;
use crate::server::tools::shared::{load_step_or_error, require_str};

mod tests;

/// Ambient context for node assistant tool execution.
pub struct StepToolContext {
    pub workflow_id: Uuid,
    pub step_id: Uuid,
}

/// Valid archetype values that map to execution_mode.
const VALID_ARCHETYPES: &[&str] = &["belief_capture", "room", "workforce"];

/// Execute a universal node assistant tool by name.
pub async fn execute_node_assistant_tool(
    name: &str,
    input: &Value,
    repo: &dyn WorkflowRepo,
    ctx: &StepToolContext,
) -> Value {
    match name {
        "set_node_archetype" => execute_set_archetype(input, repo, ctx).await,
        "set_node_name" => execute_set_name(input, repo, ctx).await,
        "set_node_description" => execute_set_description(input, repo, ctx).await,
        "render_panel" => execute_render_panel(input),
        _ => json!({ "error": format!("Unknown node assistant tool: {}", name) }),
    }
}

fn execute_render_panel(input: &Value) -> Value {
    let content = match require_str(input, "content") {
        Ok(v) => v,
        Err(e) => return e,
    };

    let submit_label = input["submit_label"].as_str().unwrap_or("Submit");

    json!({
        "rendered": true,
        "content": content,
        "submit_label": submit_label,
    })
}

async fn execute_set_archetype(
    input: &Value,
    repo: &dyn WorkflowRepo,
    ctx: &StepToolContext,
) -> Value {
    let archetype = match require_str(input, "archetype") {
        Ok(v) => v,
        Err(e) => return e,
    };

    if !VALID_ARCHETYPES.contains(&archetype) {
        return json!({
            "error": format!(
                "Invalid archetype '{}'. Must be one of: {}",
                archetype,
                VALID_ARCHETYPES.join(", ")
            )
        });
    }

    let mut step = match load_step_or_error(repo, ctx.step_id).await {
        Ok(s) => s,
        Err(e) => return e,
    };

    step.execution_mode = archetype.to_string();

    match repo.update_step(step).await {
        Ok(_) => json!({
            "archetype": archetype,
            "step_id": ctx.step_id.to_string(),
        }),
        Err(e) => json!({ "error": e.to_string() }),
    }
}

async fn execute_set_name(input: &Value, repo: &dyn WorkflowRepo, ctx: &StepToolContext) -> Value {
    let name = match require_str(input, "name") {
        Ok(v) => v,
        Err(e) => return e,
    };

    let mut step = match load_step_or_error(repo, ctx.step_id).await {
        Ok(s) => s,
        Err(e) => return e,
    };

    step.name = Some(name.to_string());

    match repo.update_step(step).await {
        Ok(_) => json!({
            "name": name,
            "step_id": ctx.step_id.to_string(),
        }),
        Err(e) => json!({ "error": e.to_string() }),
    }
}

async fn execute_set_description(
    input: &Value,
    repo: &dyn WorkflowRepo,
    ctx: &StepToolContext,
) -> Value {
    let description = match require_str(input, "description") {
        Ok(v) => v,
        Err(e) => return e,
    };

    let mut step = match load_step_or_error(repo, ctx.step_id).await {
        Ok(s) => s,
        Err(e) => return e,
    };

    step.description = description.to_string();

    match repo.update_step(step).await {
        Ok(_) => json!({
            "description": description,
            "step_id": ctx.step_id.to_string(),
        }),
        Err(e) => json!({ "error": e.to_string() }),
    }
}
