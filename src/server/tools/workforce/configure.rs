use std::collections::{HashMap, HashSet};

use serde_json::{json, Value};
use uuid::Uuid;

use crate::db::traits::WorkflowRepo;
use crate::server::services::pipeline::{self, AddStepInput, UpdateStepInput};
use crate::server::tools::shared::{require_array, require_str};

use super::{
    ensure_mission_brief, normalize_name, pipeline_ctx, recompute_execution_order, resolve_user_id,
    upsert_mission_brief_field, WorkforceToolContext,
};

/// Declaratively configure the full team: task, agents, and dependencies.
///
/// Diffs desired state against current state and applies minimal mutations.
/// Agent matching is case-insensitive. Recomputes execution order once at
/// the end.
pub(super) async fn execute_configure_team(
    input: &Value,
    repo: &dyn WorkflowRepo,
    ctx: &WorkforceToolContext,
) -> Value {
    // --- Parse input ---------------------------------------------------
    let task = match require_str(input, "task") {
        Ok(v) => v,
        Err(e) => return e,
    };
    let agents_arr = match require_array(input, "agents") {
        Ok(v) => v,
        Err(e) => return e,
    };
    let deps_arr = input["dependencies"]
        .as_array()
        .cloned()
        .unwrap_or_default();

    // Ensure mission brief exists early — before any validation that could
    // return an error. This guarantees the brief is in the DB even if agent
    // parsing or dependency validation fails.
    let brief_id = match ensure_mission_brief(repo, ctx).await {
        Ok(id) => id,
        Err(e) => return e,
    };

    // Parse desired agents
    let mut desired_agents: Vec<(String, String, Vec<String>)> = Vec::new();
    for (i, agent_val) in agents_arr.iter().enumerate() {
        let name = match agent_val["name"].as_str() {
            Some(n) => n.to_string(),
            None => return json!({ "error": format!("agents[{}] missing 'name'", i) }),
        };
        let role_description = match agent_val["role_description"].as_str() {
            Some(r) => r.to_string(),
            None => return json!({ "error": format!("agents[{}] missing 'role_description'", i) }),
        };
        let capabilities: Vec<String> = agent_val["capabilities"]
            .as_array()
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str().map(String::from))
                    .collect()
            })
            .unwrap_or_default();
        desired_agents.push((name, role_description, capabilities));
    }

    // Deduplicate agents by normalized name (last wins).
    // Models sometimes stutter and produce the same agent twice.
    {
        let mut seen: HashMap<String, usize> = HashMap::new();
        let mut keep = vec![true; desired_agents.len()];
        for (i, (name, _, _)) in desired_agents.iter().enumerate() {
            let norm = normalize_name(name);
            if let Some(prev) = seen.insert(norm, i) {
                keep[prev] = false;
            }
        }
        let mut i = 0;
        desired_agents.retain(|_| {
            let k = keep[i];
            i += 1;
            k
        });
    }

    // Parse desired dependencies
    let mut desired_deps: Vec<(String, String)> = Vec::new();
    for (i, dep_val) in deps_arr.iter().enumerate() {
        let from = match dep_val["from"].as_str() {
            Some(f) => f.to_string(),
            None => return json!({ "error": format!("dependencies[{}] missing 'from'", i) }),
        };
        let to = match dep_val["to"].as_str() {
            Some(t) => t.to_string(),
            None => return json!({ "error": format!("dependencies[{}] missing 'to'", i) }),
        };
        desired_deps.push((from, to));
    }

    // Validate dependency agent names exist in desired roster
    let desired_name_set: HashSet<String> = desired_agents
        .iter()
        .map(|(n, _, _)| normalize_name(n))
        .collect();
    for (from, to) in &desired_deps {
        if !desired_name_set.contains(&normalize_name(from)) {
            return json!({ "error": format!("Dependency references unknown agent '{}'", from) });
        }
        if !desired_name_set.contains(&normalize_name(to)) {
            return json!({ "error": format!("Dependency references unknown agent '{}'", to) });
        }
        if normalize_name(from) == normalize_name(to) {
            return json!({ "error": format!("Self-dependency not allowed: '{}'", from) });
        }
    }

    // --- Load current state --------------------------------------------
    let current_brief = repo.get_mission_brief(ctx.step_id).await.ok().flatten();
    let current_roster = repo.list_agent_roster(brief_id).await.unwrap_or_default();
    let user_id = match resolve_user_id(repo, ctx).await {
        Ok(id) => id,
        Err(e) => return e,
    };

    // --- Diff task ------------------------------------------------------
    let task_status = if current_brief.as_ref().map(|b| b.task_description.as_str()) == Some(task) {
        "unchanged"
    } else {
        if let Err(e) =
            upsert_mission_brief_field(repo, ctx.step_id, Some(task), None, None, None).await
        {
            return json!({ "error": format!("Failed to update task: {}", e) });
        }
        if current_brief
            .as_ref()
            .map(|b| b.task_description.is_empty())
            .unwrap_or(true)
        {
            "created"
        } else {
            "updated"
        }
    };

    // --- Diff agents ---------------------------------------------------
    let current_by_name: HashMap<String, &crate::db::TaskAgentRosterRow> = current_roster
        .iter()
        .map(|a| (normalize_name(&a.name), a))
        .collect();

    let pip_ctx = pipeline_ctx(ctx);
    let mut agent_results: Vec<Value> = Vec::new();
    let mut matched_current_ids: HashSet<Uuid> = HashSet::new();
    let mut created_count: i32 = 0;

    // Process desired agents: create or update
    for (name, role_description, capabilities) in &desired_agents {
        let norm = normalize_name(name);
        if let Some(current) = current_by_name.get(&norm) {
            matched_current_ids.insert(current.id);

            let role_changed = current.role_description != *role_description;
            let caps_changed = current.capabilities != *capabilities;

            if !role_changed && !caps_changed {
                agent_results.push(json!({ "name": name, "status": "unchanged" }));
                continue;
            }

            // Update roster agent
            let new_role = if role_changed {
                Some(role_description.clone())
            } else {
                None
            };
            let new_caps = if caps_changed {
                Some(capabilities.clone())
            } else {
                None
            };
            if let Err(e) = repo
                .update_roster_agent(current.id, None, new_role, new_caps)
                .await
            {
                return json!({ "error": format!("Failed to update agent '{}': {}", name, e) });
            }

            // Sync child step if role changed
            if role_changed {
                if let Some(child_step_id) = current.child_step_id {
                    let _ = pipeline::update_step(
                        repo,
                        &pip_ctx,
                        child_step_id,
                        UpdateStepInput {
                            description: Some(role_description.clone()),
                            ..Default::default()
                        },
                    )
                    .await;
                }
            }

            agent_results.push(json!({ "name": name, "status": "updated" }));
        } else {
            // New agent — create
            let next_order = current_roster
                .iter()
                .map(|a| a.execution_order)
                .max()
                .unwrap_or(-1)
                + 1
                + created_count;

            let (step_added, _) = match pipeline::add_step(
                repo,
                &pip_ctx,
                user_id,
                AddStepInput {
                    name: name.clone(),
                    description: role_description.clone(),
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
                Err(e) => {
                    return json!({ "error": format!("Pipeline error creating '{}': {}", name, e) })
                }
            };

            let roster_agent = match repo
                .add_roster_agent(brief_id, name, role_description, capabilities, next_order)
                .await
            {
                Ok(a) => a,
                Err(e) => {
                    return json!({ "error": format!("Failed to add agent '{}': {}", name, e) })
                }
            };

            if let Err(e) = repo
                .link_roster_agent_to_child_step(roster_agent.id, Some(step_added.step_id))
                .await
            {
                return json!({ "error": format!("Failed to link '{}': {}", name, e) });
            }

            created_count += 1;
            agent_results.push(json!({ "name": name, "status": "created" }));
        }
    }

    // Remove agents not in desired spec
    for current in &current_roster {
        if !matched_current_ids.contains(&current.id) {
            if let Some(child_step_id) = current.child_step_id {
                if let Err(e) = pipeline::remove_step(repo, &pip_ctx, child_step_id).await {
                    return json!({ "error": format!("Pipeline error removing '{}': {}", current.name, e) });
                }
            }
            if let Err(e) = repo.remove_roster_agent(current.id).await {
                return json!({ "error": format!("Failed to remove agent '{}': {}", current.name, e) });
            }
            agent_results.push(json!({ "name": current.name, "status": "removed" }));
        }
    }

    // --- Diff dependencies ---------------------------------------------
    // Reload roster to get fresh child_step_ids (including new agents)
    let updated_roster = repo.list_agent_roster(brief_id).await.unwrap_or_default();
    let mut dep_results: Vec<Value> = Vec::new();

    // Get child_workflow_id for edge operations
    let child_workflow_id = match repo.get_step(ctx.step_id).await {
        Ok(Some(s)) => s.child_workflow_id,
        _ => None,
    };

    if let Some(child_wf_id) = child_workflow_id {
        // Build agent name → child_step_id lookup
        let name_to_step: HashMap<String, Uuid> = updated_roster
            .iter()
            .filter_map(|a| a.child_step_id.map(|sid| (normalize_name(&a.name), sid)))
            .collect();

        // Build desired edge set
        let desired_edges: HashSet<(Uuid, Uuid)> = desired_deps
            .iter()
            .filter_map(|(from, to)| {
                let from_sid = name_to_step.get(&normalize_name(from))?;
                let to_sid = name_to_step.get(&normalize_name(to))?;
                Some((*from_sid, *to_sid))
            })
            .collect();

        // Load current edges (filter to agent-only edges)
        let all_edges = repo.list_edges(child_wf_id).await.unwrap_or_default();
        let agent_step_ids: HashSet<Uuid> = name_to_step.values().copied().collect();
        let current_edges: HashSet<(Uuid, Uuid)> = all_edges
            .iter()
            .filter(|e| {
                agent_step_ids.contains(&e.from_step_id) && agent_step_ids.contains(&e.to_step_id)
            })
            .map(|e| (e.from_step_id, e.to_step_id))
            .collect();

        // Reverse lookup: child_step_id → agent name
        let step_to_name: HashMap<Uuid, &str> = updated_roster
            .iter()
            .filter_map(|a| a.child_step_id.map(|sid| (sid, a.name.as_str())))
            .collect();

        // Add missing edges
        for &(from_sid, to_sid) in &desired_edges {
            if current_edges.contains(&(from_sid, to_sid)) {
                let from_name = step_to_name.get(&from_sid).unwrap_or(&"?");
                let to_name = step_to_name.get(&to_sid).unwrap_or(&"?");
                dep_results
                    .push(json!({ "from": from_name, "to": to_name, "status": "unchanged" }));
            } else {
                match pipeline::add_edge(repo, &pip_ctx, from_sid, to_sid).await {
                    Ok(_) => {
                        let from_name = step_to_name.get(&from_sid).unwrap_or(&"?");
                        let to_name = step_to_name.get(&to_sid).unwrap_or(&"?");
                        dep_results
                            .push(json!({ "from": from_name, "to": to_name, "status": "created" }));
                    }
                    Err(crate::server::services::ServiceError::Conflict(_)) => {
                        let from_name = step_to_name.get(&from_sid).unwrap_or(&"?");
                        let to_name = step_to_name.get(&to_sid).unwrap_or(&"?");
                        dep_results.push(
                            json!({ "from": from_name, "to": to_name, "status": "unchanged" }),
                        );
                    }
                    Err(crate::server::services::ServiceError::Validation(msg)) => {
                        let from_name = step_to_name.get(&from_sid).unwrap_or(&"?");
                        let to_name = step_to_name.get(&to_sid).unwrap_or(&"?");
                        return json!({
                            "error": format!(
                                "Dependency {} \u{2192} {} would create a cycle: {}",
                                from_name, to_name, msg
                            )
                        });
                    }
                    Err(e) => {
                        return json!({ "error": format!("Failed to create dependency: {}", e) });
                    }
                }
            }
        }

        // Remove extra edges
        for &(from_sid, to_sid) in &current_edges {
            if !desired_edges.contains(&(from_sid, to_sid)) {
                if let Err(e) = pipeline::remove_edge(repo, &pip_ctx, from_sid, to_sid).await {
                    return json!({ "error": format!("Failed to remove dependency: {}", e) });
                }
                let from_name = step_to_name.get(&from_sid).unwrap_or(&"?");
                let to_name = step_to_name.get(&to_sid).unwrap_or(&"?");
                dep_results.push(json!({ "from": from_name, "to": to_name, "status": "removed" }));
            }
        }
    }

    // --- Recompute execution order once --------------------------------
    let _ = recompute_execution_order(repo, ctx).await;

    // --- Return summary ------------------------------------------------
    json!({
        "task": { "description": task, "status": task_status },
        "agents": agent_results,
        "dependencies": dep_results,
    })
}
