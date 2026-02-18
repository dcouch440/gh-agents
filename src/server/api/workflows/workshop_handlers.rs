//! Workshop (node-by-node) workflow execution handlers

use axum::{
    extract::{Path, State},
    Json,
};
use std::collections::HashMap;
use std::sync::Arc;
use tracing::warn;
use uuid::Uuid;

use crate::db::pg_repo::PgRepo;
use crate::db::traits::WorkflowCollectionRepo;
use crate::server::api::AppError;
use crate::server::auth as auth_utils;
use crate::server::hub::dag::staging::{
    compute_next_executable_steps, execute_staged_step, reconstruct_dag_state_from_snapshots,
};
use crate::server::hub::dag::{
    broadcast_workflow_event, prefetch_port_metadata, WorkflowExecutionContext,
};
use crate::server::state::AppState;
use crate::server::ws::events::WorkflowEventKind;

use super::types::{
    CreateWorkshopRequest, WorkshopResponse, WorkshopStatusResponse, WorkshopStepPath,
    WorkshopStepResponse, WorkshopStepSummary,
};

/// POST /api/workflows/:id/workshop - Get or create the workshop for node-by-node execution.
#[utoipa::path(
    post,
    path = "/api/workflows/{id}/workshop",
    tag = "Workflows",
    security(("bearer_auth" = [])),
    params(("id" = Uuid, Path, description = "Workflow ID")),
    request_body(content = Option<CreateWorkshopRequest>, content_type = "application/json"),
    responses(
        (status = 200, description = "Workshop retrieved or created", body = WorkshopResponse),
        (status = 404, description = "Workflow not found")
    )
)]
pub async fn get_or_create_workshop(
    State(state): State<AppState>,
    auth: auth_utils::AuthUser,
    Path(id): Path<Uuid>,
    body: Option<Json<CreateWorkshopRequest>>,
) -> Result<Json<WorkshopResponse>, AppError> {
    let workflow_repo = &state.repos().workflows;

    // Verify workflow exists and user owns it
    let workflow = workflow_repo
        .get_workflow(id)
        .await?
        .ok_or(AppError::not_found("Workflow"))?;
    if workflow.user_id != auth.user_id.0 {
        return Err(AppError::not_found("Workflow"));
    }

    // Verify workflow has steps
    let steps = workflow_repo.list_steps(id).await?;
    if steps.is_empty() {
        return Err(AppError::bad_request("Workflow has no steps"));
    }

    // Get or create the workshop execution row
    let db = state
        .db()
        .ok_or(AppError::Internal("Database not available".into()))?
        .clone();
    let collection_repo: Arc<dyn WorkflowCollectionRepo> = Arc::new(PgRepo::new(db));
    let workshop = collection_repo
        .get_or_create_workshop(id, auth.user_id.0)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?;

    // Store initial_input if provided, so workshop steps can reference it
    let initial_input = body.and_then(|b| b.0.initial_input).unwrap_or_default();
    if !initial_input.is_empty() {
        let _ = crate::server::hub::dag::versioning::snapshot_content(
            &*state.repos().content_versions,
            workshop.id,
            Uuid::nil(),
            Uuid::nil(),
            "initial_input",
            "input",
            &initial_input,
        )
        .await;
    }

    Ok(Json(WorkshopResponse {
        run_id: workshop.id,
        workflow_id: id,
        status: workshop.status,
    }))
}

