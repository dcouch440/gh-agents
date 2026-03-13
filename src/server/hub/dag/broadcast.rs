//! WebSocket event broadcasting for workflow lifecycle events.

use uuid::Uuid;

use crate::server::state::AppState;
use crate::server::ws::events::{WorkflowEvent, WorkflowEventKind};

use super::utils::WorkflowExecutionContext;

/// Emit a workflow lifecycle event via WebSocket broadcast.
pub fn broadcast_workflow_event(
    state: &AppState,
    ctx: &WorkflowExecutionContext,
    workflow_id: Uuid,
    kind: WorkflowEventKind,
) {
    state.broadcast_workflow(WorkflowEvent {
        run_id: Some(ctx.run_id),
        workflow_id,
        user_id: Some(ctx.user_id),
        kind,
    });
}
