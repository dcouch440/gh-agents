//! Workflow execution (run) handler

use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use std::collections::HashMap;
use std::sync::Arc;
use uuid::Uuid;

use crate::db::pg_repo::PgRepo;
use crate::db::traits::WorkflowCollectionRepo;
use crate::server::api::AppError;
use crate::server::auth as auth_utils;
use crate::server::hub::dag::templates::WorkflowSnapshot;
use crate::server::hub::dag::{
    broadcast_workflow_event, execute_workflow_via_engine, WorkflowExecutionContext,
};
use crate::server::hub::ExecutionEngine;
use crate::server::state::AppState;
use crate::server::ws::events::WorkflowEventKind;

use super::types::{RunWorkflowRequest, WorkflowRunResponse};

/// POST /api/workflows/:id/run - Execute a workflow directly (without a collection).
#[utoipa::path(
    post,
    path = "/api/workflows/{id}/run",
    tag = "Workflows",
    security(("bearer_auth" = [])),
    params(("id" = Uuid, Path, description = "Workflow ID")),
    request_body(content = Option<RunWorkflowRequest>, content_type = "application/json"),
    responses(
        (status = 202, description = "Workflow execution started", body = WorkflowRunResponse),
        (status = 404, description = "Workflow not found")
    )
)]
pub async fn run_workflow(
    State(state): State<AppState>,
    auth: auth_utils::AuthUser,
    Path(id): Path<Uuid>,
    body: Option<Json<RunWorkflowRequest>>,
) -> Result<(StatusCode, Json<WorkflowRunResponse>), AppError> {
    let workflow_repo = &state.repos().workflows;

    // Verify workflow exists and user owns it
    let workflow = workflow_repo
        .get_workflow(id)
        .await?
        .ok_or(AppError::not_found("Workflow"))?;
    if workflow.user_id != auth.user_id.0 {
        return Err(AppError::not_found("Workflow"));
    }

    // Create standalone workflow execution row
    let db = state
        .db()
        .ok_or(AppError::Internal("Database not available".into()))?
        .clone();
    let collection_repo: Arc<dyn WorkflowCollectionRepo> = Arc::new(PgRepo::new(db));
    let execution = collection_repo
        .create_standalone_workflow_execution(id, auth.user_id.0)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?;

    let execution_id = execution.id;

    // Extract template_id and initial_input from body before consuming
    let (body_input, template_id) = match body {
        Some(Json(req)) => (req.initial_input, req.template_id),
        None => (None, None),
    };

    // If template_id is provided, load and deserialize the frozen snapshot
    let snapshot: Option<Arc<WorkflowSnapshot>> = match template_id {
        Some(tid) => {
            let template = workflow_repo
                .get_template(tid)
                .await?
                .ok_or(AppError::not_found("Template"))?;
            if template.workflow_id != id {
                return Err(AppError::not_found("Template"));
            }
            let ws: WorkflowSnapshot = serde_json::from_value(template.snapshot)
                .map_err(|e| AppError::Internal(format!("Invalid template snapshot: {e}")))?;
            Some(Arc::new(ws))
        }
        None => None,
    };

    // Load steps + edges from snapshot or live DB
    let (steps, edges) = match &snapshot {
        Some(snap) => (snap.steps.clone(), snap.edges.clone()),
        None => {
            let s = workflow_repo.list_steps(id).await?;
            let e = workflow_repo.list_edges(id).await?;
            (s, e)
        }
    };

    // Filter out hidden steps (e.g. manager dispatch anchor)
    let steps: Vec<_> = steps.into_iter().filter(|s| s.visible).collect();

    if steps.is_empty() {
        return Err(AppError::bad_request("Workflow has no steps"));
    }

    // Build execution engine
    let provider = state
        .provider()
        .ok_or(AppError::Internal("LLM provider not configured".into()))?
        .clone();
    let engine = ExecutionEngine::new(provider, state.env().debug_stream);

    // Resolve initial_input: prefer POST body, fall back to first context step's prompt_template
    let initial_input = body_input.unwrap_or_else(|| {
        steps
            .iter()
            .find(|s| s.execution_mode == "context" || s.execution_mode == "input")
            .map(|s| s.prompt_template.clone())
            .unwrap_or_default()
    });

    let mut prior_outputs = HashMap::new();
    if !initial_input.is_empty() {
        prior_outputs.insert(
            "input".to_string(),
            serde_json::Value::String(initial_input.clone()),
        );
    }

    // Build container config — every workforce run gets a container.
    // If the workflow has a repo URL, agents get a git clone. Otherwise,
    // agents get an empty workspace (JuiceFS mount if available).
    let wf_row = workflow_repo
        .get_workflow(id)
        .await?
        .ok_or(AppError::not_found("Workflow"))?;
    let github_token =
        crate::execution::RedactedString::new(state.env().github_token.clone().unwrap_or_default());
    let container_config = Some(crate::server::hub::dag::ContainerExecutionConfig {
        clone_url: wf_row.target_repo_url.clone().unwrap_or_default(),
        branch: wf_row.target_branch.clone(),
        github_token,
        image: None,
        memory_limit: None,
        cpu_limit: None,
        vpn_enabled: wf_row.vpn_enabled,
        workflow_id: Some(id),
        run_id: Some(execution_id),
        overlay_enabled: state.workspace().is_some(),
    });

    let wg_client = if container_config.as_ref().is_some_and(|c| c.vpn_enabled) {
        crate::execution::WgEasyConfig::from_env()
            .map(|cfg| std::sync::Arc::new(crate::execution::WgEasyClient::new(cfg)))
    } else {
        None
    };

    let ctx = WorkflowExecutionContext {
        stage_execution_id: execution_id,
        run_id: execution_id,
        user_id: auth.user_id.0,
        initial_input,
        prior_outputs,
        execution_context: None,
        container_config,
        wg_client,
        snapshot,
    };

    // Spawn execution in background (non-blocking, return 202)
    let bg_state = state.clone();
    let bg_collection_repo = collection_repo.clone();
    tokio::spawn(async move {
        // Mark as running
        let _ = bg_collection_repo
            .update_workflow_execution_status(execution_id, "running", None, None)
            .await;

        match execute_workflow_via_engine(&engine, &bg_state, &ctx, &steps, &edges, None).await {
            Ok(result) => {
                // Aggregate outputs
                let mut aggregated = serde_json::Map::new();
                for output in result.outputs.values() {
                    if let Some(structured) = &output.structured_output {
                        aggregated.insert(output.variable_name.clone(), structured.clone());
                    } else if !output.raw_output.is_empty() {
                        aggregated.insert(
                            output.variable_name.clone(),
                            serde_json::Value::String(output.raw_output.clone()),
                        );
                    }
                }
                let outputs_json = serde_json::Value::Object(aggregated);

                let _ = bg_collection_repo
                    .update_workflow_execution_status(
                        execution_id,
                        "completed",
                        Some(outputs_json),
                        None,
                    )
                    .await;
                broadcast_workflow_event(
                    &bg_state,
                    &ctx,
                    id,
                    WorkflowEventKind::Completed {
                        duration_ms: Some(result.duration_ms),
                    },
                );
            }
            Err(e) => {
                let error_msg = e.to_string();
                let _ = bg_collection_repo
                    .update_workflow_execution_status(
                        execution_id,
                        "failed",
                        None,
                        Some(error_msg.clone()),
                    )
                    .await;
                broadcast_workflow_event(
                    &bg_state,
                    &ctx,
                    id,
                    WorkflowEventKind::Failed { error: error_msg },
                );
            }
        }
    });

    Ok((
        StatusCode::ACCEPTED,
        Json(WorkflowRunResponse {
            execution_id,
            workflow_id: id,
            status: "pending".to_string(),
        }),
    ))
}
