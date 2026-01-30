//! Agent management tools for the orchestrator.
//!
//! Defines tool schemas (for the Anthropic tool use API) and execution
//! handlers that let the orchestrator LLM create, list, and assign agents.

use std::collections::HashMap;
use std::time::Duration;

use serde_json::{json, Value};

use crate::agents::{
    AgentCommand, AgentResponse, ClusterId, CommunicationStyle, OutputFormat, RoleContext, RoleId,
    TaskAssignment, TaskConstraints, TaskContext,
};
use crate::db::{AgentRow, ClusterRow};
use crate::llm::Tool;
use crate::types::{AgentPersona, AgentTier, ModelConfig, UserId};

use super::state::AppState;

/// Return all agent management tool definitions for the Anthropic API.
pub fn agent_tools() -> Vec<Tool> {
    vec![
        Tool {
            name: "list_agents".to_string(),
            description: "List all agents in the pool with their status and tier.".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {},
                "required": []
            }),
        },
        Tool {
            name: "list_roles".to_string(),
            description: "List all available roles with their descriptions, categories, and communication styles. Use this to choose a role when assigning tasks.".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {},
                "required": []
            }),
        },
        Tool {
            name: "create_agent".to_string(),
            description: "Create a new agent in the pool. Returns the agent ID.".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "tier": {
                        "type": "string",
                        "enum": ["orchestrator", "worker", "utility"],
                        "description": "The agent tier: orchestrator (planning/review), worker (implementation), or utility (simple tasks)"
                    },
                    "name": {
                        "type": "string",
                        "description": "Optional display name for the agent"
                    }
                },
                "required": ["tier"]
            }),
        },
        Tool {
            name: "assign_task".to_string(),
            description: "Assign a task to an agent by its ID. The agent will begin working on it."
                .to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "agent_id": {
                        "type": "string",
                        "description": "The UUID of the agent to assign the task to"
                    },
                    "title": {
                        "type": "string",
                        "description": "Short title for the task"
                    },
                    "description": {
                        "type": "string",
                        "description": "Detailed description of what the agent should do"
                    },
                    "role": {
                        "type": "string",
                        "enum": ["orchestrator", "worker", "utility", "reviewer", "summarizer", "complaint-finder", "risk-assessor", "scope-definer"],
                        "description": "Optional role for the agent. Defaults to 'worker'. Each role has specialized prompts and context."
                    },
                    "allowed_tools": {
                        "type": "array",
                        "items": { "type": "string" },
                        "description": "Optional list of execution tool names this agent can use. If omitted, all tools are available. Options: read_file, write_file, list_files, git_status, git_diff, git_add, git_commit, git_branch, run_tests, run_command"
                    }
                },
                "required": ["agent_id", "title", "description"]
            }),
        },
        Tool {
            name: "get_task_result".to_string(),
            description:
                "Check the result of a previously assigned task. Returns pending, started, in_progress, completed, or failed."
                    .to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "task_id": {
                        "type": "string",
                        "description": "The task UUID returned by assign_task"
                    }
                },
                "required": ["task_id"]
            }),
        },
        Tool {
            name: "list_pending_approvals".to_string(),
            description: "List all pending approval requests from agents that need a decision."
                .to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {},
                "required": []
            }),
        },
        Tool {
            name: "respond_to_approval".to_string(),
            description:
                "Approve or deny a pending approval request from an agent."
                    .to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "agent_id": {
                        "type": "string",
                        "description": "The UUID of the agent that requested approval"
                    },
                    "approved": {
                        "type": "boolean",
                        "description": "True to approve, false to deny"
                    },
                    "reason": {
                        "type": "string",
                        "description": "Reason for denial (required when denying)"
                    }
                },
                "required": ["agent_id", "approved"]
            }),
        },
        Tool {
            name: "remove_agent".to_string(),
            description: "Remove an agent from the pool by its ID.".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "agent_id": {
                        "type": "string",
                        "description": "The UUID of the agent to remove"
                    }
                },
                "required": ["agent_id"]
            }),
        },
        Tool {
            name: "create_cluster".to_string(),
            description: "Create a named cluster for grouping agents that share context.".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "name": {
                        "type": "string",
                        "description": "Name for the cluster (e.g. 'frontend', 'backend-api')"
                    },
                    "description": {
                        "type": "string",
                        "description": "Optional description of the cluster's purpose"
                    }
                },
                "required": ["name"]
            }),
        },
        Tool {
            name: "add_to_cluster".to_string(),
            description: "Add an agent to a cluster. The agent will receive the cluster's shared context with every task.".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "cluster_id": {
                        "type": "string",
                        "description": "The UUID of the cluster"
                    },
                    "agent_id": {
                        "type": "string",
                        "description": "The UUID of the agent to add"
                    }
                },
                "required": ["cluster_id", "agent_id"]
            }),
        },
        Tool {
            name: "remove_from_cluster".to_string(),
            description: "Remove an agent from a cluster.".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "cluster_id": {
                        "type": "string",
                        "description": "The UUID of the cluster"
                    },
                    "agent_id": {
                        "type": "string",
                        "description": "The UUID of the agent to remove"
                    }
                },
                "required": ["cluster_id", "agent_id"]
            }),
        },
        Tool {
            name: "list_clusters".to_string(),
            description: "List all clusters with their members.".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {},
                "required": []
            }),
        },
    ]
}

