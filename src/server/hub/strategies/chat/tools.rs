//! Step tool resolution and archetype-specific dispatch.

use serde_json::Value;

use crate::llm::Tool;
use crate::server::state::AppState;

use super::config::StepChatContext;

/// Universal tools available to all archetypes.
const UNIVERSAL_TOOLS: &[&str] = &[
    "set_node_name",
    "set_node_description",
    "render_panel",
    "think",
    "update_notes",
];

/// Universal tool names handled by node_assistant.
const NODE_ASSISTANT_TOOLS: &[&str] = &["set_node_name", "set_node_description", "render_panel"];

/// Resolve tool definitions by step execution mode.
///
/// Always includes universal tools alongside archetype-specific ones.
pub(crate) fn resolve_step_tools(execution_mode: &str) -> Vec<Tool> {
    let archetype_specific: &[&str] = match execution_mode {
        "documenter" => &[
            "create_doc_def",
            "update_doc_def",
            "delete_doc_def",
            "update_config",
        ],
        "task_force" => &[
            "set_task",
            "add_agent",
            "update_agent",
            "remove_agent",
            "set_capabilities",
            "set_failure_mode",
        ],
        "belief_capture" => &[
            "set_extraction_focus",
            "set_tag_vocabulary",
            "set_contradiction_handling",
            "set_confidence_threshold",
        ],
        "room" => &[
            "set_meeting_purpose",
            "add_member",
            "update_member",
            "remove_member",
            "set_max_turns",
            "set_interaction_mode",
        ],
        _ => &[],
    };
    UNIVERSAL_TOOLS
        .iter()
        .chain(archetype_specific.iter())
        .filter_map(|name| crate::tools::registry::get_tool_definition(name))
        .collect()
}

/// Try to dispatch a tool call to a step-specific handler.
/// Returns `Some(result)` if handled, `None` to fall through to generic tools.
pub(super) async fn dispatch_step_tool(
    name: &str,
    input: &Value,
    state: &AppState,
    ctx: &StepChatContext,
) -> Option<Value> {
    // update_notes — needs state for board overview spawn
    if name == "update_notes" {
        let content = input["content"].as_str().unwrap_or("");
        match state
            .repos()
            .workflows
            .upsert_assistant_notes(ctx.step_id, content)
            .await
        {
            Ok(()) => {
                crate::server::hub::board_overview::spawn_board_overview_update(
                    state.clone(),
                    ctx.workflow_id,
                );
                return Some(serde_json::json!({ "status": "ok" }));
            }
            Err(e) => return Some(serde_json::json!({ "error": e.to_string() })),
        }
    }

    // Universal tools (all archetypes)
    if NODE_ASSISTANT_TOOLS.contains(&name) {
        let tool_ctx = crate::server::tools::node_assistant::StepToolContext {
            workflow_id: ctx.workflow_id,
            step_id: ctx.step_id,
        };
        let result = crate::server::tools::node_assistant::execute_node_assistant_tool(
            name,
            input,
            state.repos().workflows.as_ref(),
            &tool_ctx,
        )
        .await;
        return Some(result);
    }

    // Archetype-specific dispatch
    match ctx.execution_mode.as_str() {
        "documenter" => {
            const DOCUMENTER_TOOLS: &[&str] = &[
                "create_doc_def",
                "update_doc_def",
                "delete_doc_def",
                "update_config",
            ];
            if DOCUMENTER_TOOLS.contains(&name) {
                let tool_ctx = crate::server::tools::documenter::DocumenterToolContext {
                    workflow_id: ctx.workflow_id,
                    step_id: ctx.step_id,
                };
                let result = crate::server::tools::documenter::execute_documenter_tool(
                    name,
                    input,
                    state.repos().workflows.as_ref(),
                    &tool_ctx,
                )
                .await;
                return Some(result);
            }
            None
        }
        "task_force" => {
            const TASK_FORCE_TOOLS: &[&str] = &[
                "set_task",
                "add_agent",
                "update_agent",
                "remove_agent",
                "set_capabilities",
                "set_failure_mode",
            ];
            if TASK_FORCE_TOOLS.contains(&name) {
                let tool_ctx = crate::server::tools::task_force::TaskForceToolContext {
                    workflow_id: ctx.workflow_id,
                    step_id: ctx.step_id,
                };
                let result = crate::server::tools::task_force::execute_task_force_tool(
                    name,
                    input,
                    state.repos().workflows.as_ref(),
                    &tool_ctx,
                )
                .await;
                return Some(result);
            }
            None
        }
        "belief_capture" => {
            const BELIEF_CAPTURE_TOOLS: &[&str] = &[
                "set_extraction_focus",
                "set_tag_vocabulary",
                "set_contradiction_handling",
                "set_confidence_threshold",
            ];
            if BELIEF_CAPTURE_TOOLS.contains(&name) {
                let tool_ctx = crate::server::tools::belief_capture::BeliefCaptureToolContext {
                    workflow_id: ctx.workflow_id,
                    step_id: ctx.step_id,
                };
                let result = crate::server::tools::belief_capture::execute_belief_capture_tool(
                    name,
                    input,
                    state.repos().workflows.as_ref(),
                    &tool_ctx,
                )
                .await;
                return Some(result);
            }
            None
        }
        "room" => {
            const ROOM_TOOLS: &[&str] = &[
                "set_meeting_purpose",
                "add_member",
                "update_member",
                "remove_member",
                "set_max_turns",
                "set_interaction_mode",
            ];
            if ROOM_TOOLS.contains(&name) {
                let tool_ctx = crate::server::tools::room_config::RoomConfigToolContext {
                    workflow_id: ctx.workflow_id,
                    step_id: ctx.step_id,
                };
                let result = crate::server::tools::room_config::execute_room_config_tool(
                    name,
                    input,
                    state.repos().workflows.as_ref(),
                    &tool_ctx,
                )
                .await;
                return Some(result);
            }
            None
        }
        _ => None,
    }
}
