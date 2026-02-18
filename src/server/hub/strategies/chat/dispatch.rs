//! Dispatch and cancel_dispatch tool handlers for chat sessions.
//!
//! These handlers bridge the chat assistant to the background dispatch system.
//! `dispatch` spawns a background agent task; `cancel_dispatch` cancels one.

use serde_json::{json, Value};
use uuid::Uuid;

use crate::server::state::AppState;
use crate::server::ws::events::{SessionEvent, SessionEventKind};

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

/// Spawn a background dispatch task.
async fn handle_dispatch(input: &Value, state: &AppState, ctx: &StepChatContext) -> Value {
    let Some(instruction) = input["instruction"].as_str() else {
        return json!({ "error": "Missing required parameter: instruction" });
    };

    if instruction.trim().is_empty() {
        return json!({ "error": "Instruction cannot be empty" });
    }

    // Register the task in the registry
    let (execution_id, _cancel_token) = state.task_registry().spawn_task(
        ctx.step_id,
        ctx.workflow_id,
        // Session ID is not available in StepChatContext, use a nil UUID.
        // The runner will update this if needed.
        Uuid::nil(),
        instruction.to_string(),
    );

    // Spawn the background runner
    let runner_state = state.clone();
    let runner_step_id = ctx.step_id;
    let runner_workflow_id = ctx.workflow_id;
    let runner_instruction = instruction.to_string();
    let runner_execution_id = execution_id;

    tokio::spawn(async move {
        crate::server::executors::dispatch::run_dispatch_task(
            runner_state,
            runner_execution_id,
            runner_step_id,
            runner_workflow_id,
            runner_instruction,
        )
        .await;
    });

    // Broadcast started event
    // Session ID not available here, so we use step_id as the session scope.
    // The frontend subscribes to session events and matches by step_id.
    state.broadcast_session(SessionEvent {
        session_id: Uuid::nil(),
        user_id: None,
        kind: SessionEventKind::DispatchStarted {
            execution_id,
            step_id: ctx.step_id,
            instruction: instruction.to_string(),
        },
    });

    json!({
        "execution_id": execution_id.to_string(),
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

    let cancelled = state.task_registry().cancel_task(execution_id);

    if cancelled {
        state.broadcast_session(SessionEvent {
            session_id: Uuid::nil(),
            user_id: None,
            kind: SessionEventKind::DispatchCancelled {
                execution_id,
                step_id: ctx.step_id,
            },
        });

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
