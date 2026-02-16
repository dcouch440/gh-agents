//! Run detail handlers — per-step results for a specific historical execution.

use axum::{
    extract::{Path, State},
    Json,
};
use std::sync::Arc;

use crate::db::pg_repo::PgRepo;
use crate::db::traits::WorkflowCollectionRepo;
use crate::server::api::AppError;
use crate::server::auth as auth_utils;
use crate::server::state::AppState;

use super::last_run_handlers::build_step_run_response;
use super::types::{
    ExecutionPath, ExecutionStepPath, RunDetailResponse, RunStepResultResponse,
    WorkflowExecutionResponse,
};

/// GET /api/workflows/:wid/executions/:eid/steps — All step results for a specific run.
#[utoipa::path(
    get,
    path = "/api/workflows/{wid}/executions/{eid}/steps",
    tag = "Workflows",
    security(("bearer_auth" = [])),
    params(
        ("wid" = Uuid, Path, description = "Workflow ID"),
        ("eid" = Uuid, Path, description = "Execution ID"),
    ),
    responses(
        (status = 200, description = "Run detail with per-step results", body = RunDetailResponse),
        (status = 404, description = "Workflow or execution not found")
    )
)]
pub async fn get_run_detail(
    State(state): State<AppState>,
    auth: auth_utils::AuthUser,
    Path(path): Path<ExecutionPath>,
) -> Result<Json<RunDetailResponse>, AppError> {
    let workflow_repo = &state.repos().workflows;

    // Verify workflow ownership
    let workflow = workflow_repo
        .get_workflow(path.wid)
        .await?
        .ok_or(AppError::not_found("Workflow"))?;
    if workflow.user_id != auth.user_id.0 {
        return Err(AppError::not_found("Workflow"));
    }

    // Load execution and verify it belongs to this workflow
    let db = state
        .db()
        .ok_or(AppError::Internal("Database not available".into()))?
        .clone();
    let collection_repo: Arc<dyn WorkflowCollectionRepo> = Arc::new(PgRepo::new(db));
    let execution = collection_repo
        .get_workflow_execution(path.eid)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?
        .ok_or(AppError::not_found("Execution"))?;
    if execution.workflow_id != path.wid {
        return Err(AppError::not_found("Execution"));
    }

    // Load all steps for the workflow
    let steps = workflow_repo.list_steps(path.wid).await?;

    // Build per-step results
    let mut step_results = Vec::new();
    let mut total_input_tokens: i64 = 0;
    let mut total_output_tokens: i64 = 0;
    let mut total_cost_usd: f64 = 0.0;

    for step in &steps {
        // Skip context/input steps that have no execution data
        if step.execution_mode == "context" || step.execution_mode == "input" {
            continue;
        }

        match build_step_run_response(&state, step, path.eid).await {
            Ok(resp) => {
                total_input_tokens += resp.input_tokens.unwrap_or(0);
                total_output_tokens += resp.output_tokens.unwrap_or(0);
                total_cost_usd += resp.cost_usd.unwrap_or(0.0);

                step_results.push(RunStepResultResponse {
                    step_id: step.id,
                    step_name: step.name.clone(),
                    execution_mode: step.execution_mode.clone(),
                    execution_id: Some(resp.execution_id),
                    status: resp.status,
                    started_at: resp.started_at,
                    completed_at: resp.completed_at,
                    duration_ms: resp.duration_ms,
                    output: resp.output,
                    structured_output: resp.structured_output,
                    input_tokens: resp.input_tokens,
                    output_tokens: resp.output_tokens,
                    cost_usd: resp.cost_usd,
                    error: resp.error,
                    phases: resp.phases,
                    child_execution_id: resp.child_execution_id,
                    child_steps: resp.child_steps,
                });
            }
            Err(_) => {
                // Step may not have been executed in this run (e.g., skipped by conditional edge)
                step_results.push(RunStepResultResponse {
                    step_id: step.id,
                    step_name: step.name.clone(),
                    execution_mode: step.execution_mode.clone(),
                    execution_id: None,
                    status: "skipped".to_string(),
                    started_at: None,
                    completed_at: None,
                    duration_ms: None,
                    output: None,
                    structured_output: None,
                    input_tokens: None,
                    output_tokens: None,
                    cost_usd: None,
                    error: None,
                    phases: None,
                    child_execution_id: None,
                    child_steps: None,
                });
            }
        }
    }

    // Duration from execution timestamps
    let duration_ms = match (execution.started_at, execution.completed_at) {
        (Some(start), Some(end)) => Some((end - start).num_milliseconds().unsigned_abs()),
        _ => None,
    };

    // Look up template name (gracefully handle deleted templates)
    let template_name = if let Some(tid) = execution.template_id {
        workflow_repo
            .get_template(tid)
            .await
            .ok()
            .flatten()
            .map(|t| t.name)
    } else {
        None
    };

    let exec_response = WorkflowExecutionResponse {
        id: execution.id,
        workflow_id: execution.workflow_id,
        status: execution.status,
        started_at: execution.started_at,
        completed_at: execution.completed_at,
        outputs: execution.outputs,
        error: execution.error,
        execution_mode: execution.execution_mode,
        template_id: execution.template_id,
    };

    Ok(Json(RunDetailResponse {
        execution: exec_response,
        steps: step_results,
        total_input_tokens,
        total_output_tokens,
        total_cost_usd,
        duration_ms,
        template_name,
    }))
}

