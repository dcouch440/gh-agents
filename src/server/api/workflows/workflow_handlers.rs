//! Workflow CRUD handlers

use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use uuid::Uuid;

use crate::constants::MAX_TITLE_LENGTH;
use crate::server::api::AppError;
use crate::server::auth as auth_utils;
use crate::server::state::AppState;

use super::types::{CreateWorkflowRequest, UpdateWorkflowRequest, WorkflowResponse};

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
    let repo = &state.repos().workflows;
    let rows = repo.list_workflows(auth.user_id.0).await?;
    let items = rows
        .into_iter()
        .map(|r| WorkflowResponse {
            id: r.id,
            name: r.name,
            description: r.description,
            created_at: r.created_at,
            container_enabled: r.container_enabled,
            target_repo_url: r.target_repo_url,
            target_branch: r.target_branch,
            vpn_enabled: r.vpn_enabled,
        })
        .collect();
    Ok(Json(items))
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
    if req.name.trim().is_empty() || req.name.len() > MAX_TITLE_LENGTH {
        return Err(AppError::bad_request(
            "Workflow name must be non-empty and within length limit",
        ));
    }
    let repo = &state.repos().workflows;
    let row = repo
        .create_workflow(
            auth.user_id.0,
            req.name,
            req.description.unwrap_or_default(),
            req.container_enabled.unwrap_or(false),
            req.target_repo_url,
            req.target_branch,
            req.vpn_enabled.unwrap_or(false),
        )
        .await?;
    Ok((
        StatusCode::CREATED,
        Json(WorkflowResponse {
            id: row.id,
            name: row.name,
            description: row.description,
            created_at: row.created_at,
            container_enabled: row.container_enabled,
            target_repo_url: row.target_repo_url,
            target_branch: row.target_branch,
            vpn_enabled: row.vpn_enabled,
        }),
    ))
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
    let repo = &state.repos().workflows;
    let row = repo
        .get_workflow(id)
        .await?
        .ok_or(AppError::not_found("Workflow"))?;
    if row.user_id != auth.user_id.0 {
        return Err(AppError::not_found("Workflow"));
    }
    Ok(Json(WorkflowResponse {
        id: row.id,
        name: row.name,
        description: row.description,
        created_at: row.created_at,
        container_enabled: row.container_enabled,
        target_repo_url: row.target_repo_url,
        target_branch: row.target_branch,
        vpn_enabled: row.vpn_enabled,
    }))
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
    let repo = &state.repos().workflows;
    let existing = repo
        .get_workflow(id)
        .await?
        .ok_or(AppError::not_found("Workflow"))?;
    if existing.user_id != auth.user_id.0 {
        return Err(AppError::not_found("Workflow"));
    }
    if let Some(ref name) = req.name {
        if name.trim().is_empty() || name.len() > MAX_TITLE_LENGTH {
            return Err(AppError::bad_request(
                "Workflow name must be non-empty and within length limit",
            ));
        }
    }
    let row = repo
        .update_workflow(
            id,
            req.name,
            req.description,
            req.container_enabled,
            req.target_repo_url,
            req.target_branch,
            req.vpn_enabled,
        )
        .await?;
    Ok(Json(WorkflowResponse {
        id: row.id,
        name: row.name,
        description: row.description,
        created_at: row.created_at,
        container_enabled: row.container_enabled,
        target_repo_url: row.target_repo_url,
        target_branch: row.target_branch,
        vpn_enabled: row.vpn_enabled,
    }))
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
    let repo = &state.repos().workflows;
    let existing = repo
        .get_workflow(id)
        .await?
        .ok_or(AppError::not_found("Workflow"))?;
    if existing.user_id != auth.user_id.0 {
        return Err(AppError::not_found("Workflow"));
    }
    repo.delete_workflow(id).await?;
    Ok(StatusCode::NO_CONTENT)
}