/// Execute a tool by name with the given JSON input.
///
/// Returns a JSON value describing the result.
pub async fn execute_tool(name: &str, input: &Value, state: &AppState, user_id: UserId) -> Value {
    match name {
        "list_agents" => execute_list_agents(state).await,
        "list_roles" => execute_list_roles(state).await,
        "create_agent" => execute_create_agent(input, state, user_id).await,
        "assign_task" => execute_assign_task(input, state).await,
        "get_task_result" => execute_get_task_result(input, state).await,
        "list_pending_approvals" => execute_list_pending_approvals(state).await,
        "respond_to_approval" => execute_respond_to_approval(input, state).await,
        "remove_agent" => execute_remove_agent(input, state).await,
        "create_cluster" => execute_create_cluster(input, state, user_id).await,
        "add_to_cluster" => execute_add_to_cluster(input, state).await,
        "remove_from_cluster" => execute_remove_from_cluster(input, state).await,
        "list_clusters" => execute_list_clusters(state).await,
        _ => json!({ "error": format!("Unknown tool: {}", name) }),
    }
}

async fn execute_list_agents(state: &AppState) -> Value {
    let Some(pool) = &state.pool else {
        return json!({ "error": "Agent pool not initialized" });
    };

    let pool = pool.lock().await;
    let stats = pool.stats();

    json!({
        "orchestrators": {
            "total": stats.orchestrators.total,
            "available": stats.orchestrators.available,
            "max": stats.orchestrators.max
        },
        "workers": {
            "total": stats.workers.total,
            "available": stats.workers.available,
            "max": stats.workers.max
        },
        "utilities": {
            "total": stats.utilities.total,
            "available": stats.utilities.available,
            "max": stats.utilities.max
        }
    })
}

async fn execute_list_roles(state: &AppState) -> Value {
    let Some(rm) = &state.role_manager else {
        return json!({ "error": "Role manager not initialized" });
    };

    let roles: Vec<Value> = rm
        .library()
        .list_all()
        .iter()
        .map(|role| {
            json!({
                "id": role.id.0.as_str(),
                "name": role.name,
                "category": format!("{:?}", role.category),
                "description": role.description,
                "style": format!("{:?}", role.style),
                "output_format": format!("{:?}", role.output_format),
                "can_delegate_to": role.can_delegate_to.iter().map(|r| r.0.as_str()).collect::<Vec<_>>(),
                "is_custom": role.is_custom,
            })
        })
        .collect();

    json!({
        "roles": roles,
        "count": roles.len()
    })
}

async fn execute_create_agent(input: &Value, state: &AppState, user_id: UserId) -> Value {
    let Some(pool) = &state.pool else {
        return json!({ "error": "Agent pool not initialized" });
    };
    let Some(dispatcher) = &state.dispatcher else {
        return json!({ "error": "Dispatcher not initialized" });
    };

    let tier_str = input["tier"].as_str().unwrap_or("worker");
    let tier = match tier_str {
        "orchestrator" => AgentTier::Orchestrator,
        "worker" => AgentTier::Worker,
        "utility" => AgentTier::Utility,
        other => return json!({ "error": format!("Invalid tier: {}", other) }),
    };

    let name = input["name"]
        .as_str()
        .unwrap_or(tier_str)
        .to_string();

    let persona = AgentPersona {
        name: name.clone(),
        ..Default::default()
    };

    let model_config = ModelConfig::default();
    let mut pool = pool.lock().await;
    let mut dispatcher = dispatcher.lock().await;

    match pool.spawn_agent_with_dispatcher(tier, persona, model_config.clone(), &mut dispatcher) {
        Ok(agent_id) => {
            // Persist to DB
            if let Err(e) = state.repo.upsert_agent(user_id, AgentRow {
                id: agent_id.0,
                tier: tier_str.to_string(),
                persona_name: name.clone(),
                model_id: model_config.model_id.clone(),
                status: "idle".to_string(),
            }).await {
                tracing::error!("Failed to persist agent: {}", e);
            }

            json!({
                "agent_id": agent_id.0.to_string(),
                "tier": tier_str,
                "name": name,
                "status": "created"
            })
        }
        Err(e) => json!({ "error": e.to_string() }),
    }
}

