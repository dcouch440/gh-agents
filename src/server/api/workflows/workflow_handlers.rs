//! Workflow CRUD handlers

use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use uuid::Uuid;

use crate::db::traits::{CreateWorkflowInput, UpdateWorkflowInput};
use crate::server::api::AppError;
use crate::server::auth as auth_utils;
use crate::server::services::workflows;
use crate::server::state::AppState;

use super::types::{CreateWorkflowRequest, UpdateWorkflowRequest, WorkflowResponse};

fn workflow_response(row: crate::db::WorkflowRow) -> WorkflowResponse {
    WorkflowResponse {
        id: row.id,
        name: row.name,
        description: row.description,
        created_at: row.created_at,
        container_enabled: row.container_enabled,
        target_repo_url: row.target_repo_url,
        target_branch: row.target_branch,
        vpn_enabled: row.vpn_enabled,
    }
}

/// GET /api/workflows
#[utoipa::path(
    get,
    path = "/api/workflows",
    tag = "Workflows",
    security(("bearer_auth" = [])),
    responses(
        (status = 200, description = "List of workflows", body = Vec<WorkflowResponse>)
    )
)]
pub async fn list_workflows(
    State(state): State<AppState>,
    auth: auth_utils::AuthUser,
) -> Result<Json<Vec<WorkflowResponse>>, AppError> {
    let rows = workflows::list_workflows(state.repos().workflows.as_ref(), auth.user_id.0).await?;
    Ok(Json(rows.into_iter().map(workflow_response).collect()))
}

/// POST /api/workflows
#[utoipa::path(
    post,
    path = "/api/workflows",
    tag = "Workflows",
    security(("bearer_auth" = [])),
    request_body = CreateWorkflowRequest,
    responses(
        (status = 201, description = "Workflow created", body = WorkflowResponse),
        (status = 400, description = "Invalid request")
    )
)]
pub async fn create_workflow(
    State(state): State<AppState>,
    auth: auth_utils::AuthUser,
    Json(req): Json<CreateWorkflowRequest>,
) -> Result<(StatusCode, Json<WorkflowResponse>), AppError> {
    let row = workflows::create_workflow(
        state.repos().workflows.as_ref(),
        CreateWorkflowInput {
            user_id: auth.user_id.0,
            name: req.name,
            description: req.description.unwrap_or_default(),
            container_enabled: req.container_enabled.unwrap_or(false),
            target_repo_url: req.target_repo_url,
            target_branch: req.target_branch,
            vpn_enabled: req.vpn_enabled.unwrap_or(false),
        },
    )
    .await?;
    Ok((StatusCode::CREATED, Json(workflow_response(row))))
}

/// GET /api/workflows/:id
#[utoipa::path(
    get,
    path = "/api/workflows/{id}",
    tag = "Workflows",
    security(("bearer_auth" = [])),
    params(("id" = Uuid, Path, description = "Workflow ID")),
    responses(
        (status = 200, description = "Workflow found", body = WorkflowResponse),
        (status = 404, description = "Not found")
    )
)]
pub async fn get_workflow(
    State(state): State<AppState>,
    auth: auth_utils::AuthUser,
    Path(id): Path<Uuid>,
) -> Result<Json<WorkflowResponse>, AppError> {
    let row = workflows::get_workflow(state.repos().workflows.as_ref(), auth.user_id.0, id).await?;
    Ok(Json(workflow_response(row)))
}

/// PUT /api/workflows/:id
#[utoipa::path(
    put,
    path = "/api/workflows/{id}",
    tag = "Workflows",
    security(("bearer_auth" = [])),
    params(("id" = Uuid, Path, description = "Workflow ID")),
    request_body = UpdateWorkflowRequest,
    responses(
        (status = 200, description = "Updated workflow", body = WorkflowResponse),
        (status = 404, description = "Not found")
    )
)]
pub async fn update_workflow(
    State(state): State<AppState>,
    auth: auth_utils::AuthUser,
    Path(id): Path<Uuid>,
    Json(req): Json<UpdateWorkflowRequest>,
) -> Result<Json<WorkflowResponse>, AppError> {
    let row = workflows::update_workflow(
        state.repos().workflows.as_ref(),
        auth.user_id.0,
        id,
        UpdateWorkflowInput {
            id,
            name: req.name,
            description: req.description,
            container_enabled: req.container_enabled,
            target_repo_url: req.target_repo_url,
            target_branch: req.target_branch,
            vpn_enabled: req.vpn_enabled,
        },
    )
    .await?;
    Ok(Json(workflow_response(row)))
}

