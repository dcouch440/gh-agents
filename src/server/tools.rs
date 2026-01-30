//! Agent management tools for the orchestrator.
//!
//! Defines tool schemas (for the Anthropic tool use API) and execution
//! handlers that let the orchestrator LLM create, list, and assign agents.

use std::collections::HashMap;
use std::time::Duration;

use serde_json::{json, Value};

use crate::agents::{
    AgentCommand, AgentResponse, ClusterId, CommunicationStyle, OutputFormat, PipelineId,
    RoleContext, RoleId, ScheduleId, TaskAssignment, TaskConstraints, TaskContext, TriggerEvent,
};
use crate::db::{AgentRow, ClusterRow, PipelineRow, PipelineStageRow, ScheduleRow, TriggerRow};
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
        Tool {
            name: "create_pipeline".to_string(),
            description: "Create a named pipeline for chaining agent workflows. Stages are added separately.".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "name": {
                        "type": "string",
                        "description": "Name for the pipeline (e.g. 'code-review', 'deploy')"
                    }
                },
                "required": ["name"]
            }),
        },
        Tool {
            name: "add_pipeline_stage".to_string(),
            description: "Append a stage to a pipeline. Each stage assigns a task to an agent. Stages run in order, with the previous stage's output fed as context.".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "pipeline_id": {
                        "type": "string",
                        "description": "The UUID of the pipeline"
                    },
                    "agent_id": {
                        "type": "string",
                        "description": "The UUID of the agent for this stage"
                    },
                    "role": {
                        "type": "string",
                        "description": "Optional role for the agent at this stage"
                    },
                    "approval_required": {
                        "type": "boolean",
                        "description": "If true, the pipeline pauses for approval before this stage's output advances"
                    }
                },
                "required": ["pipeline_id", "agent_id"]
            }),
        },
        Tool {
            name: "start_pipeline".to_string(),
            description: "Start a pipeline run. Assigns the first stage's agent immediately. Subsequent stages auto-trigger on completion.".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "pipeline_id": {
                        "type": "string",
                        "description": "The UUID of the pipeline to run"
                    },
                    "task": {
                        "type": "string",
                        "description": "The task description to pass through the pipeline"
                    }
                },
                "required": ["pipeline_id", "task"]
            }),
        },
        Tool {
            name: "get_pipeline_status".to_string(),
            description: "Get the status of a pipeline run including current stage and per-stage results.".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "run_id": {
                        "type": "string",
                        "description": "The UUID of the pipeline run"
                    }
                },
                "required": ["run_id"]
            }),
        },
        Tool {
            name: "create_schedule".to_string(),
            description: "Create a periodic schedule that assigns a task to an agent at a fixed interval.".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "name": {
                        "type": "string",
                        "description": "Name for the schedule (e.g. 'hourly-tests')"
                    },
                    "agent_id": {
                        "type": "string",
                        "description": "The UUID of the agent to run"
                    },
                    "interval_seconds": {
                        "type": "integer",
                        "description": "Interval in seconds between runs (e.g. 3600 for hourly)"
                    },
                    "task_title": {
                        "type": "string",
                        "description": "Title for the scheduled tasks"
                    },
                    "task_description": {
                        "type": "string",
                        "description": "Description template for the scheduled tasks"
                    },
                    "role": {
                        "type": "string",
                        "description": "Optional role for the agent when running"
                    }
                },
                "required": ["name", "agent_id", "interval_seconds", "task_title", "task_description"]
            }),
        },
        Tool {
            name: "list_schedules".to_string(),
            description: "List all periodic schedules with their status (enabled, last run time, interval).".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {},
                "required": []
            }),
        },
        Tool {
            name: "toggle_schedule".to_string(),
            description: "Enable or disable a periodic schedule.".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "schedule_id": {
                        "type": "string",
                        "description": "The UUID of the schedule"
                    },
                    "enabled": {
                        "type": "boolean",
                        "description": "True to enable, false to disable"
                    }
                },
                "required": ["schedule_id", "enabled"]
            }),
        },
        Tool {
            name: "create_trigger".to_string(),
            description: "Create an event-driven trigger that assigns a task to an agent when a task completes or fails.".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "name": {
                        "type": "string",
                        "description": "Name for the trigger (e.g. 'auto-review')"
                    },
                    "event_type": {
                        "type": "string",
                        "enum": ["task_completed", "task_failed"],
                        "description": "The event that fires this trigger"
                    },
                    "agent_id": {
                        "type": "string",
                        "description": "The UUID of the agent to assign work to when triggered"
                    },
                    "task_title": {
                        "type": "string",
                        "description": "Title for the triggered task"
                    },
                    "task_description": {
                        "type": "string",
                        "description": "Description for the triggered task"
                    },
                    "role": {
                        "type": "string",
                        "description": "Optional role for the agent"
                    }
                },
                "required": ["name", "event_type", "agent_id", "task_title", "task_description"]
            }),
        },
        Tool {
            name: "list_triggers".to_string(),
            description: "List all event-driven triggers.".to_string(),
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
        "create_pipeline" => execute_create_pipeline(input, state, user_id).await,
        "add_pipeline_stage" => execute_add_pipeline_stage(input, state, user_id).await,
        "start_pipeline" => execute_start_pipeline(input, state).await,
        "get_pipeline_status" => execute_get_pipeline_status(input, state).await,
        "create_schedule" => execute_create_schedule(input, state, user_id).await,
        "list_schedules" => execute_list_schedules(state).await,
        "toggle_schedule" => execute_toggle_schedule(input, state, user_id).await,
        "create_trigger" => execute_create_trigger(input, state, user_id).await,
        "list_triggers" => execute_list_triggers(state).await,
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

async fn execute_create_pipeline(input: &Value, state: &AppState, user_id: UserId) -> Value {
    let Some(name) = input["name"].as_str() else {
        return json!({ "error": "name is required" });
    };

    let mut mgr = state.pipeline_manager.write().await;
    let id = mgr.create_pipeline(name.to_string());

    // Persist to DB
    if let Err(e) = state.repo.upsert_pipeline(user_id, PipelineRow {
        id: id.0,
        name: name.to_string(),
    }).await {
        tracing::error!("Failed to persist pipeline: {}", e);
    }

    json!({
        "status": "created",
        "pipeline_id": id.0.to_string(),
        "name": name
    })
}

async fn execute_add_pipeline_stage(input: &Value, state: &AppState, user_id: UserId) -> Value {
    let Some(pipeline_str) = input["pipeline_id"].as_str() else {
        return json!({ "error": "pipeline_id is required" });
    };
    let Some(agent_str) = input["agent_id"].as_str() else {
        return json!({ "error": "agent_id is required" });
    };

    let Ok(pipeline_uuid) = uuid::Uuid::parse_str(pipeline_str) else {
        return json!({ "error": format!("Invalid pipeline UUID: {}", pipeline_str) });
    };
    let Ok(agent_uuid) = uuid::Uuid::parse_str(agent_str) else {
        return json!({ "error": format!("Invalid agent UUID: {}", agent_str) });
    };

    let role = input["role"].as_str().map(String::from);
    let approval_required = input["approval_required"].as_bool().unwrap_or(false);

    let mut mgr = state.pipeline_manager.write().await;
    match mgr.add_stage(
        PipelineId(pipeline_uuid),
        crate::agents::AgentId(agent_uuid),
        role.clone(),
        approval_required,
    ) {
        Ok(stage_number) => {
            // Persist to DB
            let _ = user_id; // user_id used for pipeline ownership, stage inherits
            if let Err(e) = state.repo.upsert_pipeline_stage(PipelineStageRow {
                pipeline_id: pipeline_uuid,
                stage_number: stage_number as i32,
                agent_id: agent_uuid,
                role: role.clone(),
                approval_required,
            }).await {
                tracing::error!("Failed to persist pipeline stage: {}", e);
            }

            json!({
                "status": "added",
                "pipeline_id": pipeline_str,
                "stage_number": stage_number,
                "agent_id": agent_str,
                "role": role,
                "approval_required": approval_required
            })
        }
        Err(e) => json!({ "error": e.to_string() }),
    }
}

async fn execute_start_pipeline(input: &Value, state: &AppState) -> Value {
    let Some(pipeline_str) = input["pipeline_id"].as_str() else {
        return json!({ "error": "pipeline_id is required" });
    };
    let Some(task) = input["task"].as_str() else {
        return json!({ "error": "task is required" });
    };

    let Ok(pipeline_uuid) = uuid::Uuid::parse_str(pipeline_str) else {
        return json!({ "error": format!("Invalid pipeline UUID: {}", pipeline_str) });
    };

    let Some(dispatcher) = &state.dispatcher else {
        return json!({ "error": "Dispatcher not initialized" });
    };

    // Start the run and get first stage info
    let (run_id, first_agent_id, first_role) = {
        let mut mgr = state.pipeline_manager.write().await;
        match mgr.start_run(PipelineId(pipeline_uuid), task.to_string()) {
            Ok((run_id, first_stage)) => {
                let agent_id = first_stage.agent_id.clone();
                let role = first_stage.role.clone();
                (run_id, agent_id, role)
            }
            Err(e) => return json!({ "error": e.to_string() }),
        }
    };

    // Build task assignment for first stage (reusing assign_task logic)
    let role_str = first_role.as_deref().unwrap_or("worker");
    let role_id = RoleId::new(role_str);

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
                (
                    format!("You are a {} working on: {}", role_str, task),
                    CommunicationStyle::Technical,
                    OutputFormat::CodeAndReport,
                    vec![],
                )
            }
        } else {
            (
                format!("You are a {} working on: {}", role_str, task),
                CommunicationStyle::Technical,
                OutputFormat::CodeAndReport,
                vec![],
            )
        };

    let project_root = std::env::current_dir().unwrap_or_default();
    let execution_context = Some(crate::execution::ExecutionContext::new(project_root));

    let assignment = TaskAssignment {
        task_id: uuid::Uuid::new_v4(),
        title: format!("Pipeline stage 0: {}", task),
        description: task.to_string(),
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
            execution_context,
        },
        constraints: TaskConstraints::default(),
        timeout: Duration::from_secs(300),
        role_id,
    };

    let task_id = assignment.task_id;

    // Record task_id in pipeline run
    {
        let mut mgr = state.pipeline_manager.write().await;
        mgr.record_stage_task(run_id, 0, task_id);
    }

    let dispatcher = dispatcher.lock().await;
    match dispatcher
        .send_to_agent(&first_agent_id, AgentCommand::AssignTask(assignment))
        .await
    {
        Ok(()) => json!({
            "status": "started",
            "run_id": run_id.to_string(),
            "pipeline_id": pipeline_str,
            "current_stage": 0,
            "task_id": task_id.to_string()
        }),
        Err(e) => json!({ "error": e.to_string() }),
    }
}

