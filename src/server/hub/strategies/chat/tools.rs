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
    "dispatch",
    "cancel_dispatch",
];

/// Universal tool names handled by node_assistant.
const NODE_ASSISTANT_TOOLS: &[&str] = &["set_node_name", "set_node_description", "render_panel"];

/// Tools excluded from the workforce assistant's chat session.
/// The dispatch sub-agent owns note-taking for workforce nodes.
const WORKFORCE_CHAT_EXCLUDED: &[&str] = &["update_notes"];

/// Resolve tool definitions for step chat sessions.
///
/// For workforce mode, returns universal tools minus excluded tools — the
/// assistant dispatches to the background agent instead of calling mutation
/// tools directly. Notes are owned by the dispatch sub-agent.
/// For other modes, delegates to `resolve_step_tools()` which includes
/// both universal and archetype-specific tools.
pub(crate) fn resolve_chat_step_tools(execution_mode: &str) -> Vec<Tool> {
    match execution_mode {
        "workforce" => UNIVERSAL_TOOLS
            .iter()
            .filter(|name| !WORKFORCE_CHAT_EXCLUDED.contains(name))
            .filter_map(|name| crate::tools::registry::get_tool_definition(name))
            .collect(),
        _ => resolve_step_tools(execution_mode),
    }
}

/// Resolve tool definitions by step execution mode.
///
/// Always includes universal tools alongside archetype-specific ones.
/// Used by DispatchStrategy (background agent) which needs the full tool set.
pub(crate) fn resolve_step_tools(execution_mode: &str) -> Vec<Tool> {
    let archetype_specific: &[&str] = match execution_mode {
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
        "workforce" => &[
            "set_task",
            "add_agent",
            "update_agent",
            "remove_agent",
            "set_dependency",
            "remove_dependency",
            "set_capabilities",
            "set_failure_mode",
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

    // Dispatch tools (background service layer)
    if name == "dispatch" || name == "cancel_dispatch" {
        let result = super::dispatch::handle_dispatch_tool(name, input, state, ctx).await;
        return Some(result);
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
        "workforce" => {
            const WORKFORCE_TOOLS: &[&str] = &[
                "set_task",
                "add_agent",
                "update_agent",
                "remove_agent",
                "set_dependency",
                "remove_dependency",
                "set_capabilities",
                "set_failure_mode",
            ];
            if WORKFORCE_TOOLS.contains(&name) {
                let tool_ctx = crate::server::tools::workforce::WorkforceToolContext {
                    workflow_id: ctx.workflow_id,
                    step_id: ctx.step_id,
                };
                let result = crate::server::tools::workforce::execute_workforce_tool(
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
