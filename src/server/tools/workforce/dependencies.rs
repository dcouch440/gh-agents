use serde_json::{json, Value};

use crate::db::traits::WorkflowRepo;
use crate::server::services::pipeline;
use crate::server::tools::shared::require_str;

use super::{
    agent_not_found_error, find_agent_by_name, pipeline_ctx, recompute_execution_order,
    WorkforceToolContext,
};

pub(super) async fn execute_set_dependency(
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
        None => return agent_not_found_error(from_name, &roster),
    };
    let to_agent = match find_agent_by_name(&roster, to_name) {
        Some(a) => a,
        None => return agent_not_found_error(to_name, &roster),
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

    let sequence_names: Vec<&str> = execution_sequence
        .iter()
        .filter_map(|v| v["name"].as_str())
        .collect();

    json!({
        "created": true,
        "from": from_agent.name,
        "to": to_agent.name,
        "execution_order": sequence_names,
    })
}

pub(super) async fn execute_remove_dependency(
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
        None => return agent_not_found_error(from_name, &roster),
    };
    let to_agent = match find_agent_by_name(&roster, to_name) {
        Some(a) => a,
        None => return agent_not_found_error(to_name, &roster),
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

    let sequence_names: Vec<&str> = execution_sequence
        .iter()
        .filter_map(|v| v["name"].as_str())
        .collect();

    json!({
        "removed": true,
        "from": from_agent.name,
        "to": to_agent.name,
        "execution_order": sequence_names,
    })
}