/// DELETE /api/workflows/:id
#[utoipa::path(
    delete,
    path = "/api/workflows/{id}",
    tag = "Workflows",
    security(("bearer_auth" = [])),
    params(("id" = Uuid, Path, description = "Workflow ID")),
    responses(
        (status = 204, description = "Deleted successfully"),
        (status = 404, description = "Not found")
    )
)]
pub async fn delete_workflow(
    State(state): State<AppState>,
    auth: auth_utils::AuthUser,
    Path(id): Path<Uuid>,
) -> Result<StatusCode, AppError> {
    workflows::delete_workflow(state.repos().workflows.as_ref(), auth.user_id.0, id).await?;
    Ok(StatusCode::NO_CONTENT)
}

/// GET /api/workflows/:id/agent-session
///
/// Get or create the workflow agent's persistent chat session.
/// Returns the session with `draft_config.role = "workflow_agent"`.
pub async fn get_or_create_workflow_agent_session(
    State(state): State<AppState>,
    auth: auth_utils::AuthUser,
    Path(workflow_id): Path<Uuid>,
) -> Result<Json<super::types::WorkflowAgentSessionResponse>, AppError> {
    // Verify workflow ownership
    let wf = state
        .repos()
        .workflows
        .get_workflow(workflow_id)
        .await?
        .ok_or(AppError::not_found("Workflow"))?;
    if wf.user_id != auth.user_id.0 {
        return Err(AppError::not_found("Workflow"));
    }

    // Try to find existing workflow agent session
    if let Some(session) = state
        .repos()
        .sessions
        .find_workflow_agent_session(workflow_id)
        .await?
    {
        return Ok(Json(super::types::WorkflowAgentSessionResponse {
            session_id: session.id,
            workflow_id,
            title: session.title,
            created_at: session.created_at,
        }));
    }

    // Create new session
    let session_id = Uuid::new_v4();
    let title = format!("{} — Workflow Agent", wf.name);
    let draft_config = serde_json::json!({
        "role": "workflow_agent",
        "workflow_id": workflow_id.to_string(),
    });

    state
        .repos()
        .sessions
        .create_session(
            auth.user_id,
            session_id,
            "workflow_agent",
            &title,
            None,
            Some(draft_config),
        )
        .await?;

    let session = state
        .repos()
        .sessions
        .get_session(session_id)
        .await?
        .ok_or(AppError::Internal(
            "Session not found after creation".into(),
        ))?;

    use crate::server::ws::events::{SessionEvent, SessionEventKind};
    state.broadcast_session(SessionEvent {
        session_id,
        user_id: Some(auth.user_id.0),
        kind: SessionEventKind::Created {
            title: session.title.clone(),
            mode_id: session.mode_id.clone(),
        },
    });

    Ok(Json(super::types::WorkflowAgentSessionResponse {
        session_id: session.id,
        workflow_id,
        title: session.title,
        created_at: session.created_at,
    }))
}

/// POST /workflows/:id/generate — trigger system node agents for described nodes.
///
/// Builds dispatch instructions for all workforce steps that have descriptions
/// but haven't been configured yet (or whose descriptions changed), then spawns
/// the sequential design pipeline as a background task.
pub async fn generate_workflow(
    State(state): State<AppState>,
    auth: auth_utils::AuthUser,
    Path(workflow_id): Path<Uuid>,
) -> Result<Json<super::types::GenerateResponse>, AppError> {
    use crate::server::services::board::instruction::{NodeChangeType, NodeDispatchInstruction};

    let wf = state
        .repos()
        .workflows
        .get_workflow(workflow_id)
        .await?
        .ok_or(AppError::not_found("Workflow"))?;
    if wf.user_id != auth.user_id.0 {
        return Err(AppError::not_found("Workflow"));
    }

    let steps = state
        .repos()
        .workflows
        .list_steps(workflow_id)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?;
    let edges = state
        .repos()
        .workflows
        .list_edges(workflow_id)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?;

    // Build instructions for workforce steps that need generation
    let instructions: Vec<NodeDispatchInstruction> = steps
        .iter()
        .filter(|s| s.execution_mode == "workforce" && !s.description.is_empty())
        .map(|s| NodeDispatchInstruction {
            element_id: s.id.to_string(),
            step_id: s.id,
            execution_mode: s.execution_mode.clone(),
            instruction: s.description.clone(),
            change_type: if s.child_workflow_id.is_some() {
                NodeChangeType::Updated
            } else {
                NodeChangeType::New
            },
        })
        .collect();

    let generating = instructions.len();

    if !instructions.is_empty() {
        let state_clone = state.clone();
        let user_id = auth.user_id;
        tokio::spawn(async move {
            crate::server::services::dispatch::sequential::run_sequential_design_pipeline(
                state_clone,
                workflow_id,
                user_id,
                instructions,
                steps,
                edges,
            )
            .await;
        });
    }

    Ok(Json(super::types::GenerateResponse { generating }))
}
