//! Dispatch and cancel_dispatch tool handlers for chat sessions.
//!
//! These handlers bridge the chat assistant to the background dispatch system.
//! Delegates to the shared dispatch service for session management and spawning.

use serde_json::{json, Value};
use uuid::Uuid;

use crate::server::services::dispatch::{self, DispatchInput};
use crate::server::state::AppState;

use super::config::StepChatContext;

/// Handle the `dispatch` or `cancel_dispatch` tool call.
pub(crate) async fn handle_dispatch_tool(
    name: &str,
    input: &Value,
    state: &AppState,
    ctx: &StepChatContext,
) -> Value {
    match name {
        "dispatch" => handle_dispatch(input, state, ctx).await,
        "cancel_dispatch" => handle_cancel_dispatch(input, state, ctx).await,
        _ => json!({ "error": format!("Unknown dispatch tool: {}", name) }),
    }
}

/// Spawn a background dispatch task via the shared dispatch service.
async fn handle_dispatch(input: &Value, state: &AppState, ctx: &StepChatContext) -> Value {
    let Some(instruction) = input["instruction"].as_str() else {
        return json!({ "error": "Missing required parameter: instruction" });
    };

    if instruction.trim().is_empty() {
        return json!({ "error": "Instruction cannot be empty" });
    }

    // Resolve user_id for both L2 and L4 dispatch paths.
    let user_id = dispatch::resolve_dispatch_user_id(state, ctx.workflow_id).await;

    let output = dispatch::dispatch_to_builder(
        state,
        DispatchInput {
            step_id: ctx.step_id,
            workflow_id: ctx.workflow_id,
            user_id,
            instruction: instruction.to_string(),
            execution_mode: ctx.execution_mode.clone(),
            stroke_image: None,
        },
    )
    .await;

    json!({
        "execution_id": output.execution_id.to_string(),
        "status": "dispatched",
    })
}

/// Cancel a running background dispatch task.
async fn handle_cancel_dispatch(input: &Value, state: &AppState, ctx: &StepChatContext) -> Value {
    let Some(id_str) = input["execution_id"].as_str() else {
        return json!({ "error": "Missing required parameter: execution_id" });
    };

    let Ok(execution_id) = Uuid::parse_str(id_str) else {
        return json!({ "error": format!("Invalid UUID: {}", id_str) });
    };

    let cancelled = dispatch::cancel_dispatch(state, execution_id, ctx.step_id);

    if cancelled {
        json!({
            "execution_id": execution_id.to_string(),
            "status": "cancelled",
        })
    } else {
        json!({
            "execution_id": execution_id.to_string(),
            "status": "not_found_or_already_complete",
        })
    }
}
