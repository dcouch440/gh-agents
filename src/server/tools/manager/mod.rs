//! Topology tools for the manager builder (L2).
//!
//! These tools let the manager builder create and modify workflow topology
//! in composite operations (create_pipeline, create_parallel, insert_node, etc.)
//! rather than requiring individual step/edge CRUD calls.

pub mod resolve;

use serde_json::{json, Value};
use uuid::Uuid;

use crate::server::services::{edges, steps};
use crate::server::state::AppState;
use crate::types::UserId;

use resolve::{check_name_unique, resolve_node};
use steps::{CreateStepInput, StepPayload};

mod tests;

// ============================================================================
// Public API
// ============================================================================

/// Context for manager tool execution.
pub struct ManagerToolContext {
    pub workflow_id: Uuid,
    pub user_id: UserId,
}

/// Execute a manager topology tool by name.
pub async fn execute_manager_tool(
    name: &str,
    input: &Value,
    state: &AppState,
    ctx: &ManagerToolContext,
) -> Value {
    match name {
        "create_pipeline" => execute_create_pipeline(input, state, ctx).await,
        "create_parallel" => execute_create_parallel(input, state, ctx).await,
        "insert_node" => execute_insert_node(input, state, ctx).await,
        "remove_node" => execute_remove_node(input, state, ctx).await,
        "wire_edge" => execute_wire_edge(input, state, ctx).await,
        "remove_edge" => execute_remove_edge(input, state, ctx).await,
        _ => json!({ "error": format!("Unknown manager tool: {name}") }),
    }
}

// ============================================================================
// Helpers
// ============================================================================

/// Create a workforce step and auto-create its chat session.
///
/// Returns `(step_id, ref_id, session_id)` on success.
async fn create_workforce_node(
    state: &AppState,
    workflow_id: Uuid,
    user_id: UserId,
    name: &str,
    description: &str,
) -> Result<(Uuid, String, Uuid), String> {
    let step = steps::create_step(
        state.repos().workflows.as_ref(),
        CreateStepInput {
            workflow_id,
            user_id: user_id.0,
            payload: StepPayload {
                execution_mode: Some("workforce".to_string()),
                name: Some(name.to_string()),
                description: Some(description.to_string()),
                ..StepPayload::default()
            },
        },
    )
    .await
    .map_err(|e| format!("Failed to create node \"{name}\": {e}"))?;

    let ref_id = step.ref_id.clone().unwrap_or_default();

    // Auto-create chat session so dispatch_to_nodes can message this node
    let session_id = Uuid::new_v4();
    let title = format!("{} Chat", name);
    let draft_config = json!({
        "step_id": step.id.to_string(),
        "workflow_id": workflow_id.to_string(),
    });

    state
        .repos()
        .sessions
        .create_session(
            user_id,
            session_id,
            "step_chat",
            &title,
            None,
            Some(draft_config),
        )
        .await
        .map_err(|e| format!("Failed to create session for \"{name}\": {e}"))?;

    Ok((step.id, ref_id, session_id))
}

// ============================================================================
// Tool Handlers
// ============================================================================