async fn execute_get_pipeline_status(input: &Value, state: &AppState) -> Value {
    let Some(run_str) = input["run_id"].as_str() else {
        return json!({ "error": "run_id is required" });
    };
    let Ok(run_uuid) = uuid::Uuid::parse_str(run_str) else {
        return json!({ "error": format!("Invalid UUID: {}", run_str) });
    };

    let mgr = state.pipeline_manager.read().await;
    let Some(run) = mgr.get_run(run_uuid) else {
        return json!({ "error": "Pipeline run not found" });
    };

    // Collect per-stage results
    let results = state.task_results.read().await;
    let stage_results: Vec<Value> = run
        .stage_task_ids
        .iter()
        .map(|(stage_num, task_id)| {
            let status = match results.get(task_id) {
                None => "pending".to_string(),
                Some(AgentResponse::TaskStarted { .. }) => "started".to_string(),
                Some(AgentResponse::TaskCompleted { .. }) => "completed".to_string(),
                Some(AgentResponse::TaskFailed { .. }) => "failed".to_string(),
                Some(AgentResponse::ProgressUpdate { .. }) => "in_progress".to_string(),
                Some(_) => "unknown".to_string(),
            };
            json!({
                "stage": stage_num,
                "task_id": task_id.to_string(),
                "status": status
            })
        })
        .collect();

    let status_str = match &run.status {
        crate::agents::PipelineRunStatus::Running => "running",
        crate::agents::PipelineRunStatus::WaitingForApproval => "waiting_for_approval",
        crate::agents::PipelineRunStatus::Completed => "completed",
        crate::agents::PipelineRunStatus::Failed => "failed",
    };

    json!({
        "run_id": run_str,
        "pipeline_id": run.pipeline_id.0.to_string(),
        "status": status_str,
        "current_stage": run.current_stage,
        "stages": stage_results
    })
}

