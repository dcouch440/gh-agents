//! Step tool resolution and archetype-specific dispatch.

use serde_json::Value;

use crate::llm::Tool;
use crate::server::state::AppState;

use super::config::StepChatContext;

/// Universal tools available to all archetypes in the conversational assistant.
///
/// `set_node_description` is intentionally excluded — the assistant must
/// dispatch that to a background agent rather than calling it directly.
/// It remains available to the dispatch strategy via `resolve_step_tools()`.
const UNIVERSAL_TOOLS: &[&str] = &[
    "render_panel",
    "think",
    "update_notes",
    "dispatch",
    "cancel_dispatch",
    "set_node_name",
];

/// Node mutation tools — dispatch-only, not available to the conversational assistant.
const NODE_MUTATION_TOOLS: &[&str] = &["set_node_description"];

/// Universal tool names handled by node_assistant (includes mutations for dispatch routing).
const NODE_ASSISTANT_TOOLS: &[&str] = &["set_node_name", "set_node_description", "render_panel"];

/// Tools excluded from the workforce assistant's chat session.
/// The dispatch sub-agent owns note-taking for workforce nodes.
const WORKFORCE_CHAT_EXCLUDED: &[&str] = &["update_notes"];

/// Manager assistant (L1) tools — dispatch, cancel, think, render_panel.
const MANAGER_TOOLS: &[&str] = &["dispatch", "cancel_dispatch", "think", "render_panel"];

/// Resolve tool definitions for step chat sessions (the conversational assistant).
///
/// Returns universal tools plus archetype-specific tools, but excludes
/// `set_node_description` — the assistant must dispatch that to a background
/// agent.
///
/// For workforce mode, additionally excludes `update_notes` (dispatch sub-agent
/// owns note-taking for workforce nodes).
///
/// For manager mode, returns only manager-specific tools (no node mutation).
pub(crate) fn resolve_chat_step_tools(execution_mode: &str) -> Vec<Tool> {
    if execution_mode == "manager" {
        return MANAGER_TOOLS
            .iter()
            .filter_map(|name| crate::tools::registry::get_tool_definition(name))
            .collect();
    }

    UNIVERSAL_TOOLS
        .iter()
        .filter(|name| {
            if execution_mode == "workforce" {
                !WORKFORCE_CHAT_EXCLUDED.contains(name)
            } else {
                true
            }
        })
        .filter_map(|name| crate::tools::registry::get_tool_definition(name))
        .collect()
}

/// Resolve tool definitions by step execution mode.
///
/// Includes universal tools, node mutation tools, and archetype-specific ones.
/// Used by DispatchStrategy (background agent) which needs the full tool set
/// including mutations that the conversational assistant cannot call directly.
pub(crate) fn resolve_step_tools(execution_mode: &str) -> Vec<Tool> {
    let archetype_specific: &[&str] = match execution_mode {
        "workforce" => &[
            "configure_team",
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
        .chain(NODE_MUTATION_TOOLS.iter())
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
        "workforce" => {
            const WORKFORCE_TOOLS: &[&str] = &[
                "configure_team",
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
