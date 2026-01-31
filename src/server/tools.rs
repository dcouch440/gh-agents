//! Agent management tools for the orchestrator.
//!
//! Defines tool schemas (for the Anthropic tool use API) and execution
//! handlers that let the orchestrator LLM create, list, and assign agents.

use std::collections::HashMap;
use std::time::Duration;

use serde_json::{json, Value};

use std::sync::Arc;

use crate::agents::{
    AgentCommand, AgentResponse, ClusterId, CommunicationStyle, OutputFormat, PipelineId,
    RoleContext, RoleId, ScheduleId, TaskAssignment, TaskConstraints, TaskContext, TriggerEvent,
};
use crate::db::traits::DocumentRepo;
use crate::db::{AgentRow, ClusterRow, PipelineRow, PipelineStageRow, ScheduleRow, TriggerRow};
use crate::llm::{
    AnthropicClient, AnthropicConfig, LLMProvider, LLMRequest, Message as LlmMessage, Tool,
};
use crate::types::{AgentPersona, AgentTier, ModelConfig, UserId};

use super::state::AppState;

/// Return tool definitions filtered by allowed names.
/// If `allowed` is empty, returns all tools.
pub fn filtered_tools(allowed: &[String]) -> Vec<Tool> {
    let all = agent_tools();
    if allowed.is_empty() {
        return all;
    }
    all.into_iter()
        .filter(|t| allowed.iter().any(|a| a == &t.name))
        .collect()
}

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
            name: "create_agents".to_string(),
            description: "Create multiple agents at once. More efficient than calling create_agent repeatedly. Returns all agent IDs.".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "agents": {
                        "type": "array",
                        "items": {
                            "type": "object",
                            "properties": {
                                "tier": {
                                    "type": "string",
                                    "enum": ["orchestrator", "worker", "utility"],
                                    "description": "The agent tier"
                                },
                                "name": {
                                    "type": "string",
                                    "description": "Display name for the agent"
                                }
                            },
                            "required": ["tier"]
                        },
                        "description": "Array of agent definitions to create"
                    }
                },
                "required": ["agents"]
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
        // --- Codebase exploration tools (read-only) ---
        Tool {
            name: "read_file".to_string(),
            description: "Read a file in the project. Small files are returned directly. Large files are summarized by a fast model — use the 'focus' parameter to get relevant sections.".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "path": {
                        "type": "string",
                        "description": "File path relative to the project root (e.g. 'src/main.rs', 'ui/src/App.tsx')"
                    },
                    "focus": {
                        "type": "string",
                        "description": "Optional: what you're looking for in the file (e.g. 'error handling', 'the User struct', 'imports'). Helps extract relevant sections from large files."
                    }
                },
                "required": ["path"]
            }),
        },
        Tool {
            name: "list_files".to_string(),
            description: "List files and directories at a given path in the project. Use this to explore the codebase structure.".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "path": {
                        "type": "string",
                        "description": "Directory path relative to the project root. Use '.' or '' for the root."
                    }
                },
                "required": ["path"]
            }),
        },
        Tool {
            name: "search_files".to_string(),
            description: "Search for a pattern in project files. Returns matching lines with file paths and line numbers. Use this to find code references instead of reading entire files.".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "pattern": {
                        "type": "string",
                        "description": "Search pattern (text or regex) to find in files"
                    },
                    "path": {
                        "type": "string",
                        "description": "Optional: subdirectory to search in (e.g. 'src/', 'ui/src/'). Defaults to project root."
                    },
                    "max_results": {
                        "type": "integer",
                        "description": "Maximum number of matches to return. Defaults to 20."
                    }
                },
                "required": ["pattern"]
            }),
        },
        Tool {
            name: "think".to_string(),
            description: "Use this tool to think step-by-step before taking action. Write out your reasoning, \
                plan your approach, and consider edge cases. This tool has no side effects — it simply \
                returns your thoughts back to you. Use it before complex decisions, when choosing between \
                multiple approaches, or when you need to analyze information before responding.".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "thought": {
                        "type": "string",
                        "description": "Your step-by-step reasoning, analysis, or plan."
                    }
                },
                "required": ["thought"]
            }),
        },
        // --- Document tools ---
        Tool {
            name: "create_doc".to_string(),
            description: "Create a new document (architecture note, design doc, etc.). Returns the document ID and ref_tag. A summary is generated automatically in the background.".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "title": {
                        "type": "string",
                        "description": "Title of the document"
                    },
                    "content": {
                        "type": "string",
                        "description": "Full content/body of the document"
                    },
                    "tags": {
                        "type": "array",
                        "items": { "type": "string" },
                        "description": "Optional tags for categorization"
                    }
                },
                "required": ["title", "content"]
            }),
        },
        Tool {
            name: "update_doc".to_string(),
            description: "Update an existing document's content, title, or tags. A new summary is regenerated automatically in the background.".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "doc_id": {
                        "type": "string",
                        "description": "The UUID of the document to update"
                    },
                    "content": {
                        "type": "string",
                        "description": "New content for the document"
                    },
                    "title": {
                        "type": "string",
                        "description": "New title for the document"
                    },
                    "tags": {
                        "type": "array",
                        "items": { "type": "string" },
                        "description": "New tags for the document"
                    }
                },
                "required": ["doc_id"]
            }),
        },
        Tool {
            name: "search_docs".to_string(),
            description: "Search documents by full-text query. Returns summaries and snippets, not full content.".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "query": {
                        "type": "string",
                        "description": "Search query string"
                    }
                },
                "required": ["query"]
            }),
        },
        // --- Structured output validation tools ---
        Tool {
            name: "submit_prd".to_string(),
            description: "Submit a finalized PRD as structured JSON. Validates all fields and stores the PRD as a document. Returns validation errors if any fields are missing or invalid.".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "title": { "type": "string", "description": "PRD title" },
                    "problem_statement": { "type": "string", "description": "What problem this solves" },
                    "goals": { "type": "array", "items": { "type": "string" }, "description": "Measurable goals (min 1)" },
                    "non_goals": { "type": "array", "items": { "type": "string" }, "description": "Explicit scope boundaries (min 1)" },
                    "user_stories": { "type": "array", "items": { "type": "string" }, "description": "User stories (min 1)" },
                    "technical_approach": { "type": "string", "description": "Technical approach and architecture" },
                    "milestones": {
                        "type": "array",
                        "items": {
                            "type": "object",
                            "properties": {
                                "name": { "type": "string" },
                                "deliverables": { "type": "array", "items": { "type": "string" } }
                            },
                            "required": ["name", "deliverables"]
                        },
                        "description": "Implementation milestones (min 1)"
                    },
                    "complexity": { "type": "string", "enum": ["S", "M", "L", "XL"], "description": "Complexity estimate" },
                    "success_metrics": { "type": "array", "items": { "type": "string" }, "description": "Optional success metrics" },
                    "risks": { "type": "array", "items": { "type": "string" }, "description": "Optional risks" }
                },
                "required": ["title", "problem_statement", "goals", "non_goals", "user_stories", "technical_approach", "milestones", "complexity"]
            }),
        },
        Tool {
            name: "submit_ticket".to_string(),
            description: "Submit a decomposition ticket as structured JSON. Validates all fields and returns the validated ticket. Does not store the ticket — it flows through the pipeline system.".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "title": { "type": "string", "description": "Ticket title" },
                    "description": { "type": "string", "description": "Detailed description" },
                    "acceptance_criteria": { "type": "array", "items": { "type": "string" }, "description": "Acceptance criteria (min 1)" },
                    "files_to_modify": { "type": "array", "items": { "type": "string" }, "description": "Files to create or modify (min 1)" },
                    "complexity": { "type": "string", "enum": ["S", "M", "L", "XL"], "description": "Complexity estimate" },
                    "role": { "type": "string", "enum": ["worker", "reviewer", "utility"], "description": "Suggested agent role" },
                    "dependencies": { "type": "array", "items": { "type": "string" }, "description": "Optional ticket title dependencies" }
                },
                "required": ["title", "description", "acceptance_criteria", "files_to_modify", "complexity", "role"]
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
        "create_agents" => execute_create_agents(input, state, user_id).await,
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
        "read_file" => execute_read_file(input).await,
        "list_files" => execute_list_files(input).await,
        "search_files" => execute_search_files(input).await,
        "think" => execute_think(input),
        "create_doc" => execute_create_doc(input, state, user_id).await,
        "update_doc" => execute_update_doc(input, state).await,
        "search_docs" => execute_search_docs(input, state, user_id).await,
        "submit_prd" => execute_submit_prd(input, state, user_id).await,
        "submit_ticket" => execute_submit_ticket(input).await,
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

    let name = input["name"].as_str().unwrap_or(tier_str).to_string();

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
            if let Err(e) = state
                .repo
                .upsert_agent(
                    user_id,
                    AgentRow {
                        id: agent_id.0,
                        tier: tier_str.to_string(),
                        persona_name: name.clone(),
                        persona_prompt: String::new(),
                        persona_style: "casual".to_string(),
                        model_provider: format!("{:?}", model_config.provider).to_lowercase(),
                        model_id: model_config.model_id.clone(),
                        model_max_tokens: model_config.max_tokens as i32,
                        model_temperature: model_config.temperature,
                        status: "idle".to_string(),
                    },
                )
                .await
            {
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

async fn execute_create_agents(input: &Value, state: &AppState, user_id: UserId) -> Value {
    let Some(agents_arr) = input["agents"].as_array() else {
        return json!({ "error": "agents array is required" });
    };

    let Some(pool) = &state.pool else {
        return json!({ "error": "Agent pool not initialized" });
    };
    let Some(dispatcher) = &state.dispatcher else {
        return json!({ "error": "Dispatcher not initialized" });
    };

    let mut pool = pool.lock().await;
    let mut dispatcher = dispatcher.lock().await;
    let mut created = Vec::new();
    let mut errors = Vec::new();

    for agent_def in agents_arr {
        let tier_str = agent_def["tier"].as_str().unwrap_or("worker");
        let tier = match tier_str {
            "orchestrator" => AgentTier::Orchestrator,
            "worker" => AgentTier::Worker,
            "utility" => AgentTier::Utility,
            other => {
                errors.push(json!({ "error": format!("Invalid tier: {}", other) }));
                continue;
            }
        };

        let name = agent_def["name"].as_str().unwrap_or(tier_str).to_string();

        let persona = AgentPersona {
            name: name.clone(),
            ..Default::default()
        };

        let model_config = ModelConfig::default();

        match pool.spawn_agent_with_dispatcher(tier, persona, model_config.clone(), &mut dispatcher)
        {
            Ok(agent_id) => {
                if let Err(e) = state
                    .repo
                    .upsert_agent(
                        user_id,
                        AgentRow {
                            id: agent_id.0,
                            tier: tier_str.to_string(),
                            persona_name: name.clone(),
                            persona_prompt: String::new(),
                            persona_style: "casual".to_string(),
                            model_provider: format!("{:?}", model_config.provider).to_lowercase(),
                            model_id: model_config.model_id.clone(),
                            model_max_tokens: model_config.max_tokens as i32,
                            model_temperature: model_config.temperature,
                            status: "idle".to_string(),
                        },
                    )
                    .await
                {
                    tracing::error!("Failed to persist agent: {}", e);
                }
                created.push(json!({
                    "agent_id": agent_id.0.to_string(),
                    "tier": tier_str,
                    "name": name,
                    "status": "created"
                }));
            }
            Err(e) => {
                errors.push(json!({ "name": name, "error": e.to_string() }));
            }
        }
    }

    json!({
        "created": created,
        "errors": errors,
        "total_created": created.len()
    })
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

    // Resolve @doc:ref-tag references in the description
    let description = {
        let mut desc = description.to_string();
        if let Some(doc_repo) = &state.doc_repo {
            let re = regex::Regex::new(r"@doc:([\w-]+)").unwrap();
            let mut doc_sections = Vec::new();
            for cap in re.captures_iter(&desc) {
                let ref_tag = &cap[1];
                match doc_repo.get_document_by_ref_tag(ref_tag).await {
                    Ok(Some(row)) if !row.summary.is_empty() => {
                        doc_sections.push(format!("### @doc:{}\n{}", ref_tag, row.summary));
                    }
                    _ => {
                        tracing::debug!(
                            "Document ref @doc:{} not found or has no summary",
                            ref_tag
                        );
                    }
                }
            }
            if !doc_sections.is_empty() {
                desc.push_str("\n\n---\n## Referenced Documents\n\n");
                desc.push_str(&doc_sections.join("\n\n"));
                desc.push('\n');
            }
        }
        desc
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
    let execution_context = Some(crate::execution::ExecutionContext::new(
        project_root.clone(),
    ));

    // Compile live context from repo index (if ready)
    let index = state.repo_index.read().await;
    let (context_briefing, context_files) = if index.ready {
        let compiled =
            crate::indexing::compiler::compile_context(&index, title, &description, &project_root)
                .await;
        (compiled.briefing, compiled.relevant_files)
    } else {
        (String::new(), vec![])
    };
    drop(index);

    // Merge context briefing into conventions and context files into required_reading
    let conventions = if context_briefing.is_empty() {
        cluster_conventions
    } else {
        format!("{}\n\n{}", context_briefing, cluster_conventions)
    };
    let mut required_reading = required_reading;
    for (path, content) in context_files {
        required_reading.push(crate::agents::FileContent { path, content });
    }

    // Parse allowed_tools if provided
    let allowed_tools = input["allowed_tools"].as_array().map(|arr| {
        arr.iter()
            .filter_map(|v| v.as_str().map(String::from))
            .collect()
    });

    let mut constraints = TaskConstraints::default();
    constraints.allowed_tools = allowed_tools;

    let assignment = TaskAssignment {
        task_id: uuid::Uuid::new_v4(),
        title: title.to_string(),
        description,
        context: TaskContext {
            required_reading,
            files: cluster_files,
            history: vec![],
            conventions,
            role_context: RoleContext {
                system_prompt,
                style,
                output_format,
            },
            chat_messages: vec![],
            execution_context,
        },
        constraints,
        timeout: Duration::from_secs(crate::constants::DEFAULT_TIMEOUT_SECS),
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
    if let Err(e) = state
        .repo
        .upsert_cluster(
            user_id,
            ClusterRow {
                id: id.0,
                name: name.to_string(),
                description,
                conventions: String::new(),
                shared_files: serde_json::json!([]),
            },
        )
        .await
    {
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
            if let Err(e) = state
                .repo
                .add_cluster_member(cluster_uuid, agent_uuid)
                .await
            {
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
            if let Err(e) = state
                .repo
                .remove_cluster_member(cluster_uuid, agent_uuid)
                .await
            {
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
    if let Err(e) = state
        .repo
        .upsert_pipeline(
            user_id,
            PipelineRow {
                id: id.0,
                name: name.to_string(),
            },
        )
        .await
    {
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
            if let Err(e) = state
                .repo
                .upsert_pipeline_stage(PipelineStageRow {
                    pipeline_id: pipeline_uuid,
                    stage_number: stage_number as i32,
                    agent_id: agent_uuid,
                    role: role.clone(),
                    approval_required,
                })
                .await
            {
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
        timeout: Duration::from_secs(crate::constants::DEFAULT_TIMEOUT_SECS),
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
    if let Err(e) = state
        .repo
        .upsert_schedule(
            user_id,
            ScheduleRow {
                id: id.0,
                name: name.to_string(),
                agent_id: agent_uuid,
                interval_seconds: interval as i32,
                task_title: task_title.to_string(),
                task_description: task_description.to_string(),
                role,
                enabled: true,
                last_run_at: None,
            },
        )
        .await
    {
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
                if let Err(e) = state
                    .repo
                    .upsert_schedule(
                        user_id,
                        ScheduleRow {
                            id: schedule.id.0,
                            name: schedule.name.clone(),
                            agent_id: schedule.agent_id.0,
                            interval_seconds: schedule.interval_seconds as i32,
                            task_title: schedule.task_title.clone(),
                            task_description: schedule.task_description.clone(),
                            role: schedule.role.clone(),
                            enabled: schedule.enabled,
                            last_run_at: schedule.last_run_at,
                        },
                    )
                    .await
                {
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
    if let Err(e) = state
        .repo
        .upsert_trigger(
            user_id,
            TriggerRow {
                id: id.0,
                name: name.to_string(),
                event_type: event_str.to_string(),
                agent_id: agent_uuid,
                task_title: task_title.to_string(),
                task_description: task_description.to_string(),
                role,
            },
        )
        .await
    {
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

// --- Codebase exploration tool handlers ---

async fn execute_read_file(input: &Value) -> Value {
    let Some(path_str) = input["path"].as_str() else {
        return json!({ "error": "Missing required parameter: path" });
    };
    let focus = input["focus"].as_str();

    // Resolve relative to current working directory (project root)
    let cwd = std::env::current_dir().unwrap_or_default();
    let file_path = cwd.join(path_str);

    // Basic safety: don't escape the project root
    match file_path.canonicalize() {
        Ok(canonical) => {
            if !canonical.starts_with(&cwd) {
                return json!({ "error": "Path is outside the project directory" });
            }
            match tokio::fs::read_to_string(&canonical).await {
                Ok(content) => {
                    let size_bytes = content.len();
                    let line_count = content.lines().count();

                    // Small files: return directly
                    if content.len() <= crate::constants::TRUNCATE_SMALL_FILE {
                        return json!({
                            "path": path_str,
                            "content": content,
                            "line_count": line_count,
                            "size_bytes": size_bytes,
                            "summarized": false
                        });
                    }

                    // Large files: summarize with Haiku
                    let truncated_for_haiku: String = content
                        .chars()
                        .take(crate::constants::TRUNCATE_SUMMARIZE_INPUT)
                        .collect();
                    let focus_instruction = match focus {
                        Some(f) => format!(
                            "Focus on: {}. Extract the most relevant code sections, function signatures, and logic related to this focus area.",
                            f
                        ),
                        None => "Extract the key structures, function signatures, imports, and overall purpose of this file.".to_string(),
                    };

                    let prompt = format!(
                        "File: {} ({} lines, {} bytes)\n\n{}\n\n---\n{}",
                        path_str, line_count, size_bytes, focus_instruction, truncated_for_haiku
                    );

                    match haiku_read_file(&prompt).await {
                        Some(summary) => json!({
                            "path": path_str,
                            "summary": summary,
                            "line_count": line_count,
                            "size_bytes": size_bytes,
                            "summarized": true
                        }),
                        None => {
                            // Haiku failed — fall back to truncated content
                            let fallback: String = content
                                .chars()
                                .take(crate::constants::TRUNCATE_SMALL_FILE)
                                .collect();
                            json!({
                                "path": path_str,
                                "content": fallback,
                                "line_count": line_count,
                                "size_bytes": size_bytes,
                                "summarized": false,
                                "truncated": true
                            })
                        }
                    }
                }
                Err(e) => json!({ "error": format!("Could not read file: {}", e) }),
            }
        }
        Err(e) => json!({ "error": format!("File not found or inaccessible: {}", e) }),
    }
}

async fn execute_list_files(input: &Value) -> Value {
    let path_str = input["path"].as_str().unwrap_or(".");

    let cwd = std::env::current_dir().unwrap_or_default();
    let dir_path = if path_str.is_empty() || path_str == "." {
        cwd.clone()
    } else {
        cwd.join(path_str)
    };

    match dir_path.canonicalize() {
        Ok(canonical) => {
            if !canonical.starts_with(&cwd) {
                return json!({ "error": "Path is outside the project directory" });
            }
            match tokio::fs::read_dir(&canonical).await {
                Ok(mut entries) => {
                    let mut files = Vec::new();
                    let mut dirs = Vec::new();
                    while let Ok(Some(entry)) = entries.next_entry().await {
                        let name = entry.file_name().to_string_lossy().to_string();
                        // Skip hidden files/dirs
                        if name.starts_with('.') {
                            continue;
                        }
                        if let Ok(ft) = entry.file_type().await {
                            if ft.is_dir() {
                                dirs.push(format!("{}/", name));
                            } else {
                                files.push(name);
                            }
                        }
                    }
                    dirs.sort();
                    files.sort();
                    json!({
                        "path": path_str,
                        "directories": dirs,
                        "files": files
                    })
                }
                Err(e) => json!({ "error": format!("Could not list directory: {}", e) }),
            }
        }
        Err(e) => json!({ "error": format!("Directory not found: {}", e) }),
    }
}

async fn execute_search_files(input: &Value) -> Value {
    let Some(pattern) = input["pattern"].as_str() else {
        return json!({ "error": "Missing required parameter: pattern" });
    };
    let path_str = input["path"].as_str().unwrap_or(".");
    let max_results = input["max_results"]
        .as_u64()
        .unwrap_or(crate::constants::DEFAULT_SEARCH_RESULTS as u64) as usize;

    let cwd = std::env::current_dir().unwrap_or_default();
    let search_dir = if path_str.is_empty() || path_str == "." {
        cwd.clone()
    } else {
        cwd.join(path_str)
    };

    // Validate path
    match search_dir.canonicalize() {
        Ok(canonical) => {
            if !canonical.starts_with(&cwd) {
                return json!({ "error": "Path is outside the project directory" });
            }

            // Use grep -rn for search
            let output = tokio::process::Command::new("grep")
                .args([
                    "-rn",
                    "--include=*.rs",
                    "--include=*.ts",
                    "--include=*.tsx",
                    "--include=*.js",
                    "--include=*.json",
                    "--include=*.toml",
                    "--include=*.sql",
                    "--include=*.md",
                    "--include=*.txt",
                    "--include=*.css",
                    "--include=*.html",
                    "-m",
                    &(max_results * 2).to_string(), // overfetch for filtering
                    pattern,
                ])
                .arg(&canonical)
                .output()
                .await;

            match output {
                Ok(out) => {
                    let stdout = String::from_utf8_lossy(&out.stdout);
                    let matches: Vec<Value> = stdout
                        .lines()
                        .take(max_results)
                        .filter_map(|line| {
                            // Format: /abs/path:line_num:content
                            let rest = line.strip_prefix(canonical.to_str()?)?;
                            let rest = rest.strip_prefix('/')?;
                            let mut parts = rest.splitn(3, ':');
                            let file = parts.next()?;
                            let line_num = parts.next()?;
                            let text = parts.next().unwrap_or("").trim();
                            Some(json!({
                                "file": file,
                                "line": line_num.parse::<u64>().unwrap_or(0),
                                "text": &text[..text.len().min(200)]
                            }))
                        })
                        .collect();

                    let total = stdout.lines().count();
                    json!({
                        "pattern": pattern,
                        "matches": matches,
                        "total_matches": total,
                        "truncated": total > max_results
                    })
                }
                Err(e) => json!({ "error": format!("Search failed: {}", e) }),
            }
        }
        Err(e) => json!({ "error": format!("Directory not found: {}", e) }),
    }
}

/// The think tool is a no-op — it returns the agent's reasoning back to it.
/// This gives the model a scratchpad to reason step-by-step before acting.
fn execute_think(input: &Value) -> Value {
    let thought = input["thought"].as_str().unwrap_or("");
    json!({ "thought_recorded": true, "length": thought.len() })
}

// --- Document tool handlers ---

/// Generate a kebab-case ref_tag from a title.
fn title_to_ref_tag(title: &str) -> String {
    title
        .to_lowercase()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join("-")
        .chars()
        .filter(|c| c.is_alphanumeric() || *c == '-')
        .collect()
}

/// Call Haiku to summarize a file for the orchestrator context.
pub async fn haiku_read_file(prompt: &str) -> Option<String> {
    let config = AnthropicConfig::from_env().ok()?;
    let client = AnthropicClient::new(config).ok()?;

    let request = LLMRequest::new(
        crate::constants::MODEL_HAIKU,
        vec![LlmMessage::user(prompt.to_string())],
    )
    .with_system(
        "You are a code reader. Given a source file, extract and return the most relevant content. \
         Include function signatures, struct/type definitions, key logic, and imports. \
         Use the original code when possible — quote exact lines for precision. \
         If a focus area is specified, prioritize content related to it. \
         Be concise but preserve technical accuracy. Do not add commentary."
    )
    .with_max_tokens(crate::constants::MAX_TOKENS_FILE_READ);

    match client.send_message(request).await {
        Ok(resp) => Some(resp.content),
        Err(e) => {
            tracing::warn!("Haiku file read failed: {}", e);
            None
        }
    }
}

/// Call Haiku to generate a short summary for search indexing.
pub async fn haiku_summarize(content: &str) -> Option<String> {
    let config = AnthropicConfig::from_env().ok()?;
    let client = AnthropicClient::new(config).ok()?;

    let truncated: String = content
        .chars()
        .take(crate::constants::TRUNCATE_SUMMARY_INPUT)
        .collect();
    let request = LLMRequest::new(
        crate::constants::MODEL_HAIKU,
        vec![LlmMessage::user(truncated)],
    )
    .with_system("Summarize this document in 2-3 sentences for search indexing. Be concise.")
    .with_max_tokens(crate::constants::MAX_TOKENS_SUMMARIZE);

    match client.send_message(request).await {
        Ok(resp) => Some(resp.content),
        Err(e) => {
            tracing::warn!("Haiku summarization failed: {}", e);
            None
        }
    }
}

/// Call Haiku to generate a short title for a conversation.
pub async fn haiku_summarize_title(content: &str) -> Option<String> {
    let config = AnthropicConfig::from_env().ok()?;
    let client = AnthropicClient::new(config).ok()?;

    let truncated: String = content
        .chars()
        .take(crate::constants::TRUNCATE_TITLE_INPUT)
        .collect();
    let request = LLMRequest::new(
        crate::constants::MODEL_HAIKU,
        vec![LlmMessage::user(truncated)],
    )
    .with_system("Generate a short title (3-6 words) for this conversation. Return ONLY the title, no quotes, no punctuation at the end.")
    .with_max_tokens(crate::constants::MAX_TOKENS_TITLE);

    match client.send_message(request).await {
        Ok(resp) => {
            let title = resp.content.trim().to_string();
            if title.is_empty() {
                None
            } else {
                Some(title)
            }
        }
        Err(e) => {
            tracing::warn!("Haiku title generation failed: {}", e);
            None
        }
    }
}

/// Call Haiku to extract relevant context from a conversation summary
/// based on the user's current message.
pub async fn haiku_extract_context(summary: &str, current_message: &str) -> Option<String> {
    let config = AnthropicConfig::from_env().ok()?;
    let client = AnthropicClient::new(config).ok()?;

    let user_text = format!(
        "Summary:\n{}\n\nCurrent message:\n{}",
        summary, current_message
    );
    let request = LLMRequest::new(
        crate::constants::MODEL_HAIKU,
        vec![LlmMessage::user(user_text)],
    )
    .with_system("You extract relevant context from a conversation summary based on the user's current message. Return 2-4 sentences of context that are directly relevant to what the user is asking about now. If nothing is relevant, return 'No prior context needed.'")
    .with_max_tokens(crate::constants::MAX_TOKENS_CONTEXT);

    match client.send_message(request).await {
        Ok(resp) => Some(resp.content),
        Err(e) => {
            tracing::warn!("Haiku context extraction failed: {}", e);
            None
        }
    }
}

/// Spawn a background task to generate and store a document summary.
fn spawn_summary_task(doc_repo: Arc<dyn DocumentRepo>, doc_id: uuid::Uuid, content: String) {
    tokio::spawn(async move {
        if let Some(summary) = haiku_summarize(&content).await {
            if let Err(e) = doc_repo.update_document_summary(doc_id, summary).await {
                tracing::error!("Failed to update document summary: {}", e);
            }
        }
    });
}

async fn execute_create_doc(input: &Value, state: &AppState, user_id: UserId) -> Value {
    let Some(doc_repo) = &state.doc_repo else {
        return json!({ "error": "Document repository not initialized" });
    };

    let Some(title) = input["title"].as_str() else {
        return json!({ "error": "title is required" });
    };
    let Some(content) = input["content"].as_str() else {
        return json!({ "error": "content is required" });
    };

    let tags: Vec<String> = input["tags"]
        .as_array()
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str().map(String::from))
                .collect()
        })
        .unwrap_or_default();

    let ref_tag = title_to_ref_tag(title);

    match doc_repo
        .create_document(
            user_id.0,
            None, // session_id
            title.to_string(),
            content.to_string(),
            "architecture".to_string(),
            ref_tag.clone(),
            tags,
        )
        .await
    {
        Ok(row) => {
            // Spawn background summary generation
            spawn_summary_task(Arc::clone(doc_repo), row.id, content.to_string());

            json!({
                "doc_id": row.id.to_string(),
                "ref_tag": ref_tag,
                "title": title
            })
        }
        Err(e) => json!({ "error": e.to_string() }),
    }
}

async fn execute_update_doc(input: &Value, state: &AppState) -> Value {
    let Some(doc_repo) = &state.doc_repo else {
        return json!({ "error": "Document repository not initialized" });
    };

    let Some(id_str) = input["doc_id"].as_str() else {
        return json!({ "error": "doc_id is required" });
    };
    let Ok(doc_id) = uuid::Uuid::parse_str(id_str) else {
        return json!({ "error": format!("Invalid UUID: {}", id_str) });
    };

    let content = input["content"].as_str().map(String::from);
    let title = input["title"].as_str().map(String::from);
    let tags: Option<Vec<String>> = input["tags"].as_array().map(|arr| {
        arr.iter()
            .filter_map(|v| v.as_str().map(String::from))
            .collect()
    });

    match doc_repo
        .update_document(doc_id, content.clone(), title.clone(), tags)
        .await
    {
        Ok(row) => {
            // Spawn background summary regeneration using updated content
            let summary_content = content.unwrap_or(row.content.clone());
            spawn_summary_task(Arc::clone(doc_repo), doc_id, summary_content);

            json!({
                "updated": true,
                "doc_id": doc_id.to_string(),
                "title": row.title
            })
        }
        Err(e) => json!({ "error": e.to_string() }),
    }
}

async fn execute_search_docs(input: &Value, state: &AppState, user_id: UserId) -> Value {
    let Some(doc_repo) = &state.doc_repo else {
        return json!({ "error": "Document repository not initialized" });
    };

    let Some(query) = input["query"].as_str() else {
        return json!({ "error": "query is required" });
    };

    match doc_repo.search_documents(user_id.0, query).await {
        Ok(results) => {
            let items: Vec<Value> = results
                .into_iter()
                .map(|r| {
                    json!({
                        "id": r.id.to_string(),
                        "title": r.title,
                        "ref_tag": r.ref_tag,
                        "summary": r.summary,
                        "snippet": r.snippet
                    })
                })
                .collect();
            json!({ "results": items, "count": items.len() })
        }
        Err(e) => json!({ "error": e.to_string() }),
    }
}

// --- Structured output validation tool handlers ---

async fn execute_submit_prd(input: &Value, state: &AppState, user_id: UserId) -> Value {
    let mut errors = Vec::new();

    // Validate required string fields
    let title = input["title"].as_str().unwrap_or("");
    if title.is_empty() {
        errors.push("Missing field: title".to_string());
    }
    let problem_statement = input["problem_statement"].as_str().unwrap_or("");
    if problem_statement.is_empty() {
        errors.push("Missing field: problem_statement".to_string());
    }
    let technical_approach = input["technical_approach"].as_str().unwrap_or("");
    if technical_approach.is_empty() {
        errors.push("Missing field: technical_approach".to_string());
    }

    // Validate required array fields
    let goals = input["goals"].as_array();
    if goals.map_or(true, |a| a.is_empty()) {
        errors.push("goals must have at least 1 entry".to_string());
    }
    let non_goals = input["non_goals"].as_array();
    if non_goals.map_or(true, |a| a.is_empty()) {
        errors.push("non_goals must have at least 1 entry".to_string());
    }
    let user_stories = input["user_stories"].as_array();
    if user_stories.map_or(true, |a| a.is_empty()) {
        errors.push("user_stories must have at least 1 entry".to_string());
    }

    // Validate milestones
    let milestones = input["milestones"].as_array();
    if milestones.map_or(true, |a| a.is_empty()) {
        errors.push("milestones must have at least 1 entry".to_string());
    } else if let Some(ms) = milestones {
        for (i, m) in ms.iter().enumerate() {
            if m["name"].as_str().unwrap_or("").is_empty() {
                errors.push(format!("milestones[{}] missing name", i));
            }
            if m["deliverables"].as_array().map_or(true, |a| a.is_empty()) {
                errors.push(format!(
                    "milestones[{}] must have at least 1 deliverable",
                    i
                ));
            }
        }
    }

    // Validate complexity
    let complexity = input["complexity"].as_str().unwrap_or("");
    if !matches!(complexity, "S" | "M" | "L" | "XL") {
        errors.push("complexity must be one of: S, M, L, XL".to_string());
    }

    if !errors.is_empty() {
        return json!({ "valid": false, "errors": errors });
    }

    // Format PRD as markdown
    let goals_arr = goals.unwrap();
    let non_goals_arr = non_goals.unwrap();
    let user_stories_arr = user_stories.unwrap();
    let milestones_arr = milestones.unwrap();

    let mut md = format!("# PRD: {}\n\n## Status: APPROVED\n\n", title);
    md.push_str(&format!(
        "## Problem Statement\n\n{}\n\n",
        problem_statement
    ));

    md.push_str("## Goals\n\n");
    for g in goals_arr {
        md.push_str(&format!("- {}\n", g.as_str().unwrap_or("")));
    }

    md.push_str("\n## Non-Goals\n\n");
    for ng in non_goals_arr {
        md.push_str(&format!("- {}\n", ng.as_str().unwrap_or("")));
    }

    md.push_str("\n## User Stories\n\n");
    for us in user_stories_arr {
        md.push_str(&format!("- {}\n", us.as_str().unwrap_or("")));
    }

    md.push_str(&format!(
        "\n## Technical Approach\n\n{}\n\n",
        technical_approach
    ));

    md.push_str("## Milestones\n\n");
    for m in milestones_arr {
        md.push_str(&format!("### {}\n\n", m["name"].as_str().unwrap_or("")));
        if let Some(deliverables) = m["deliverables"].as_array() {
            for d in deliverables {
                md.push_str(&format!("- {}\n", d.as_str().unwrap_or("")));
            }
        }
        md.push('\n');
    }

    md.push_str(&format!("## Complexity: {}\n\n", complexity));

    if let Some(metrics) = input["success_metrics"].as_array() {
        if !metrics.is_empty() {
            md.push_str("## Success Metrics\n\n");
            for m in metrics {
                md.push_str(&format!("- {}\n", m.as_str().unwrap_or("")));
            }
            md.push('\n');
        }
    }

    if let Some(risks) = input["risks"].as_array() {
        if !risks.is_empty() {
            md.push_str("## Risks\n\n");
            for r in risks {
                md.push_str(&format!("- {}\n", r.as_str().unwrap_or("")));
            }
            md.push('\n');
        }
    }

    // Store as document
    let Some(doc_repo) = &state.doc_repo else {
        return json!({ "error": "Document repository not initialized" });
    };

    let ref_tag = title_to_ref_tag(title);

    match doc_repo
        .create_document(
            user_id.0,
            None,
            title.to_string(),
            md.clone(),
            "prd".to_string(),
            ref_tag.clone(),
            vec!["prd".to_string()],
        )
        .await
    {
        Ok(row) => {
            spawn_summary_task(Arc::clone(doc_repo), row.id, md);
            json!({
                "valid": true,
                "doc_id": row.id.to_string(),
                "ref_tag": ref_tag
            })
        }
        Err(e) => json!({ "error": e.to_string() }),
    }
}

async fn execute_submit_ticket(input: &Value) -> Value {
    let mut errors = Vec::new();

    let title = input["title"].as_str().unwrap_or("");
    if title.is_empty() {
        errors.push("Missing field: title".to_string());
    }
    let description = input["description"].as_str().unwrap_or("");
    if description.is_empty() {
        errors.push("Missing field: description".to_string());
    }

    let acceptance_criteria = input["acceptance_criteria"].as_array();
    if acceptance_criteria.map_or(true, |a| a.is_empty()) {
        errors.push("acceptance_criteria must have at least 1 entry".to_string());
    }
    let files_to_modify = input["files_to_modify"].as_array();
    if files_to_modify.map_or(true, |a| a.is_empty()) {
        errors.push("files_to_modify must have at least 1 entry".to_string());
    }

    let complexity = input["complexity"].as_str().unwrap_or("");
    if !matches!(complexity, "S" | "M" | "L" | "XL") {
        errors.push("complexity must be one of: S, M, L, XL".to_string());
    }

    let role = input["role"].as_str().unwrap_or("");
    if !matches!(role, "worker" | "reviewer" | "utility") {
        errors.push("role must be one of: worker, reviewer, utility".to_string());
    }

    if !errors.is_empty() {
        return json!({ "valid": false, "errors": errors });
    }

    let dependencies: Vec<String> = input["dependencies"]
        .as_array()
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str().map(String::from))
                .collect()
        })
        .unwrap_or_default();

    json!({
        "valid": true,
        "ticket": {
            "title": title,
            "description": description,
            "acceptance_criteria": acceptance_criteria.unwrap(),
            "files_to_modify": files_to_modify.unwrap(),
            "complexity": complexity,
            "role": role,
            "dependencies": dependencies
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn agent_tools_returns_all_tools() {
        let tools = agent_tools();
        assert_eq!(tools.len(), 31);
        assert_eq!(tools[0].name, "list_agents");
        assert_eq!(tools[1].name, "list_roles");
        assert_eq!(tools[2].name, "create_agent");
        assert_eq!(tools[3].name, "create_agents");
        assert_eq!(tools[4].name, "assign_task");
        assert_eq!(tools[5].name, "get_task_result");
        assert_eq!(tools[6].name, "list_pending_approvals");
        assert_eq!(tools[7].name, "respond_to_approval");
        assert_eq!(tools[8].name, "remove_agent");
        assert_eq!(tools[9].name, "create_cluster");
        assert_eq!(tools[10].name, "add_to_cluster");
        assert_eq!(tools[11].name, "remove_from_cluster");
        assert_eq!(tools[12].name, "list_clusters");
        assert_eq!(tools[13].name, "create_pipeline");
        assert_eq!(tools[14].name, "add_pipeline_stage");
        assert_eq!(tools[15].name, "start_pipeline");
        assert_eq!(tools[16].name, "get_pipeline_status");
        assert_eq!(tools[17].name, "create_schedule");
        assert_eq!(tools[18].name, "list_schedules");
        assert_eq!(tools[19].name, "toggle_schedule");
        assert_eq!(tools[20].name, "create_trigger");
        assert_eq!(tools[21].name, "list_triggers");
        assert_eq!(tools[22].name, "read_file");
        assert_eq!(tools[23].name, "list_files");
        assert_eq!(tools[24].name, "search_files");
        assert_eq!(tools[26].name, "create_doc");
        assert_eq!(tools[27].name, "update_doc");
        assert_eq!(tools[28].name, "search_docs");
        assert_eq!(tools[29].name, "submit_prd");
        assert_eq!(tools[30].name, "submit_ticket");
    }

    #[test]
    fn tool_schemas_are_valid_json() {
        for tool in agent_tools() {
            assert!(tool.input_schema.is_object());
            assert!(tool.input_schema["type"].as_str() == Some("object"));
        }
    }
}
