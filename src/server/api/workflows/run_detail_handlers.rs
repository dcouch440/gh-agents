//! Run detail handlers — per-step results for a specific historical execution.

use axum::{
    body::Body,
    extract::{Path, State},
    http::{header, StatusCode},
    response::Response,
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
    let step_results = build_run_steps(&state, &steps, path.eid).await;

    let mut total_input_tokens: i64 = 0;
    let mut total_output_tokens: i64 = 0;
    let mut total_cost_usd: f64 = 0.0;
    for resp in &step_results {
        total_input_tokens += resp.input_tokens.unwrap_or(0);
        total_output_tokens += resp.output_tokens.unwrap_or(0);
        total_cost_usd += resp.cost_usd.unwrap_or(0.0);
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

/// Build per-step results for one execution.
///
/// Shared by `get_run_detail` and the live-state endpoint so both describe a run
/// identically. Steps with no execution data for this run report `"skipped"`;
/// `context`/`input` steps are omitted because they never execute.
pub(crate) async fn build_run_steps(
    state: &AppState,
    steps: &[crate::db::WorkflowStepRow],
    execution_id: uuid::Uuid,
) -> Vec<RunStepResultResponse> {
    let mut step_results = Vec::new();

    for step in steps {
        if step.execution_mode == "context" || step.execution_mode == "input" {
            continue;
        }

        match build_step_run_response(state, step, execution_id).await {
            Ok(resp) => step_results.push(RunStepResultResponse {
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
            }),
            // Step may not have been executed in this run (e.g. skipped by a
            // conditional edge, or the run has not reached it yet).
            Err(_) => step_results.push(RunStepResultResponse {
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
            }),
        }
    }

    step_results
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
    }))
}

/// GET /api/workflows/:wid/executions/:eid/files — Download all run files as a zip archive.
#[utoipa::path(
    get,
    path = "/api/workflows/{wid}/executions/{eid}/files",
    tag = "Workflows",
    security(("bearer_auth" = [])),
    params(
        ("wid" = Uuid, Path, description = "Workflow ID"),
        ("eid" = Uuid, Path, description = "Execution ID"),
    ),
    responses(
        (status = 200, description = "Zip archive of run files", content_type = "application/zip"),
        (status = 404, description = "Workflow, execution, or no files found")
    )
)]
pub async fn download_run_files(
    State(state): State<AppState>,
    auth: auth_utils::AuthUser,
    Path(path): Path<ExecutionPath>,
) -> Result<Response, AppError> {
    // Verify workflow ownership
    let workflow = state
        .repos()
        .workflows
        .get_workflow(path.wid)
        .await?
        .ok_or(AppError::not_found("Workflow"))?;
    if workflow.user_id != auth.user_id.0 {
        return Err(AppError::not_found("Workflow"));
    }

    // Verify execution belongs to this workflow
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

    // Read files from JuiceFS workspace
    let workspace = state
        .workspace()
        .ok_or(AppError::Internal("Workspace not available".into()))?;

    let files = workspace
        .list_files(path.wid, path.eid, None)
        .map_err(|e| AppError::Internal(e.to_string()))?;

    if files.is_empty() {
        return Err(AppError::not_found("Run files"));
    }

    // Build zip in memory from workspace files
    let buf = std::io::Cursor::new(Vec::new());
    let mut zip = zip::ZipWriter::new(buf);
    let options = zip::write::SimpleFileOptions::default()
        .compression_method(zip::CompressionMethod::Deflated);

    let mut count = 0usize;
    for rel_path in &files {
        // Skip internal metadata
        if rel_path.starts_with(".nexor") {
            continue;
        }

        let content = workspace
            .read_file(path.wid, path.eid, rel_path)
            .map_err(|e| AppError::Internal(e.to_string()))?;
        let Some(bytes) = content else { continue };

        let name = rel_path.to_string_lossy();
        zip.start_file(name.as_ref(), options)
            .map_err(|e| AppError::Internal(e.to_string()))?;
        std::io::Write::write_all(&mut zip, &bytes)
            .map_err(|e| AppError::Internal(e.to_string()))?;
        count += 1;
    }

    if count == 0 {
        return Err(AppError::not_found("Run files"));
    }

    let zip_bytes = zip
        .finish()
        .map_err(|e| AppError::Internal(e.to_string()))?
        .into_inner();

    let filename = format!("run-{}.zip", &path.eid.to_string()[..8]);
    Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, "application/zip")
        .header(
            header::CONTENT_DISPOSITION,
            format!("attachment; filename=\"{filename}\""),
        )
        .header(header::CONTENT_LENGTH, zip_bytes.len())
        .body(Body::from(zip_bytes))
        .map_err(|e| AppError::Internal(e.to_string()))
}
