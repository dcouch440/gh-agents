//! Agent management tools for the orchestrator.
//!
//! Defines tool schemas (for the Anthropic tool use API) and execution
//! handlers that let the orchestrator LLM create, list, and assign agents.

use std::collections::HashMap;
use std::time::Duration;

use serde_json::{json, Value};

use crate::agents::{
    AgentCommand, AgentResponse, CommunicationStyle, OutputFormat, RoleContext, RoleId,
    TaskAssignment, TaskConstraints, TaskContext,
};
use crate::llm::Tool;
use crate::types::{AgentPersona, AgentTier, ModelConfig};

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
    ]
}

/// Execute a tool by name with the given JSON input.
///
/// Returns a JSON value describing the result.
pub async fn execute_tool(name: &str, input: &Value, state: &AppState) -> Value {
    match name {
        "list_agents" => execute_list_agents(state).await,
        "list_roles" => execute_list_roles(state).await,
        "create_agent" => execute_create_agent(input, state).await,
        "assign_task" => execute_assign_task(input, state).await,
        "get_task_result" => execute_get_task_result(input, state).await,
        "list_pending_approvals" => execute_list_pending_approvals(state).await,
        "respond_to_approval" => execute_respond_to_approval(input, state).await,
        "remove_agent" => execute_remove_agent(input, state).await,
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

async fn execute_create_agent(input: &Value, state: &AppState) -> Value {
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

    let mut pool = pool.lock().await;
    let mut dispatcher = dispatcher.lock().await;

    match pool.spawn_agent_with_dispatcher(tier, persona, ModelConfig::default(), &mut dispatcher) {
        Ok(agent_id) => json!({
            "agent_id": agent_id.0.to_string(),
            "tier": tier_str,
            "name": name,
            "status": "created"
        }),
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

    let assignment = TaskAssignment {
        task_id: uuid::Uuid::new_v4(),
        title: title.to_string(),
        description: description.to_string(),
        context: TaskContext {
            required_reading,
            files: vec![],
            history: vec![],
            conventions: String::new(),
            role_context: RoleContext {
                system_prompt,
                style,
                output_format,
            },
            chat_messages: vec![],
        },
        constraints: TaskConstraints::default(),
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
        Ok(()) => json!({ "status": "removed", "agent_id": id_str }),
        Err(e) => json!({ "error": e.to_string() }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn agent_tools_returns_eight_tools() {
        let tools = agent_tools();
        assert_eq!(tools.len(), 8);
        assert_eq!(tools[0].name, "list_agents");
        assert_eq!(tools[1].name, "list_roles");
        assert_eq!(tools[2].name, "create_agent");
        assert_eq!(tools[3].name, "assign_task");
        assert_eq!(tools[4].name, "get_task_result");
        assert_eq!(tools[5].name, "list_pending_approvals");
        assert_eq!(tools[6].name, "respond_to_approval");
        assert_eq!(tools[7].name, "remove_agent");
    }

    #[test]
    fn tool_schemas_are_valid_json() {
        for tool in agent_tools() {
            assert!(tool.input_schema.is_object());
            assert!(tool.input_schema["type"].as_str() == Some("object"));
        }
    }
}
