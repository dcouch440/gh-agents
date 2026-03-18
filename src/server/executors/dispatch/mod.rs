//! Background dispatch task runner.
//!
//! Spawns the ExecutionEngine with a DispatchStrategy to configure a step
//! based on a plain English instruction. Runs with NullSink (no streaming)
//! and broadcasts WebSocket events for lifecycle tracking.

use uuid::Uuid;

use crate::db::traits::CreateAgentExecutionInput;
use crate::server::hub::engine::ExecutionEngine;
use crate::server::hub::recorder::ExecutionRecorder;
use crate::server::hub::strategies::DispatchStrategy;
use crate::server::hub::strategy::ExecutionStrategy;
use crate::server::hub::streaming::DispatchStreamSink;
use crate::server::services::dispatch::Passdown;
use crate::server::state::AppState;
use crate::server::ws::events::{SessionEvent, SessionEventKind};
use crate::types::ExecutionType;
use crate::types::UserId;

pub(crate) mod designer_handoff;

mod tests;

/// Run a background dispatch task to completion.
///
/// Called from `tokio::spawn` in the chat dispatch handler. Builds a
/// DispatchStrategy, runs the engine loop, and updates the TaskRegistry
/// with the outcome.
///
/// `session_id` is the persistent L4 builder session — the instruction and
/// outcome are persisted as messages so subsequent dispatches see history.
#[allow(clippy::too_many_arguments)]
pub async fn run_dispatch_task(
    state: AppState,
    execution_id: Uuid,
    step_id: Uuid,
    workflow_id: Uuid,
    instruction: String,
    session_id: Uuid,
    user_id: UserId,
    previous_step_handoff: Vec<crate::server::services::dispatch::PreviousStepHandoff>,
    next_step_text: Vec<crate::server::services::dispatch::NextStepText>,
) {
    // Get the cancel token from the registry
    let cancel_token = match state.task_registry().get_task(execution_id) {
        Some(entry) => entry.cancel_token,
        None => {
            tracing::error!(
                execution_id = %execution_id,
                "Dispatch task not found in registry"
            );
            return;
        }
    };

    // Persist the instruction as a user message in the builder session.
    if let Err(e) = state
        .repos()
        .sessions
        .insert_session_message(
            user_id,
            session_id,
            Uuid::new_v4(),
            "user".to_string(),
            instruction.clone(),
        )
        .await
    {
        tracing::warn!(
            session_id = %session_id,
            error = %e,
            "Failed to persist dispatch instruction"
        );
    }

    // Build the strategy
    let mut strategy = match DispatchStrategy::new(
        state.clone(),
        step_id,
        workflow_id,
        instruction.clone(),
        Some(session_id),
    )
    .await
    {
        Ok(s) => s,
        Err(e) => {
            tracing::error!(
                execution_id = %execution_id,
                error = %e,
                "Failed to build DispatchStrategy"
            );
            persist_outcome(&state, session_id, user_id, &format!("Error: {e}")).await;
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

    // Get the LLM provider
    let provider = match state.provider() {
        Some(p) => p.clone(),
        None => {
            let err = "No LLM provider configured";
            tracing::error!(execution_id = %execution_id, err);
            persist_outcome(&state, session_id, user_id, err).await;
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

    // Create agent execution record for persistence
    let ae_id = match state
        .repos()
        .agent_executions
        .create_agent_execution(CreateAgentExecutionInput {
            execution_type: ExecutionType::Dispatch,
            agent_id: None,
            workflow_step_id: Some(step_id),
            parent_agent_execution_id: None,
            system_prompt_rendered: strategy.system_prompt().to_string(),
            input: instruction.clone(),
            room_session_id: None,
            speaker_order: None,
            workflow_execution_id: None,
        })
        .await
    {
        Ok(row) => Some(row.id),
        Err(e) => {
            tracing::warn!(
                execution_id = %execution_id,
                error = %e,
                "Failed to create agent execution record for dispatch"
            );
            None
        }
    };

    // Link agent execution ID to strategy for debug stream events
    strategy.set_agent_execution_id(ae_id);

    // Run the engine
    let engine = ExecutionEngine::new(provider, state.env().debug_stream);
    let recorder = ExecutionRecorder::new(
        &*state.repos().sessions,
        &*state.repos().chat_messages,
        Some(&*state.repos().agent_executions),
        Some(&*state.repos().token_ledger),
    );
    let sink = DispatchStreamSink::new(state.clone(), execution_id, step_id);

    let result = engine
        .execute(
            &strategy,
            &instruction,
            &sink,
            &recorder,
            Some(&cancel_token),
        )
        .await;

    match result {
        Ok(exec_result) => {
            // Retrieve captured passdown from strategy (or fallback)
            let raw_passdown = strategy.take_passdown();
            let has_passdown = raw_passdown.is_some();
            let passdown = raw_passdown.unwrap_or_else(|| Passdown {
                summary: if exec_result.content.is_empty() {
                    "Completed with no response".to_string()
                } else {
                    exec_result.content.clone()
                },
                question: None,
            });

            // Persist passdown as structured JSON in agent_execution output
            let passdown_json = serde_json::to_string(&passdown).unwrap_or_default();
            persist_outcome(&state, session_id, user_id, &passdown.summary).await;

            // Write question + summary into step_question_state for board state.
            // This replaces the old cheap-LLM question detection — the builder
            // declares its question explicitly via complete_task.
            if let Err(e) = state
                .repos()
                .workflows
                .upsert_step_question_state(step_id, &passdown.summary, passdown.question.clone())
                .await
            {
                tracing::warn!(
                    step_id = %step_id,
                    error = %e,
                    "Failed to upsert step question state from passdown"
                );
            }

            // Trigger designer handoff only if the builder produced a passdown
            // (i.e. called complete_task). Designer tool calls stream through the
            // same DispatchStreamSink (same execution_id) so the trace is continuous.
            if has_passdown {
                let changed_agents = strategy.take_changed_agents();
                designer_handoff::run_designer_after_builder(
                    &state,
                    step_id,
                    workflow_id,
                    user_id,
                    execution_id,
                    &instruction,
                    changed_agents,
                    previous_step_handoff,
                    next_step_text,
                )
                .await;
            }

            // Persist trace AFTER designer handoff so the persisted trace
            // includes both builder and designer tool events.
            persist_trace(
                &state,
                execution_id,
                ae_id,
                "completed",
                Some(&passdown_json),
            )
            .await;

            state
                .task_registry()
                .mark_completed(execution_id, Some(passdown.summary.clone()));

            broadcast_dispatch_event(
                &state,
                SessionEventKind::DispatchCompleted {
                    execution_id,
                    step_id,
                    summary: passdown.summary,
                    question: passdown.question,
                },
            );

            tracing::info!(
                execution_id = %execution_id,
                step_id = %step_id,
                rounds = exec_result.rounds_used,
                "Dispatch task completed (designer handoff done)"
            );
        }
        Err(e) => {
            let error_msg = e.to_string();
            persist_outcome(&state, session_id, user_id, &format!("Error: {error_msg}")).await;

            // Check if this was a cancellation
            if cancel_token.is_cancelled() {
                persist_trace(&state, execution_id, ae_id, "cancelled", None).await;
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
                    "Dispatch task cancelled"
                );
            } else {
                persist_trace(&state, execution_id, ae_id, "failed", Some(&error_msg)).await;
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
                    "Dispatch task failed"
                );
            }
        }
    }
}

/// Persist the dispatch outcome as an assistant message in the builder session.
async fn persist_outcome(state: &AppState, session_id: Uuid, user_id: UserId, content: &str) {
    if let Err(e) = state
        .repos()
        .sessions
        .insert_session_message(
            user_id,
            session_id,
            Uuid::new_v4(),
            "assistant".to_string(),
            content.to_string(),
        )
        .await
    {
        tracing::warn!(
            session_id = %session_id,
            error = %e,
            "Failed to persist dispatch outcome"
        );
    }
}

/// Persist the dispatch trace and update agent_execution status.
async fn persist_trace(
    state: &AppState,
    execution_id: Uuid,
    ae_id: Option<Uuid>,
    status: &str,
    output: Option<&str>,
) {
    let Some(ae_id) = ae_id else { return };

    // Serialize trace from in-memory TaskRegistry
    if let Some(task) = state.task_registry().get_task(execution_id) {
        if let Ok(trace_json) = serde_json::to_value(&task.trace) {
            if let Err(e) = state
                .repos()
                .agent_executions
                .update_execution_trace(ae_id, trace_json)
                .await
            {
                tracing::warn!(ae_id = %ae_id, error = %e, "Failed to persist dispatch trace");
            }
        }
    }

    // Update agent execution status
    if let Err(e) = state
        .repos()
        .agent_executions
        .update_agent_execution_status(ae_id, status, output.map(|s| s.to_string()), None)
        .await
    {
        tracing::warn!(ae_id = %ae_id, error = %e, "Failed to update dispatch agent execution status");
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
