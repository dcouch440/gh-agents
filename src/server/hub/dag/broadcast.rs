//! WebSocket event broadcasting for workflow lifecycle events.
//!
//! Handles both direct broadcasting on the workflow's channel and
//! sub-workflow relay to the parent's channel.

use uuid::Uuid;

use crate::server::state::AppState;
use crate::server::ws::events::{WorkflowEvent, WorkflowEventKind};

use super::utils::{SubWorkflowParentContext, WorkflowExecutionContext};

/// Emit a workflow lifecycle event via WebSocket broadcast.
///
/// When executing inside a sub-workflow (i.e. `ctx.parent_context` is set),
/// step-level events are also relayed to the parent's channel as
/// `SubWorkflowStepProgress` so the frontend can render nested execution.
pub fn broadcast_workflow_event(
    state: &AppState,
    ctx: &WorkflowExecutionContext,
    workflow_id: Uuid,
    kind: WorkflowEventKind,
) {
    // If executing inside a sub-workflow, relay step events to parent's channel
    if let Some(parent) = &ctx.parent_context {
        if let Some(relay_kind) = build_parent_relay(&kind, parent, ctx.run_id) {
            state.broadcast_workflow(WorkflowEvent {
                run_id: Some(parent.parent_run_id),
                workflow_id: parent.parent_workflow_id,
                user_id: Some(ctx.user_id),
                kind: relay_kind,
            });
        }
    }

    // Broadcast original event on the current channel
    state.broadcast_workflow(WorkflowEvent {
        run_id: Some(ctx.run_id),
        workflow_id,
        user_id: Some(ctx.user_id),
        kind,
    });
}

/// Build a `SubWorkflowStepProgress` relay event for the parent's channel.
///
/// Returns `Some` for step lifecycle events (started/completed/failed),
/// `None` for workflow-level or progress events.
pub(crate) fn build_parent_relay(
    kind: &WorkflowEventKind,
    parent: &SubWorkflowParentContext,
    child_execution_id: Uuid,
) -> Option<WorkflowEventKind> {
    match kind {
        WorkflowEventKind::StepStarted {
            step_id, step_name, ..
        } => Some(WorkflowEventKind::SubWorkflowStepProgress {
            parent_step_id: parent.parent_step_id,
            child_execution_id,
            child_step_id: *step_id,
            child_step_name: step_name.clone(),
            status: "started".into(),
            input_tokens: None,
            output_tokens: None,
            duration_ms: None,
            error: None,
        }),
        WorkflowEventKind::StepCompleted {
            step_id,
            step_name,
            input_tokens,
            output_tokens,
            duration_ms,
            ..
        } => Some(WorkflowEventKind::SubWorkflowStepProgress {
            parent_step_id: parent.parent_step_id,
            child_execution_id,
            child_step_id: *step_id,
            child_step_name: step_name.clone(),
            status: "completed".into(),
            input_tokens: *input_tokens,
            output_tokens: *output_tokens,
            duration_ms: *duration_ms,
            error: None,
        }),
        WorkflowEventKind::StepFailed {
            step_id,
            step_name,
            error,
        } => Some(WorkflowEventKind::SubWorkflowStepProgress {
            parent_step_id: parent.parent_step_id,
            child_execution_id,
            child_step_id: *step_id,
            child_step_name: step_name.clone(),
            status: "failed".into(),
            input_tokens: None,
            output_tokens: None,
            duration_ms: None,
            error: Some(error.clone()),
        }),
        _ => None,
    }
}