async fn execute_assign_task(input: &Value, state: &AppState) -> Value {
    let Some(dispatcher) = &state.dispatcher else {
        return json!({ "error": "Dispatcher not initialized" });
    };

    let Some(id_str) = input["agent_id"].as_str() else {
        return json!({ "error": "agent_id is required" });
    };
    let Some(title) = input["title"].as_str() else {
        return json!({ "error": "title is required" });
    };
    let Some(description) = input["description"].as_str() else {
        return json!({ "error": "description is required" });
    };

    let Ok(uuid) = uuid::Uuid::parse_str(id_str) else {
        return json!({ "error": format!("Invalid UUID: {}", id_str) });
    };

    let role_str = input["role"].as_str().unwrap_or("worker");
    let role_id = RoleId::new(role_str);

    // Build role-aware context if RoleManager is available
    let (system_prompt, style, output_format, required_reading) =
        if let Some(rm) = &state.role_manager {
            if let Some(role) = rm.get_role(&role_id) {
                let vars = HashMap::new();
                let ctx = rm.build_context_for_role(role, &vars).await;
                let prompt = ctx.build_system_prompt();
                let s = role.style;
                let fmt = role.output_format.clone();
                let files = ctx
                    .loaded_files
                    .into_iter()
                    .map(|f| crate::agents::FileContent {
                        path: f.path,
                        content: f.content,
                    })
                    .collect();
                (prompt, s, fmt, files)
            } else {
                // Unknown role, fall back to defaults
                (
                    format!("You are a {} working on: {}", role_str, title),
                    CommunicationStyle::Technical,
                    OutputFormat::CodeAndReport,
                    vec![],
                )
            }
        } else {
            (
                format!("You are a {} working on: {}", role_str, title),
                CommunicationStyle::Technical,
                OutputFormat::CodeAndReport,
                vec![],
            )
        };

    let agent_id = crate::agents::AgentId(uuid);

    // Look up cluster context for this agent
    let cluster_mgr = state.cluster_manager.read().await;
    let (cluster_conventions, cluster_files) =
        if let Some(cluster) = cluster_mgr.get_agent_cluster(&agent_id) {
            (
                cluster.shared_context.conventions.clone(),
                cluster.shared_context.shared_files.clone(),
            )
        } else {
            (String::new(), vec![])
        };
    drop(cluster_mgr);

    // Build execution context from project root
    let project_root = std::env::current_dir().unwrap_or_default();
    let execution_context = Some(crate::execution::ExecutionContext::new(project_root));

    // Parse allowed_tools if provided
    let allowed_tools = input["allowed_tools"]
        .as_array()
        .map(|arr| arr.iter().filter_map(|v| v.as_str().map(String::from)).collect());

    let mut constraints = TaskConstraints::default();
    constraints.allowed_tools = allowed_tools;

    let assignment = TaskAssignment {
        task_id: uuid::Uuid::new_v4(),
        title: title.to_string(),
        description: description.to_string(),
        context: TaskContext {
            required_reading,
            files: cluster_files,
            history: vec![],
            conventions: cluster_conventions,
            role_context: RoleContext {
                system_prompt,
                style,
                output_format,
            },
            chat_messages: vec![],
            execution_context,
        },
        constraints,
        timeout: Duration::from_secs(300),
        role_id,
    };

    let task_id = assignment.task_id;
    let dispatcher = dispatcher.lock().await;
    match dispatcher
        .send_to_agent(&agent_id, AgentCommand::AssignTask(assignment))
        .await
    {
        Ok(()) => json!({
            "status": "assigned",
            "task_id": task_id.to_string(),
            "agent_id": id_str,
            "title": title,
            "role": role_str
        }),
        Err(e) => json!({ "error": e.to_string() }),
    }
}