/// POST /api/workflows/:id/workshop/steps/:step_id/execute - Execute one step in the workshop.
#[utoipa::path(
    post,
    path = "/api/workflows/{id}/workshop/steps/{step_id}/execute",
    tag = "Workflows",
    security(("bearer_auth" = [])),
    params(
        ("id" = Uuid, Path, description = "Workflow ID"),
        ("step_id" = Uuid, Path, description = "Step ID to execute"),
    ),
    responses(
        (status = 200, description = "Step executed", body = WorkshopStepResponse),
        (status = 404, description = "Workflow or step not found"),
        (status = 409, description = "Step not ready or workshop busy")
    )
)]
pub async fn execute_workshop_step(
    State(state): State<AppState>,
    auth: auth_utils::AuthUser,
    Path(path): Path<WorkshopStepPath>,
) -> Result<Json<WorkshopStepResponse>, AppError> {
    let workflow_repo = &state.repos().workflows;

    // Verify workflow exists and user owns it
    let workflow = workflow_repo
        .get_workflow(path.id)
        .await?
        .ok_or(AppError::not_found("Workflow"))?;
    if workflow.user_id != auth.user_id.0 {
        return Err(AppError::not_found("Workflow"));
    }

    // Look up the workshop for this workflow
    let db = state
        .db()
        .ok_or(AppError::Internal("Database not available".into()))?
        .clone();
    let collection_repo: Arc<dyn WorkflowCollectionRepo> = Arc::new(PgRepo::new(db));
    let workshop = collection_repo
        .get_or_create_workshop(path.id, auth.user_id.0)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?;

    let run_id = workshop.id;

    // Concurrency guard: must be "workshop" (not running another step).
    // Auto-recover stale "workshop_running" — the spawned task always resets
    // status on completion, so a stuck status means the server crashed.
    if workshop.status == "workshop_running" {
        warn!(
            run_id = %run_id,
            "Workshop stuck in 'workshop_running' — auto-recovering stale lock"
        );
        let _ = collection_repo
            .update_workflow_execution_status(run_id, "workshop", None, None)
            .await;
    } else if workshop.status != "workshop" {
        return Err(AppError::Conflict(format!(
            "Workshop is '{}', not ready for step execution",
            workshop.status
        )));
    }

    // Load steps + edges
    let steps = workflow_repo.list_steps(path.id).await?;
    let edges = workflow_repo.list_edges(path.id).await?;

    // Find target step
    let step = steps
        .iter()
        .find(|s| s.id == path.step_id)
        .ok_or(AppError::not_found("Step"))?;

    // Pre-fetch port metadata
    let port_meta = prefetch_port_metadata(&state, &steps).await;

    // Reconstruct DagState from snapshots
    let dag_state = reconstruct_dag_state_from_snapshots(
        &*state.repos().content_versions,
        &steps,
        &port_meta,
        run_id,
    )
    .await
    .map_err(|e| AppError::Internal(e.to_string()))?;

    // Check step readiness
    use crate::server::hub::dag::check_step_readiness;
    use crate::server::hub::dag::StepReadiness;

    match check_step_readiness(
        path.step_id,
        &edges,
        &dag_state.completed,
        &dag_state.completed_envelopes,
    ) {
        StepReadiness::Ready => { /* proceed */ }
        StepReadiness::Waiting => {
            // Compute which upstream steps are missing
            let parents: Vec<Uuid> = edges
                .iter()
                .filter(|e| e.to_step_id == path.step_id)
                .filter(|e| !dag_state.completed.contains_key(&e.from_step_id))
                .map(|e| e.from_step_id)
                .collect();
            return Err(AppError::Conflict(format!(
                "Step not ready — missing upstream steps: {:?}",
                parents
            )));
        }
        StepReadiness::Skipped => {
            return Err(AppError::Conflict(
                "Step would be skipped — no matching conditional edges".to_string(),
            ));
        }
    }

    // Mark as running
    let _ = collection_repo
        .update_workflow_execution_status(run_id, "workshop_running", None, None)
        .await;

    // Build execution context
    let initial_input = String::new();
    let mut prior_outputs = HashMap::new();
    for (key, val) in &dag_state.var_outputs {
        prior_outputs.insert(key.clone(), val.clone());
    }

    let ctx = WorkflowExecutionContext {
        stage_execution_id: run_id,
        run_id,
        user_id: auth.user_id.0,
        initial_input,
        prior_outputs,
        execution_context: None,
        container_config: None,
        wg_client: None,
        snapshot: None,
        parent_context: None,
    };

    // Extract data needed for broadcast before moving into spawn
    let step_clone = step.clone();
    let step_name = step
        .output_variable_name
        .clone()
        .unwrap_or_else(|| path.step_id.to_string());
    let step_agent_id = step.agent_id;
    let workflow_id = path.id;
    let step_id = path.step_id;

    // Broadcast step started (before spawn so it's immediate)
    broadcast_workflow_event(
        &state,
        &ctx,
        workflow_id,
        WorkflowEventKind::StepStarted {
            step_id,
            step_name: step_name.clone(),
            agent_id: step_agent_id,
            execution_id: Some(run_id),
        },
    );

    // Spawn execution in a background task so it survives client disconnect.
    // Long-running steps (workforce, for_each) can take minutes; if the user
    // navigates away, the spawned task still runs to completion and snapshots
    // the result for later hydration.
    let (tx, rx) = tokio::sync::oneshot::channel();
    let bg_state = state.clone();
    let bg_ctx = ctx.clone();

    tokio::spawn(async move {
        let mut dag_state = dag_state;

        let result = execute_staged_step(
            &bg_state,
            &bg_ctx,
            &step_clone,
            &steps,
            &edges,
            &mut dag_state,
            &port_meta,
        )
        .await;

        // Always reset status back to workshop
        let _ = collection_repo
            .update_workflow_execution_status(run_id, "workshop", None, None)
            .await;

        match result {
            Ok(step_result) => {
                let next = compute_next_executable_steps(&steps, &edges, &dag_state);

                broadcast_workflow_event(
                    &bg_state,
                    &bg_ctx,
                    workflow_id,
                    WorkflowEventKind::StepCompleted {
                        step_id,
                        step_name: step_name.clone(),
                        agent_id: step_agent_id,
                        output: step_result.output.as_ref().map(|v| v.to_string()),
                        input_tokens: Some(step_result.tokens_in as u64),
                        output_tokens: Some(step_result.tokens_out as u64),
                        duration_ms: Some(step_result.duration_ms),
                    },
                );

                let _ = tx.send(Ok(WorkshopStepResponse {
                    step_id: step_result.step_id,
                    status: step_result.status,
                    output: step_result.output,
                    tokens_in: step_result.tokens_in,
                    tokens_out: step_result.tokens_out,
                    cost_usd: step_result.cost_usd,
                    duration_ms: step_result.duration_ms,
                    next_executable_steps: next,
                }));
            }
            Err(e) => {
                broadcast_workflow_event(
                    &bg_state,
                    &bg_ctx,
                    workflow_id,
                    WorkflowEventKind::StepFailed {
                        step_id,
                        step_name: step_name.clone(),
                        error: e.to_string(),
                    },
                );

                let _ = tx.send(Err(e.to_string()));
            }
        }
    });

    // Await result — if client disconnects, the spawned task still runs to
    // completion and saves the snapshot for later hydration.
    match rx.await {
        Ok(Ok(response)) => Ok(Json(response)),
        Ok(Err(e)) => Err(AppError::Internal(e)),
        Err(_) => Err(AppError::Internal(
            "workshop execution task dropped unexpectedly".into(),
        )),
    }
}

