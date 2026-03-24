//! Background dispatch task runner.
//!
//! Runs the SystemNodeStrategy in a container to configure a step based on a
//! plain English instruction. Broadcasts WebSocket events for lifecycle tracking.

use uuid::Uuid;

use crate::server::state::AppState;
use crate::server::ws::events::{SessionEvent, SessionEventKind};
use crate::types::UserId;

pub(crate) mod system_node;

mod tests;

/// Persist the dispatch outcome as an assistant message in the builder session.
pub(super) async fn persist_outcome(
    state: &AppState,
    session_id: Uuid,
    user_id: UserId,
    content: &str,
) {
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
pub(super) async fn persist_trace(
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
pub(super) fn broadcast_dispatch_event(state: &AppState, kind: SessionEventKind) {
    state.broadcast_session(SessionEvent {
        session_id: Uuid::nil(),
        user_id: None,
        kind,
    });
}