async fn execute_get_task_result(input: &Value, state: &AppState) -> Value {
    let Some(id_str) = input["task_id"].as_str() else {
        return json!({ "error": "task_id is required" });
    };
    let Ok(task_id) = uuid::Uuid::parse_str(id_str) else {
        return json!({ "error": format!("Invalid UUID: {}", id_str) });
    };

    let results = state.task_results.read().await;
    match results.get(&task_id) {
        None => json!({ "status": "pending", "task_id": id_str }),
        Some(AgentResponse::TaskStarted { .. }) => {
            json!({ "status": "started", "task_id": id_str })
        }
        Some(AgentResponse::TaskCompleted { result, .. }) => {
            json!({
                "status": "completed",
                "task_id": id_str,
                "output": result.output,
                "files_modified": result.files_modified,
            })
        }
        Some(AgentResponse::TaskFailed { result, .. }) => {
            json!({
                "status": "failed",
                "task_id": id_str,
                "errors": result.errors,
                "output": result.output,
            })
        }
        Some(AgentResponse::ProgressUpdate { update, .. }) => {
            json!({
                "status": "in_progress",
                "task_id": id_str,
                "message": update.message,
                "progress_percent": update.progress_percent,
            })
        }
        Some(_) => json!({ "status": "unknown", "task_id": id_str }),
    }
}

async fn execute_list_pending_approvals(state: &AppState) -> Value {
    let results = state.task_results.read().await;
    let pending: Vec<Value> = results
        .values()
        .filter_map(|resp| {
            if let AgentResponse::ApprovalRequest { agent_id, request } = resp {
                Some(json!({
                    "agent_id": agent_id.0.to_string(),
                    "task_id": request.task_id.to_string(),
                    "action": request.action,
                    "details": request.details,
                }))
            } else {
                None
            }
        })
        .collect();

    json!({
        "pending_approvals": pending,
        "count": pending.len()
    })
}

async fn execute_respond_to_approval(input: &Value, state: &AppState) -> Value {
    let Some(dispatcher) = &state.dispatcher else {
        return json!({ "error": "Dispatcher not initialized" });
    };

    let Some(id_str) = input["agent_id"].as_str() else {
        return json!({ "error": "agent_id is required" });
    };
    let approved = input["approved"].as_bool().unwrap_or(false);

    let Ok(uuid) = uuid::Uuid::parse_str(id_str) else {
        return json!({ "error": format!("Invalid UUID: {}", id_str) });
    };

    let agent_id = crate::agents::AgentId(uuid);

    let command = if approved {
        AgentCommand::GrantApproval
    } else {
        let reason = input["reason"]
            .as_str()
            .unwrap_or("Denied by orchestrator")
            .to_string();
        AgentCommand::DenyApproval { reason }
    };

    let dispatcher = dispatcher.lock().await;
    match dispatcher.send_to_agent(&agent_id, command).await {
        Ok(()) => json!({
            "status": if approved { "approved" } else { "denied" },
            "agent_id": id_str
        }),
        Err(e) => json!({ "error": e.to_string() }),
    }
}

async fn execute_create_cluster(input: &Value, state: &AppState, user_id: UserId) -> Value {
    let Some(name) = input["name"].as_str() else {
        return json!({ "error": "name is required" });
    };
    let description = input["description"].as_str().unwrap_or("").to_string();

    let mut mgr = state.cluster_manager.write().await;
    let id = mgr.create_cluster(name.to_string(), description.clone());

    // Persist to DB
    if let Err(e) = state.repo.upsert_cluster(user_id, ClusterRow {
        id: id.0,
        name: name.to_string(),
        description,
        conventions: String::new(),
        shared_files: serde_json::json!([]),
    }).await {
        tracing::error!("Failed to persist cluster: {}", e);
    }

    json!({
        "status": "created",
        "cluster_id": id.0.to_string(),
        "name": name
    })
}