/// GET /api/workflows/:id/workshop - Get workshop status + completed steps.
#[utoipa::path(
    get,
    path = "/api/workflows/{id}/workshop",
    tag = "Workflows",
    security(("bearer_auth" = [])),
    params(
        ("id" = Uuid, Path, description = "Workflow ID"),
    ),
    responses(
        (status = 200, description = "Workshop status", body = WorkshopStatusResponse),
        (status = 404, description = "Workflow not found")
    )
)]
pub async fn get_workshop(
    State(state): State<AppState>,
    auth: auth_utils::AuthUser,
    Path(id): Path<Uuid>,
) -> Result<Json<WorkshopStatusResponse>, AppError> {
    let workflow_repo = &state.repos().workflows;

    // Verify workflow exists and user owns it
    let workflow = workflow_repo
        .get_workflow(id)
        .await?
        .ok_or(AppError::not_found("Workflow"))?;
    if workflow.user_id != auth.user_id.0 {
        return Err(AppError::not_found("Workflow"));
    }

    // Get or create the workshop
    let db = state
        .db()
        .ok_or(AppError::Internal("Database not available".into()))?
        .clone();
    let collection_repo: Arc<dyn WorkflowCollectionRepo> = Arc::new(PgRepo::new(db));
    let workshop = collection_repo
        .get_or_create_workshop(id, auth.user_id.0)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?;

    let run_id = workshop.id;

    // Auto-recover stale workshop_running on page load
    if workshop.status == "workshop_running" {
        warn!(
            run_id = %run_id,
            "Workshop stuck in 'workshop_running' on GET — auto-recovering"
        );
        let _ = collection_repo
            .update_workflow_execution_status(run_id, "workshop", None, None)
            .await;
    }

    // Load steps + edges
    let steps = workflow_repo.list_steps(id).await?;
    let edges = workflow_repo.list_edges(id).await?;

    let port_meta = prefetch_port_metadata(&state, &steps).await;

    // Reconstruct DagState from snapshots
    let dag_state = reconstruct_dag_state_from_snapshots(
        &*state.repos().content_versions,
        &steps,
        &port_meta,
        run_id,
    )
    .await
    .map_err(|e| AppError::Internal(e.to_string()))?;

    // Build completed steps list (include output from snapshots)
    let completed_steps: Vec<WorkshopStepSummary> = dag_state
        .completed
        .iter()
        .map(|(step_id, step_output)| WorkshopStepSummary {
            step_id: *step_id,
            status: "completed".to_string(),
            output: step_output.structured_output.clone(),
        })
        .collect();

    // Compute next executable steps
    let next = compute_next_executable_steps(&steps, &edges, &dag_state);

    Ok(Json(WorkshopStatusResponse {
        run_id,
        workflow_id: id,
        status: "workshop".to_string(),
        completed_steps,
        next_executable_steps: next,
    }))
}