async fn execute_create_pipeline(
    input: &Value,
    state: &AppState,
    ctx: &ManagerToolContext,
) -> Value {
    let Some(nodes) = input["nodes"].as_array() else {
        return json!({ "error": "Missing required parameter: nodes" });
    };
    if nodes.is_empty() {
        return json!({ "error": "nodes array must not be empty" });
    }

    let repo = state.repos().workflows.as_ref();

    // Resolve optional source node
    let source_step_id = if let Some(source) = input["source"].as_str() {
        match resolve_node(repo, ctx.workflow_id, source).await {
            Ok(step) => Some(step.id),
            Err(e) => return json!({ "error": e }),
        }
    } else {
        None
    };

    // Validate all names are unique before creating anything
    let existing_steps = match repo.list_steps(ctx.workflow_id).await {
        Ok(steps) => steps,
        Err(e) => return json!({ "error": format!("DB error: {e}") }),
    };

    for node in nodes {
        if let Some(name) = node["name"].as_str() {
            if let Err(e) = check_name_unique(&existing_steps, name) {
                return json!({ "error": e });
            }
        }
    }

    // Also check for duplicates within the input
    let mut seen_names: Vec<String> = Vec::new();
    for node in nodes {
        if let Some(name) = node["name"].as_str() {
            let lower = name.to_lowercase();
            if seen_names.contains(&lower) {
                return json!({ "error": format!("Duplicate node name in request: \"{name}\"") });
            }
            seen_names.push(lower);
        }
    }

    // Create nodes sequentially (order matters for edges)
    let mut created = Vec::new();
    for node in nodes {
        let name = node["name"].as_str().unwrap_or("Unnamed");
        let description = node["description"].as_str().unwrap_or("");

        match create_workforce_node(state, ctx.workflow_id, ctx.user_id, name, description).await {
            Ok((step_id, ref_id, _session_id)) => {
                created
                    .push(json!({ "ref": ref_id, "name": name, "step_id": step_id.to_string() }));
            }
            Err(e) => return json!({ "error": e }),
        }
    }

    // Wire sequential edges
    let user_id = ctx.user_id.0;
    let workflow_id = ctx.workflow_id;

    // Source → first node
    if let Some(source_id) = source_step_id {
        let first_id: Uuid = created[0]["step_id"].as_str().unwrap().parse().unwrap();
        if let Err(e) = edges::add_edge(repo, user_id, workflow_id, source_id, first_id).await {
            return json!({ "error": format!("Failed to wire source edge: {e}") });
        }
    }

    // Chain: node[0] → node[1] → ... → node[n]
    for i in 0..created.len().saturating_sub(1) {
        let from_id: Uuid = created[i]["step_id"].as_str().unwrap().parse().unwrap();
        let to_id: Uuid = created[i + 1]["step_id"].as_str().unwrap().parse().unwrap();
        if let Err(e) = edges::add_edge(repo, user_id, workflow_id, from_id, to_id).await {
            return json!({ "error": format!("Failed to wire edge: {e}") });
        }
    }

    // Strip step_id from response (internal detail)
    let nodes_response: Vec<Value> = created
        .iter()
        .map(|n| json!({ "ref": n["ref"], "name": n["name"] }))
        .collect();

    json!({ "nodes": nodes_response })
}

async fn execute_create_parallel(
    input: &Value,
    state: &AppState,
    ctx: &ManagerToolContext,
) -> Value {
    let Some(parallel) = input["parallel"].as_array() else {
        return json!({ "error": "Missing required parameter: parallel" });
    };
    if parallel.len() < 2 {
        return json!({ "error": "parallel array must have at least 2 nodes" });
    }

    let repo = state.repos().workflows.as_ref();

    // Resolve optional source and target
    let source_step_id = if let Some(source) = input["source"].as_str() {
        match resolve_node(repo, ctx.workflow_id, source).await {
            Ok(step) => Some(step.id),
            Err(e) => return json!({ "error": e }),
        }
    } else {
        None
    };

    let target_step_id = if let Some(target) = input["target"].as_str() {
        match resolve_node(repo, ctx.workflow_id, target).await {
            Ok(step) => Some(step.id),
            Err(e) => return json!({ "error": e }),
        }
    } else {
        None
    };

    // Validate names
    let existing_steps = match repo.list_steps(ctx.workflow_id).await {
        Ok(steps) => steps,
        Err(e) => return json!({ "error": format!("DB error: {e}") }),
    };

    let mut seen_names: Vec<String> = Vec::new();
    for node in parallel {
        if let Some(name) = node["name"].as_str() {
            if let Err(e) = check_name_unique(&existing_steps, name) {
                return json!({ "error": e });
            }
            let lower = name.to_lowercase();
            if seen_names.contains(&lower) {
                return json!({ "error": format!("Duplicate node name in request: \"{name}\"") });
            }
            seen_names.push(lower);
        }
    }

    // Create parallel nodes
    let mut created = Vec::new();
    for node in parallel {
        let name = node["name"].as_str().unwrap_or("Unnamed");
        let description = node["description"].as_str().unwrap_or("");

        match create_workforce_node(state, ctx.workflow_id, ctx.user_id, name, description).await {
            Ok((step_id, ref_id, _session_id)) => {
                created
                    .push(json!({ "ref": ref_id, "name": name, "step_id": step_id.to_string() }));
            }
            Err(e) => return json!({ "error": e }),
        }
    }

    let user_id = ctx.user_id.0;
    let workflow_id = ctx.workflow_id;

    // Wire source → each parallel node
    if let Some(source_id) = source_step_id {
        for node in &created {
            let node_id: Uuid = node["step_id"].as_str().unwrap().parse().unwrap();
            if let Err(e) = edges::add_edge(repo, user_id, workflow_id, source_id, node_id).await {
                return json!({ "error": format!("Failed to wire source edge: {e}") });
            }
        }
    }

    // Wire each parallel node → target
    if let Some(target_id) = target_step_id {
        for node in &created {
            let node_id: Uuid = node["step_id"].as_str().unwrap().parse().unwrap();
            if let Err(e) = edges::add_edge(repo, user_id, workflow_id, node_id, target_id).await {
                return json!({ "error": format!("Failed to wire target edge: {e}") });
            }
        }
    }

    let nodes_response: Vec<Value> = created
        .iter()
        .map(|n| json!({ "ref": n["ref"], "name": n["name"] }))
        .collect();

    json!({ "nodes": nodes_response })
}