async fn execute_create_schedule(input: &Value, state: &AppState, user_id: UserId) -> Value {
    let Some(name) = input["name"].as_str() else {
        return json!({ "error": "name is required" });
    };
    let Some(agent_str) = input["agent_id"].as_str() else {
        return json!({ "error": "agent_id is required" });
    };
    let Some(interval) = input["interval_seconds"].as_u64() else {
        return json!({ "error": "interval_seconds is required" });
    };
    let Some(task_title) = input["task_title"].as_str() else {
        return json!({ "error": "task_title is required" });
    };
    let Some(task_description) = input["task_description"].as_str() else {
        return json!({ "error": "task_description is required" });
    };

    let Ok(agent_uuid) = uuid::Uuid::parse_str(agent_str) else {
        return json!({ "error": format!("Invalid agent UUID: {}", agent_str) });
    };

    let role = input["role"].as_str().map(String::from);

    let mut mgr = state.schedule_manager.write().await;
    let id = mgr.create_schedule(
        name.to_string(),
        crate::agents::AgentId(agent_uuid),
        interval,
        task_title.to_string(),
        task_description.to_string(),
        role.clone(),
    );

    // Persist to DB
    if let Err(e) = state.repo.upsert_schedule(user_id, ScheduleRow {
        id: id.0,
        name: name.to_string(),
        agent_id: agent_uuid,
        interval_seconds: interval as i32,
        task_title: task_title.to_string(),
        task_description: task_description.to_string(),
        role,
        enabled: true,
        last_run_at: None,
    }).await {
        tracing::error!("Failed to persist schedule: {}", e);
    }

    json!({
        "status": "created",
        "schedule_id": id.0.to_string(),
        "name": name,
        "interval_seconds": interval
    })
}

