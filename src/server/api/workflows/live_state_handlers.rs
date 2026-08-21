//! Live-state handler — one call that describes everything currently happening
//! on a workflow, so a page refresh can rebuild the editor's view.
//!
//! Without this the frontend has to stitch together the run list, run detail and
//! a per-step dispatch lookup, and it guesses wrong in exactly the cases that
//! matter: a run that is still `pending` has no `started_at` and sorts last, and
//! dispatch state lives in an in-memory registry with no workflow-wide query.

use axum::{
    extract::{Path, State},
    Json,
};
use chrono::Utc;
use std::sync::Arc;
use uuid::Uuid;

use crate::db::pg_repo::PgRepo;
use crate::db::traits::WorkflowCollectionRepo;
use crate::db::WorkflowExecutionRow;
use crate::server::api::AppError;
use crate::server::auth as auth_utils;
use crate::server::services::workflow_state;
use crate::server::state::AppState;

use super::run_detail_handlers::build_run_steps;
use super::types::{
    LiveDispatchResponse, LiveStepBaselineResponse, WorkflowExecutionResponse,
    WorkflowLiveStateResponse,
};

fn to_execution_response(r: WorkflowExecutionRow) -> WorkflowExecutionResponse {
    WorkflowExecutionResponse {
        id: r.id,
        workflow_id: r.workflow_id,
        status: r.status,
        started_at: r.started_at,
        completed_at: r.completed_at,
        outputs: r.outputs,
        error: r.error,
        execution_mode: r.execution_mode,
        template_id: r.template_id,
    }
}

/// GET /api/workflows/:id/live-state — current run + per-node baseline + dispatches.
#[utoipa::path(
    get,
    path = "/api/workflows/{id}/live-state",
    tag = "Workflows",
    security(("bearer_auth" = [])),
    params(("id" = Uuid, Path, description = "Workflow ID")),
    responses(
        (status = 200, description = "Current live state", body = WorkflowLiveStateResponse),
        (status = 404, description = "Workflow not found")
    )
)]
pub async fn get_workflow_live_state(
    State(state): State<AppState>,
    auth: auth_utils::AuthUser,
    Path(id): Path<Uuid>,
) -> Result<Json<WorkflowLiveStateResponse>, AppError> {
    let workflow_repo = &state.repos().workflows;

    let workflow = workflow_repo
        .get_workflow(id)
        .await?
        .ok_or(AppError::not_found("Workflow"))?;
    if workflow.user_id != auth.user_id.0 {
        return Err(AppError::not_found("Workflow"));
    }

    let inputs = workflow_state::collect(&state, id).await?;

    let db = state
        .db()
        .ok_or(AppError::Internal("Database not available".into()))?
        .clone();
    let collection_repo: Arc<dyn WorkflowCollectionRepo> = Arc::new(PgRepo::new(db));

    // Active run first; fall back to the most recent finished run so the editor
    // still shows the last outputs when nothing is in flight.
    let active_run = match inputs.active_run_id {
        Some(run_id) => collection_repo
            .get_workflow_execution(run_id)
            .await
            .map_err(|e| AppError::Internal(e.to_string()))?,
        None => None,
    };

    let latest_run = match &active_run {
        Some(run) => Some(run.clone()),
        None => collection_repo
            .list_workflow_executions_by_workflow(id, auth.user_id.0)
            .await
            .map_err(|e| AppError::Internal(e.to_string()))?
            .into_iter()
            .next(),
    };

    let run_steps = match &latest_run {
        Some(run) => build_run_steps(&state, &inputs.steps, run.id).await,
        None => Vec::new(),
    };

    let persisted: Vec<_> = inputs.latest_dispatch_by_step.values().cloned().collect();
    let dispatches = workflow_state::merge_dispatches(&inputs.registry_tasks, &persisted);
    let generating = workflow_state::is_generating(&dispatches);

    let steps = inputs
        .steps
        .iter()
        .filter(|s| s.visible)
        .map(|step| LiveStepBaselineResponse {
            step_id: step.id,
            name: step.name.clone(),
            execution_mode: step.execution_mode.clone(),
            baseline_status: workflow_state::resolve_baseline_status(
                step,
                inputs.latest_dispatch_by_step.get(&step.id),
            )
            .to_string(),
            pinned: step.pinned,
            has_run_summary: !step.run_results_summary.is_empty(),
            is_running_in_active_run: inputs.running_step_ids.contains(&step.id),
        })
        .collect();

    let dispatches = dispatches
        .into_iter()
        .map(|d| LiveDispatchResponse {
            step_id: d.step_id,
            execution_id: d.execution_id,
            status: d.status,
            instruction: d.instruction,
            created_at: d.created_at,
            result: d.result,
            trace_len: d.trace_len,
            source: d.source.as_str().to_string(),
        })
        .collect();

    Ok(Json(WorkflowLiveStateResponse {
        workflow_id: id,
        server_time: Utc::now(),
        active_run: active_run.map(to_execution_response),
        latest_run: latest_run.map(to_execution_response),
        run_steps,
        steps,
        dispatches,
        generating,
    }))
}