async fn execute_insert_node(input: &Value, state: &AppState, ctx: &ManagerToolContext) -> Value {
    let Some(from_ref) = input["from"].as_str() else {
        return json!({ "error": "Missing required parameter: from" });
    };
    let Some(to_ref) = input["to"].as_str() else {
        return json!({ "error": "Missing required parameter: to" });
    };
    let Some(node_spec) = input["node"].as_object() else {
        return json!({ "error": "Missing required parameter: node" });
    };
    let Some(name) = node_spec.get("name").and_then(|v| v.as_str()) else {
        return json!({ "error": "node.name is required" });
    };

    let repo = state.repos().workflows.as_ref();

    // Resolve from and to nodes
    let from_step = match resolve_node(repo, ctx.workflow_id, from_ref).await {
        Ok(s) => s,
        Err(e) => return json!({ "error": e }),
    };
    let to_step = match resolve_node(repo, ctx.workflow_id, to_ref).await {
        Ok(s) => s,
        Err(e) => return json!({ "error": e }),
    };

    // Validate name uniqueness
    let existing_steps = match repo.list_steps(ctx.workflow_id).await {
        Ok(steps) => steps,
        Err(e) => return json!({ "error": format!("DB error: {e}") }),
    };
    if let Err(e) = check_name_unique(&existing_steps, name) {
        return json!({ "error": e });
    }

    // Verify edge exists between from → to
    let all_edges = match repo.list_edges(ctx.workflow_id).await {
        Ok(e) => e,
        Err(e) => return json!({ "error": format!("DB error: {e}") }),
    };
    let edge_exists = all_edges
        .iter()
        .any(|e| e.from_step_id == from_step.id && e.to_step_id == to_step.id);
    if !edge_exists {
        return json!({ "error": format!("No edge exists from \"{from_ref}\" to \"{to_ref}\"") });
    }

    let description = node_spec
        .get("description")
        .and_then(|v| v.as_str())
        .unwrap_or("");
    let user_id = ctx.user_id.0;
    let workflow_id = ctx.workflow_id;

    // Create the new node
    let (new_step_id, ref_id, _session_id) =
        match create_workforce_node(state, ctx.workflow_id, ctx.user_id, name, description).await {
            Ok(r) => r,
            Err(e) => return json!({ "error": e }),
        };

    // Remove old edge, wire through new node
    if let Err(e) = edges::remove_edge(repo, user_id, workflow_id, from_step.id, to_step.id).await {
        return json!({ "error": format!("Failed to remove old edge: {e}") });
    }
    if let Err(e) = edges::add_edge(repo, user_id, workflow_id, from_step.id, new_step_id).await {
        return json!({ "error": format!("Failed to wire from→new edge: {e}") });
    }
    if let Err(e) = edges::add_edge(repo, user_id, workflow_id, new_step_id, to_step.id).await {
        return json!({ "error": format!("Failed to wire new→to edge: {e}") });
    }

    json!({ "node": { "ref": ref_id, "name": name } })
}

