//! Dispatch API handlers.
//!
//! Provides endpoints for:
//! - Querying dispatch task execution traces and task lists
//! - Sending instructions directly to builder agents (direct dispatch)
//! - Cancelling running dispatch tasks

use axum::extract::{Path, State};
use axum::Json;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::server::api::AppError;
use crate::server::auth::AuthUser;
use crate::server::services::dispatch::{self, DispatchInput};
use crate::server::state::task_registry::TraceEvent;
use crate::server::state::AppState;

mod tests;

// ── Response Types ─────────────────────────────────────────────────────────

/// Response for a single dispatch task's trace.
#[derive(Debug, Serialize)]
pub struct DispatchTraceResponse {
    pub execution_id: Uuid,
    pub step_id: Uuid,
    pub workflow_id: Uuid,
    pub status: String,
    pub instruction: String,
    pub trace: Vec<TraceEvent>,
    pub result: Option<String>,
}

/// Summary of a dispatch task (no trace data).
#[derive(Debug, Serialize)]
pub struct DispatchTaskSummary {
    pub execution_id: Uuid,
    pub step_id: Uuid,
    pub status: String,
    pub instruction: String,
    pub result: Option<String>,
    pub trace_len: usize,
    pub created_at: String,
}

/// Response listing dispatch tasks for a step.
#[derive(Debug, Serialize)]
pub struct DispatchTasksResponse {
    pub tasks: Vec<DispatchTaskSummary>,
}

/// Request body for direct dispatch.
#[derive(Debug, Deserialize)]
pub struct DispatchSendRequest {
    pub instruction: String,
    pub workflow_id: Uuid,
}

/// Response for dispatch send/cancel operations.
#[derive(Debug, Serialize)]
pub struct DispatchActionResponse {
    pub execution_id: Uuid,
    pub status: String,
}

// ── Handlers ───────────────────────────────────────────────────────────────

/// GET /dispatch/:execution_id/trace — fetch the full execution trace.
pub async fn get_dispatch_trace(
    State(state): State<AppState>,
    _auth: AuthUser,
    Path(execution_id): Path<Uuid>,
) -> Result<Json<DispatchTraceResponse>, AppError> {
    let entry = state
        .task_registry()
        .get_task(execution_id)
        .ok_or_else(|| AppError::not_found("Dispatch task"))?;

    Ok(Json(DispatchTraceResponse {
        execution_id: entry.execution_id,
        step_id: entry.step_id,
        workflow_id: entry.workflow_id,
        status: entry.status.as_str().to_string(),
        instruction: entry.instruction,
        trace: entry.trace,
        result: entry.result,
    }))
}

/// GET /dispatch/step/:step_id — list all dispatch tasks for a step.
pub async fn list_dispatch_tasks(
    State(state): State<AppState>,
    _auth: AuthUser,
    Path(step_id): Path<Uuid>,
) -> Result<Json<DispatchTasksResponse>, AppError> {
    let tasks = state.task_registry().list_tasks_for_step(step_id);

    let summaries = tasks
        .into_iter()
        .map(|entry| DispatchTaskSummary {
            execution_id: entry.execution_id,
            step_id: entry.step_id,
            status: entry.status.as_str().to_string(),
            instruction: entry.instruction,
            result: entry.result,
            trace_len: entry.trace.len(),
            created_at: entry.created_at.to_rfc3339(),
        })
        .collect();

    Ok(Json(DispatchTasksResponse { tasks: summaries }))
}

/// POST /dispatch/step/:step_id/send — send an instruction directly to a builder.
///
/// Bypasses the chat assistant, dispatching straight to the L4 node builder.
/// Looks up the step's execution_mode to route to the correct executor.
pub async fn dispatch_send(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(step_id): Path<Uuid>,
    Json(body): Json<DispatchSendRequest>,
) -> Result<Json<DispatchActionResponse>, AppError> {
    if body.instruction.trim().is_empty() {
        return Err(AppError::bad_request("Instruction cannot be empty"));
    }

    // Load step to get execution_mode
    let step = state
        .repos()
        .workflows
        .get_step(step_id)
        .await
        .map_err(|e| AppError::Internal(format!("Failed to load step: {e}")))?
        .ok_or_else(|| AppError::not_found("Step"))?;

    let user_id = auth.user_id;

    let output = dispatch::dispatch_to_builder(
        &state,
        DispatchInput {
            step_id,
            workflow_id: body.workflow_id,
            user_id,
            instruction: body.instruction,
            execution_mode: step.execution_mode,
        },
    )
    .await;

    Ok(Json(DispatchActionResponse {
        execution_id: output.execution_id,
        status: "dispatched".to_string(),
    }))
}

/// POST /dispatch/:execution_id/cancel — cancel a running dispatch task.
pub async fn dispatch_cancel(
    State(state): State<AppState>,
    _auth: AuthUser,
    Path(execution_id): Path<Uuid>,
) -> Result<Json<DispatchActionResponse>, AppError> {
    // Look up the task to get step_id for the cancel broadcast
    let step_id = state
        .task_registry()
        .get_task(execution_id)
        .map(|e| e.step_id)
        .unwrap_or(Uuid::nil());

    let cancelled = dispatch::cancel_dispatch(&state, execution_id, step_id);

    Ok(Json(DispatchActionResponse {
        execution_id,
        status: if cancelled {
            "cancelled".to_string()
        } else {
            "not_found_or_already_complete".to_string()
        },
    }))
}
