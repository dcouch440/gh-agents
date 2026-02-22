//! Dispatch and cancel_dispatch tool handlers for chat sessions.
//!
//! These handlers bridge the chat assistant to the background dispatch system.
//! `dispatch` spawns a background agent task; `cancel_dispatch` cancels one.

use serde_json::{json, Value};
use uuid::Uuid;

use crate::server::state::AppState;
use crate::server::ws::events::{SessionEvent, SessionEventKind};
use crate::types::UserId;

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

    // Resolve user_id for both L2 and L4 dispatch paths.
    let user_id = resolve_dispatch_user_id(state, ctx).await;

    // Find-or-create the persistent builder session.
    let session_id = find_or_create_builder_session(state, ctx, user_id).await;

    // Register the task in the registry
    let (execution_id, _cancel_token) = state.task_registry().spawn_task(
        ctx.step_id,
        ctx.workflow_id,
        session_id,
        instruction.to_string(),
    );

    // Spawn the appropriate background runner based on execution mode
    let runner_state = state.clone();
    let runner_step_id = ctx.step_id;
    let runner_workflow_id = ctx.workflow_id;
    let runner_instruction = instruction.to_string();
    let runner_execution_id = execution_id;

    if ctx.execution_mode == "manager" {
        tokio::spawn(async move {
            crate::server::executors::manager_dispatch::run_manager_dispatch_task(
                runner_state,
                runner_execution_id,
                runner_workflow_id,
                user_id,
                runner_step_id,
                runner_instruction,
                session_id,
            )
            .await;
        });
    } else {
        tokio::spawn(async move {
            crate::server::executors::dispatch::run_dispatch_task(
                runner_state,
                runner_execution_id,
                runner_step_id,
                runner_workflow_id,
                runner_instruction,
                session_id,
                user_id,
            )
            .await;
        });
    }

    // Broadcast started event
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

/// Resolve the user_id for a dispatch task.
///
/// Looks up the workflow to get the owner. Falls back to a nil UUID
/// (the executor's ownership checks will catch this safely).
async fn resolve_dispatch_user_id(state: &AppState, ctx: &StepChatContext) -> UserId {
    match state.repos().workflows.get_workflow(ctx.workflow_id).await {
        Ok(Some(wf)) => UserId(wf.user_id),
        _ => UserId(Uuid::nil()),
    }
}

/// Find or create the persistent builder session for a dispatch agent.
///
/// L2 (manager mode): Looks up by `workflow_id` + `role = manager_builder`.
/// L4 (node mode): Looks up by `step_id` + `role = builder`.
///
/// Creates the session if it doesn't exist, using the workflow owner as
/// the session user.
async fn find_or_create_builder_session(
    state: &AppState,
    ctx: &StepChatContext,
    user_id: UserId,
) -> Uuid {
    let sessions = state.repos().sessions.as_ref();

    if ctx.execution_mode == "manager" {
        // L2: manager builder session, keyed by workflow_id
        match sessions.find_manager_builder_session(ctx.workflow_id).await {
            Ok(Some(session)) => return session.id,
            Ok(None) => {}
            Err(e) => {
                tracing::warn!(
                    workflow_id = %ctx.workflow_id,
                    error = %e,
                    "Failed to find manager builder session, creating new"
                );
            }
        }

        let session_id = Uuid::new_v4();
        let draft_config = json!({
            "workflow_id": ctx.workflow_id.to_string(),
            "role": "manager_builder",
        });

        if let Err(e) = sessions
            .create_session(
                user_id,
                session_id,
                "dispatch",
                "Manager Builder",
                None,
                Some(draft_config),
            )
            .await
        {
            tracing::error!(
                workflow_id = %ctx.workflow_id,
                error = %e,
                "Failed to create manager builder session"
            );
        }

        session_id
    } else {
        // L4: node builder session, keyed by step_id
        match sessions.find_builder_session_by_step_id(ctx.step_id).await {
            Ok(Some(session)) => return session.id,
            Ok(None) => {}
            Err(e) => {
                tracing::warn!(
                    step_id = %ctx.step_id,
                    error = %e,
                    "Failed to find builder session, creating new"
                );
            }
        }

        let session_id = Uuid::new_v4();
        let draft_config = json!({
            "step_id": ctx.step_id.to_string(),
            "workflow_id": ctx.workflow_id.to_string(),
            "role": "builder",
        });

        if let Err(e) = sessions
            .create_session(
                user_id,
                session_id,
                "dispatch",
                "Node Builder",
                None,
                Some(draft_config),
            )
            .await
        {
            tracing::error!(
                step_id = %ctx.step_id,
                error = %e,
                "Failed to create builder session"
            );
        }

        session_id
    }
}