async fn execute_remove_node(input: &Value, state: &AppState, ctx: &ManagerToolContext) -> Value {
    let Some(node_ref) = input["node"].as_str() else {
        return json!({ "error": "Missing required parameter: node" });
    };
    let reconnect = input["reconnect"].as_bool().unwrap_or(true);

    let repo = state.repos().workflows.as_ref();

    let step = match resolve_node(repo, ctx.workflow_id, node_ref).await {
        Ok(s) => s,
        Err(e) => return json!({ "error": e }),
    };

    let all_edges = match repo.list_edges(ctx.workflow_id).await {
        Ok(e) => e,
        Err(e) => return json!({ "error": format!("DB error: {e}") }),
    };

    // Find predecessors and successors
    let predecessors: Vec<Uuid> = all_edges
        .iter()
        .filter(|e| e.to_step_id == step.id)
        .map(|e| e.from_step_id)
        .collect();
    let successors: Vec<Uuid> = all_edges
        .iter()
        .filter(|e| e.from_step_id == step.id)
        .map(|e| e.to_step_id)
        .collect();

    let user_id = ctx.user_id.0;
    let workflow_id = ctx.workflow_id;

    // Reconnect predecessors → successors if requested
    if reconnect {
        for &pred in &predecessors {
            for &succ in &successors {
                if let Err(e) = edges::add_edge(repo, user_id, workflow_id, pred, succ).await {
                    return json!({ "error": format!("Failed to reconnect: {e}") });
                }
            }
        }
    }

    // Delete the step (also cleans up session)
    if let Err(e) = steps::delete_step(
        repo,
        state.repos().sessions.as_ref(),
        user_id,
        workflow_id,
        step.id,
    )
    .await
    {
        return json!({ "error": format!("Failed to delete node: {e}") });
    }

    json!({
        "removed": node_ref,
        "reconnected": reconnect,
    })
}

async fn execute_wire_edge(input: &Value, state: &AppState, ctx: &ManagerToolContext) -> Value {
    let Some(from_ref) = input["from"].as_str() else {
        return json!({ "error": "Missing required parameter: from" });
    };
    let Some(to_ref) = input["to"].as_str() else {
        return json!({ "error": "Missing required parameter: to" });
    };

    let repo = state.repos().workflows.as_ref();

    let from_step = match resolve_node(repo, ctx.workflow_id, from_ref).await {
        Ok(s) => s,
        Err(e) => return json!({ "error": e }),
    };
    let to_step = match resolve_node(repo, ctx.workflow_id, to_ref).await {
        Ok(s) => s,
        Err(e) => return json!({ "error": e }),
    };

    match edges::add_edge(
        repo,
        ctx.user_id.0,
        ctx.workflow_id,
        from_step.id,
        to_step.id,
    )
    .await
    {
        Ok(_) => json!({ "status": "ok", "from": from_ref, "to": to_ref }),
        Err(e) => json!({ "error": format!("Failed to add edge: {e}") }),
    }
}

async fn execute_remove_edge(input: &Value, state: &AppState, ctx: &ManagerToolContext) -> Value {
    let Some(from_ref) = input["from"].as_str() else {
        return json!({ "error": "Missing required parameter: from" });
    };
    let Some(to_ref) = input["to"].as_str() else {
        return json!({ "error": "Missing required parameter: to" });
    };

    let repo = state.repos().workflows.as_ref();

    let from_step = match resolve_node(repo, ctx.workflow_id, from_ref).await {
        Ok(s) => s,
        Err(e) => return json!({ "error": e }),
    };
    let to_step = match resolve_node(repo, ctx.workflow_id, to_ref).await {
        Ok(s) => s,
        Err(e) => return json!({ "error": e }),
    };

    match edges::remove_edge(
        repo,
        ctx.user_id.0,
        ctx.workflow_id,
        from_step.id,
        to_step.id,
    )
    .await
    {
        Ok(_) => json!({ "status": "ok", "from": from_ref, "to": to_ref }),
        Err(e) => json!({ "error": format!("Failed to remove edge: {e}") }),
    }
}