async fn execute_add_to_cluster(input: &Value, state: &AppState) -> Value {
    let Some(cluster_str) = input["cluster_id"].as_str() else {
        return json!({ "error": "cluster_id is required" });
    };
    let Some(agent_str) = input["agent_id"].as_str() else {
        return json!({ "error": "agent_id is required" });
    };

    let Ok(cluster_uuid) = uuid::Uuid::parse_str(cluster_str) else {
        return json!({ "error": format!("Invalid cluster UUID: {}", cluster_str) });
    };
    let Ok(agent_uuid) = uuid::Uuid::parse_str(agent_str) else {
        return json!({ "error": format!("Invalid agent UUID: {}", agent_str) });
    };

    let mut mgr = state.cluster_manager.write().await;
    match mgr.add_agent(ClusterId(cluster_uuid), crate::agents::AgentId(agent_uuid)) {
        Ok(()) => {
            if let Err(e) = state.repo.add_cluster_member(cluster_uuid, agent_uuid).await {
                tracing::error!("Failed to persist cluster member: {}", e);
            }
            json!({
                "status": "added",
                "cluster_id": cluster_str,
                "agent_id": agent_str
            })
        }
        Err(e) => json!({ "error": e.to_string() }),
    }
}

async fn execute_remove_from_cluster(input: &Value, state: &AppState) -> Value {
    let Some(cluster_str) = input["cluster_id"].as_str() else {
        return json!({ "error": "cluster_id is required" });
    };
    let Some(agent_str) = input["agent_id"].as_str() else {
        return json!({ "error": "agent_id is required" });
    };

    let Ok(cluster_uuid) = uuid::Uuid::parse_str(cluster_str) else {
        return json!({ "error": format!("Invalid cluster UUID: {}", cluster_str) });
    };
    let Ok(agent_uuid) = uuid::Uuid::parse_str(agent_str) else {
        return json!({ "error": format!("Invalid agent UUID: {}", agent_str) });
    };

    let mut mgr = state.cluster_manager.write().await;
    match mgr.remove_agent(ClusterId(cluster_uuid), crate::agents::AgentId(agent_uuid)) {
        Ok(()) => {
            if let Err(e) = state.repo.remove_cluster_member(cluster_uuid, agent_uuid).await {
                tracing::error!("Failed to persist cluster member removal: {}", e);
            }
            json!({
                "status": "removed",
                "cluster_id": cluster_str,
                "agent_id": agent_str
            })
        }
        Err(e) => json!({ "error": e.to_string() }),
    }
}

async fn execute_list_clusters(state: &AppState) -> Value {
    let mgr = state.cluster_manager.read().await;
    let clusters: Vec<Value> = mgr
        .list_clusters()
        .iter()
        .map(|c| {
            json!({
                "id": c.id.0.to_string(),
                "name": c.name,
                "description": c.description,
                "member_count": c.members.len(),
                "members": c.members.iter().map(|a| a.0.to_string()).collect::<Vec<_>>(),
            })
        })
        .collect();

    json!({
        "clusters": clusters,
        "count": clusters.len()
    })
}

async fn execute_remove_agent(input: &Value, state: &AppState) -> Value {
    let Some(pool) = &state.pool else {
        return json!({ "error": "Agent pool not initialized" });
    };

    let Some(id_str) = input["agent_id"].as_str() else {
        return json!({ "error": "agent_id is required" });
    };

    let Ok(uuid) = uuid::Uuid::parse_str(id_str) else {
        return json!({ "error": format!("Invalid UUID: {}", id_str) });
    };

    let agent_id = crate::agents::AgentId(uuid);

    let mut pool = pool.lock().await;
    match pool.remove_agent(&agent_id) {
        Ok(()) => {
            if let Err(e) = state.repo.delete_persisted_agent(uuid).await {
                tracing::error!("Failed to delete persisted agent: {}", e);
            }
            json!({ "status": "removed", "agent_id": id_str })
        }
        Err(e) => json!({ "error": e.to_string() }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn agent_tools_returns_twelve_tools() {
        let tools = agent_tools();
        assert_eq!(tools.len(), 12);
        assert_eq!(tools[0].name, "list_agents");
        assert_eq!(tools[1].name, "list_roles");
        assert_eq!(tools[2].name, "create_agent");
        assert_eq!(tools[3].name, "assign_task");
        assert_eq!(tools[4].name, "get_task_result");
        assert_eq!(tools[5].name, "list_pending_approvals");
        assert_eq!(tools[6].name, "respond_to_approval");
        assert_eq!(tools[7].name, "remove_agent");
        assert_eq!(tools[8].name, "create_cluster");
        assert_eq!(tools[9].name, "add_to_cluster");
        assert_eq!(tools[10].name, "remove_from_cluster");
        assert_eq!(tools[11].name, "list_clusters");
    }

    #[test]
    fn tool_schemas_are_valid_json() {
        for tool in agent_tools() {
            assert!(tool.input_schema.is_object());
            assert!(tool.input_schema["type"].as_str() == Some("object"));
        }
    }
}
