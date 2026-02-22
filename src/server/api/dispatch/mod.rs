//! Dispatch trace API handlers.
//!
//! Provides endpoints for querying dispatch task execution traces and
//! dispatch task lists. Trace data comes from the in-memory task registry.

use axum::extract::{Path, State};
use axum::Json;
use serde::Serialize;
use uuid::Uuid;

use crate::server::api::AppError;
use crate::server::auth::AuthUser;
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