async fn execute_list_schedules(state: &AppState) -> Value {
    let mgr = state.schedule_manager.read().await;
    let schedules: Vec<Value> = mgr
        .list_schedules()
        .iter()
        .map(|s| {
            json!({
                "id": s.id.0.to_string(),
                "name": s.name,
                "agent_id": s.agent_id.0.to_string(),
                "interval_seconds": s.interval_seconds,
                "task_title": s.task_title,
                "enabled": s.enabled,
                "last_run_at": s.last_run_at.map(|t| t.to_rfc3339()),
            })
        })
        .collect();

    json!({
        "schedules": schedules,
        "count": schedules.len()
    })
}

async fn execute_toggle_schedule(input: &Value, state: &AppState, user_id: UserId) -> Value {
    let Some(schedule_str) = input["schedule_id"].as_str() else {
        return json!({ "error": "schedule_id is required" });
    };
    let Some(enabled) = input["enabled"].as_bool() else {
        return json!({ "error": "enabled is required" });
    };

    let Ok(schedule_uuid) = uuid::Uuid::parse_str(schedule_str) else {
        return json!({ "error": format!("Invalid schedule UUID: {}", schedule_str) });
    };

    let mut mgr = state.schedule_manager.write().await;
    let sid = ScheduleId(schedule_uuid);
    match mgr.set_enabled(sid, enabled) {
        Ok(()) => {
            // Persist updated state
            if let Some(schedule) = mgr.get_schedule(&sid) {
                if let Err(e) = state.repo.upsert_schedule(user_id, ScheduleRow {
                    id: schedule.id.0,
                    name: schedule.name.clone(),
                    agent_id: schedule.agent_id.0,
                    interval_seconds: schedule.interval_seconds as i32,
                    task_title: schedule.task_title.clone(),
                    task_description: schedule.task_description.clone(),
                    role: schedule.role.clone(),
                    enabled: schedule.enabled,
                    last_run_at: schedule.last_run_at,
                }).await {
                    tracing::error!("Failed to persist schedule toggle: {}", e);
                }
            }
            json!({
                "status": if enabled { "enabled" } else { "disabled" },
                "schedule_id": schedule_str
            })
        }
        Err(e) => json!({ "error": e.to_string() }),
    }
}

