use serde_json::{json, Value};

use crate::db::traits::WorkflowRepo;
use crate::server::services::pipeline::{self, AddStepInput, UpdateStepInput};
use crate::server::tools::shared::{require_array, require_str};

use super::{
    ensure_mission_brief, pipeline_ctx, recompute_execution_order, resolve_agent_id,
    resolve_user_id, upsert_mission_brief_field, WorkforceToolContext,
};

pub(super) async fn execute_set_task(
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

pub(super) async fn execute_add_agent(
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

    // 4b. Auto-create sequential edge from previous agent (if any)
    if let Some(prev_agent) = roster
        .iter()
        .filter(|a| a.child_step_id.is_some())
        .max_by_key(|a| a.execution_order)
    {
        if let Some(prev_step_id) = prev_agent.child_step_id {
            let pip_ctx = pipeline_ctx(ctx);
            match pipeline::add_edge(repo, &pip_ctx, prev_step_id, step_added.step_id).await {
                Ok(_) | Err(crate::server::services::ServiceError::Conflict(_)) => {}
                Err(e) => {
                    tracing::warn!(
                        from_step = %prev_step_id,
                        to_step = %step_added.step_id,
                        error = %e,
                        "Failed to auto-create sequential edge for add_agent"
                    );
                }
            }
        }
    }

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

    let sequence_names: Vec<&str> = execution_sequence
        .iter()
        .filter_map(|v| v["name"].as_str())
        .collect();

    json!({
        "name": roster_agent.name,
        "role": roster_agent.role_description,
        "capabilities": roster_agent.capabilities,
        "execution_order": sequence_names,
    })
}

pub(super) async fn execute_update_agent(
    input: &Value,
    repo: &dyn WorkflowRepo,
    ctx: &WorkforceToolContext,
) -> Value {
    let agent_id = match resolve_agent_id(input, repo, ctx).await {
        Ok(v) => v,
        Err(e) => return e,
    };

    // When name is used for identification (no agent_id), don't treat it as a rename.
    // Only use name as a rename if agent_id was also provided.
    let name = if input["agent_id"].as_str().is_some() {
        input["name"].as_str().map(String::from)
    } else {
        // name was used for lookup — check for explicit new_name field
        input["new_name"].as_str().map(String::from)
    };
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

pub(super) async fn execute_remove_agent(
    input: &Value,
    repo: &dyn WorkflowRepo,
    ctx: &WorkforceToolContext,
) -> Value {
    let agent_id = match resolve_agent_id(input, repo, ctx).await {
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

    let sequence_names: Vec<&str> = execution_sequence
        .iter()
        .filter_map(|v| v["name"].as_str())
        .collect();

    json!({
        "deleted": true,
        "name": agent_name,
        "execution_order": sequence_names,
    })
}

pub(super) async fn execute_set_capabilities(
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
