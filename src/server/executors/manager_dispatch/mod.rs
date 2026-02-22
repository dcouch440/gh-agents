//! Background manager dispatch task runner.
//!
//! Spawns the ExecutionEngine with a ManagerDispatchStrategy to create/modify
//! workflow topology and dispatch instructions to nodes. Runs with NullSink
//! (no streaming) and broadcasts WebSocket events for lifecycle tracking.

use uuid::Uuid;

use crate::server::hub::engine::ExecutionEngine;
use crate::server::hub::recorder::ExecutionRecorder;
use crate::server::hub::strategies::ManagerDispatchStrategy;
use crate::server::hub::NullSink;
use crate::server::state::AppState;
use crate::server::ws::events::{SessionEvent, SessionEventKind};
use crate::types::UserId;

mod tests;

/// Run a background manager dispatch task to completion.
///
/// Called from `tokio::spawn` in the chat dispatch handler when the step
/// has `execution_mode = "manager"`. Builds a ManagerDispatchStrategy,
/// runs the engine loop, and updates the TaskRegistry with the outcome.
pub async fn run_manager_dispatch_task(
    state: AppState,
    execution_id: Uuid,
    workflow_id: Uuid,
    user_id: UserId,
    step_id: Uuid,
    instruction: String,
) {
    let cancel_token = match state.task_registry().get_task(execution_id) {
        Some(entry) => entry.cancel_token,
        None => {
            tracing::error!(
                execution_id = %execution_id,
                "Manager dispatch task not found in registry"
            );
            return;
        }
    };

    let strategy = match ManagerDispatchStrategy::new(
        state.clone(),
        workflow_id,
        user_id,
        instruction.clone(),
    )
    .await
    {
        Ok(s) => s,
        Err(e) => {
            tracing::error!(
                execution_id = %execution_id,
                error = %e,
                "Failed to build ManagerDispatchStrategy"
            );
            state
                .task_registry()
                .mark_failed(execution_id, e.to_string());
            broadcast_dispatch_event(
                &state,
                SessionEventKind::DispatchFailed {
                    execution_id,
                    step_id,
                    error: e.to_string(),
                },
            );
            return;
        }
    };

    let provider = match state.provider() {
        Some(p) => p.clone(),
        None => {
            let err = "No LLM provider configured";
            tracing::error!(execution_id = %execution_id, err);
            state
                .task_registry()
                .mark_failed(execution_id, err.to_string());
            broadcast_dispatch_event(
                &state,
                SessionEventKind::DispatchFailed {
                    execution_id,
                    step_id,
                    error: err.to_string(),
                },
            );
            return;
        }
    };

    let engine = ExecutionEngine::new(provider);
    let recorder = ExecutionRecorder::new(
        &*state.repos().sessions,
        &*state.repos().chat_messages,
        None,
        None,
    );

    let result = engine
        .execute(
            &strategy,
            &instruction,
            &NullSink,
            &recorder,
            Some(&cancel_token),
        )
        .await;

    match result {
        Ok(exec_result) => {
            let summary = if exec_result.content.is_empty() {
                "Completed with no response".to_string()
            } else {
                exec_result.content.clone()
            };

            state
                .task_registry()
                .mark_completed(execution_id, Some(summary.clone()));

            broadcast_dispatch_event(
                &state,
                SessionEventKind::DispatchCompleted {
                    execution_id,
                    step_id,
                    summary,
                },
            );

            tracing::info!(
                execution_id = %execution_id,
                workflow_id = %workflow_id,
                rounds = exec_result.rounds_used,
                "Manager dispatch task completed"
            );
        }
        Err(e) => {
            let error_msg = e.to_string();

            if cancel_token.is_cancelled() {
                state.task_registry().cancel_task(execution_id);
                broadcast_dispatch_event(
                    &state,
                    SessionEventKind::DispatchCancelled {
                        execution_id,
                        step_id,
                    },
                );
                tracing::info!(
                    execution_id = %execution_id,
                    "Manager dispatch task cancelled"
                );
            } else {
                state
                    .task_registry()
                    .mark_failed(execution_id, error_msg.clone());
                broadcast_dispatch_event(
                    &state,
                    SessionEventKind::DispatchFailed {
                        execution_id,
                        step_id,
                        error: error_msg,
                    },
                );
                tracing::error!(
                    execution_id = %execution_id,
                    error = %e,
                    "Manager dispatch task failed"
                );
            }
        }
    }
}

/// Broadcast a dispatch event on the session topic.
fn broadcast_dispatch_event(state: &AppState, kind: SessionEventKind) {
    state.broadcast_session(SessionEvent {
        session_id: Uuid::nil(),
        user_id: None,
        kind,
    });
}