async fn execute_create_trigger(input: &Value, state: &AppState, user_id: UserId) -> Value {
    let Some(name) = input["name"].as_str() else {
        return json!({ "error": "name is required" });
    };
    let Some(event_str) = input["event_type"].as_str() else {
        return json!({ "error": "event_type is required" });
    };
    let Some(agent_str) = input["agent_id"].as_str() else {
        return json!({ "error": "agent_id is required" });
    };
    let Some(task_title) = input["task_title"].as_str() else {
        return json!({ "error": "task_title is required" });
    };
    let Some(task_description) = input["task_description"].as_str() else {
        return json!({ "error": "task_description is required" });
    };

    let Some(event_type) = TriggerEvent::from_str(event_str) else {
        return json!({ "error": format!("Invalid event_type: {}. Use 'task_completed' or 'task_failed'", event_str) });
    };
    let Ok(agent_uuid) = uuid::Uuid::parse_str(agent_str) else {
        return json!({ "error": format!("Invalid agent UUID: {}", agent_str) });
    };

    let role = input["role"].as_str().map(String::from);

    let mut mgr = state.schedule_manager.write().await;
    let id = mgr.create_trigger(
        name.to_string(),
        event_type,
        crate::agents::AgentId(agent_uuid),
        task_title.to_string(),
        task_description.to_string(),
        role.clone(),
    );

    // Persist to DB
    if let Err(e) = state.repo.upsert_trigger(user_id, TriggerRow {
        id: id.0,
        name: name.to_string(),
        event_type: event_str.to_string(),
        agent_id: agent_uuid,
        task_title: task_title.to_string(),
        task_description: task_description.to_string(),
        role,
    }).await {
        tracing::error!("Failed to persist trigger: {}", e);
    }

    json!({
        "status": "created",
        "trigger_id": id.0.to_string(),
        "name": name,
        "event_type": event_str
    })
}

async fn execute_list_triggers(state: &AppState) -> Value {
    let mgr = state.schedule_manager.read().await;
    let triggers: Vec<Value> = mgr
        .list_triggers()
        .iter()
        .map(|t| {
            json!({
                "id": t.id.0.to_string(),
                "name": t.name,
                "event_type": t.event_type.as_str(),
                "agent_id": t.agent_id.0.to_string(),
                "task_title": t.task_title,
                "role": t.role,
            })
        })
        .collect();

    json!({
        "triggers": triggers,
        "count": triggers.len()
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn agent_tools_returns_twentyone_tools() {
        let tools = agent_tools();
        assert_eq!(tools.len(), 21);
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
        assert_eq!(tools[12].name, "create_pipeline");
        assert_eq!(tools[13].name, "add_pipeline_stage");
        assert_eq!(tools[14].name, "start_pipeline");
        assert_eq!(tools[15].name, "get_pipeline_status");
        assert_eq!(tools[16].name, "create_schedule");
        assert_eq!(tools[17].name, "list_schedules");
        assert_eq!(tools[18].name, "toggle_schedule");
        assert_eq!(tools[19].name, "create_trigger");
        assert_eq!(tools[20].name, "list_triggers");
    }

    #[test]
    fn tool_schemas_are_valid_json() {
        for tool in agent_tools() {
            assert!(tool.input_schema.is_object());
            assert!(tool.input_schema["type"].as_str() == Some("object"));
        }
    }
}