/// GET /api/workflows/:wid/executions/:eid/steps/:sid — Single step result for a specific run.
#[utoipa::path(
    get,
    path = "/api/workflows/{wid}/executions/{eid}/steps/{sid}",
    tag = "Workflows",
    security(("bearer_auth" = [])),
    params(
        ("wid" = Uuid, Path, description = "Workflow ID"),
        ("eid" = Uuid, Path, description = "Execution ID"),
        ("sid" = Uuid, Path, description = "Step ID"),
    ),
    responses(
        (status = 200, description = "Step execution result", body = RunStepResultResponse),
        (status = 404, description = "Workflow, execution, or step not found")
    )
)]
pub async fn get_step_run_for_execution(
    State(state): State<AppState>,
    auth: auth_utils::AuthUser,
    Path(path): Path<ExecutionStepPath>,
) -> Result<Json<RunStepResultResponse>, AppError> {
    let workflow_repo = &state.repos().workflows;

    // Verify workflow ownership
    let workflow = workflow_repo
        .get_workflow(path.wid)
        .await?
        .ok_or(AppError::not_found("Workflow"))?;
    if workflow.user_id != auth.user_id.0 {
        return Err(AppError::not_found("Workflow"));
    }

    // Verify step belongs to workflow
    let step = workflow_repo
        .get_step(path.sid)
        .await?
        .ok_or(AppError::not_found("Step"))?;
    if step.workflow_id != path.wid {
        return Err(AppError::not_found("Step"));
    }

    // Load execution and verify it belongs to this workflow
    let db = state
        .db()
        .ok_or(AppError::Internal("Database not available".into()))?
        .clone();
    let collection_repo: Arc<dyn WorkflowCollectionRepo> = Arc::new(PgRepo::new(db));
    let execution = collection_repo
        .get_workflow_execution(path.eid)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?
        .ok_or(AppError::not_found("Execution"))?;
    if execution.workflow_id != path.wid {
        return Err(AppError::not_found("Execution"));
    }

    let resp = build_step_run_response(&state, &step, path.eid).await?;

    Ok(Json(RunStepResultResponse {
        step_id: step.id,
        step_name: step.name.clone(),
        execution_mode: step.execution_mode.clone(),
        execution_id: Some(resp.execution_id),
        status: resp.status,
        started_at: resp.started_at,
        completed_at: resp.completed_at,
        duration_ms: resp.duration_ms,
        output: resp.output,
        structured_output: resp.structured_output,
        input_tokens: resp.input_tokens,
        output_tokens: resp.output_tokens,
        cost_usd: resp.cost_usd,
        error: resp.error,
        phases: resp.phases,
        child_execution_id: resp.child_execution_id,
        child_steps: resp.child_steps,
    }))
}
