//! Agent management tools for the orchestrator.
//!
//! Defines tool schemas (for the Anthropic tool use API) and execution
//! handlers that let the orchestrator LLM create, list, and assign agents.

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
        "create_agent" => execute_create_agent(input, state).await,
        "assign_task" => execute_assign_task(input, state).await,
        "get_task_result" => execute_get_task_result(input, state).await,
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

    let agent_id = crate::agents::AgentId(uuid);

    let assignment = TaskAssignment {
        task_id: uuid::Uuid::new_v4(),
        title: title.to_string(),
        description: description.to_string(),
        context: TaskContext {
            required_reading: vec![],
            files: vec![],
            history: vec![],
            conventions: String::new(),
            role_context: RoleContext {
                system_prompt: format!("You are working on: {}", title),
                style: CommunicationStyle::Technical,
                output_format: OutputFormat::CodeAndReport,
            },
            chat_messages: vec![],
        },
        constraints: TaskConstraints::default(),
        timeout: Duration::from_secs(300),
        role_id: RoleId::new("worker"),
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
            "title": title
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
    fn agent_tools_returns_five_tools() {
        let tools = agent_tools();
        assert_eq!(tools.len(), 5);
        assert_eq!(tools[0].name, "list_agents");
        assert_eq!(tools[1].name, "create_agent");
        assert_eq!(tools[2].name, "assign_task");
        assert_eq!(tools[3].name, "get_task_result");
        assert_eq!(tools[4].name, "remove_agent");
    }

    #[test]
    fn tool_schemas_are_valid_json() {
        for tool in agent_tools() {
            assert!(tool.input_schema.is_object());
            assert!(tool.input_schema["type"].as_str() == Some("object"));
        }
    }
}
