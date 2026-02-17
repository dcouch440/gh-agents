//! Workflow CRUD handlers

use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use uuid::Uuid;

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
        auth.user_id.0,
        req.name,
        req.description,
        req.container_enabled,
        req.target_repo_url,
        req.target_branch,
        req.vpn_enabled,
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
        req.name,
        req.description,
        req.container_enabled,
        req.target_repo_url,
        req.target_branch,
        req.vpn_enabled,
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
