//! Run template CRUD handlers — promote, list, get, delete.

use axum::{
    extract::{Path, State},
    Json,
};
use uuid::Uuid;

use crate::server::api::AppError;
use crate::server::auth as auth_utils;
use crate::server::hub::dag::templates::capture_workflow_snapshot;
use crate::server::state::AppState;

use super::types::{
    CreateTemplateRequest, RunTemplateDetailResponse, RunTemplateResponse, TemplatePath,
};

/// POST /api/workflows/:id/templates - Promote current workflow state to a frozen run template.
#[utoipa::path(
    post,
    path = "/api/workflows/{id}/templates",
    tag = "Workflows",
    security(("bearer_auth" = [])),
    params(("id" = Uuid, Path, description = "Workflow ID")),
    request_body(content = CreateTemplateRequest, content_type = "application/json"),
    responses(
        (status = 201, description = "Template created", body = RunTemplateResponse),
        (status = 404, description = "Workflow not found")
    )
)]
pub async fn create_template(
    State(state): State<AppState>,
    auth: auth_utils::AuthUser,
    Path(id): Path<Uuid>,
    Json(body): Json<CreateTemplateRequest>,
) -> Result<Json<RunTemplateResponse>, AppError> {
    let workflow_repo = &state.repos().workflows;

    // Verify workflow exists and user owns it
    let workflow = workflow_repo
        .get_workflow(id)
        .await?
        .ok_or(AppError::not_found("Workflow"))?;
    if workflow.user_id != auth.user_id.0 {
        return Err(AppError::not_found("Workflow"));
    }

    // Capture the complete workflow snapshot
    let snapshot = capture_workflow_snapshot(&state, id)
        .await
        .map_err(|e| AppError::Internal(format!("Failed to capture snapshot: {e}")))?;

    let snapshot_json = serde_json::to_value(&snapshot)
        .map_err(|e| AppError::Internal(format!("Failed to serialize snapshot: {e}")))?;

    // Store the template
    let template = workflow_repo
        .create_template(
            id,
            auth.user_id.0,
            &body.name,
            body.description.clone(),
            snapshot_json,
        )
        .await?;

    Ok(Json(RunTemplateResponse {
        id: template.id,
        workflow_id: template.workflow_id,
        name: template.name,
        description: template.description,
        created_at: template.created_at,
    }))
}

/// GET /api/workflows/:id/templates - List all run templates for a workflow.
#[utoipa::path(
    get,
    path = "/api/workflows/{id}/templates",
    tag = "Workflows",
    security(("bearer_auth" = [])),
    params(("id" = Uuid, Path, description = "Workflow ID")),
    responses(
        (status = 200, description = "Templates listed", body = Vec<RunTemplateResponse>),
        (status = 404, description = "Workflow not found")
    )
)]
pub async fn list_templates(
    State(state): State<AppState>,
    auth: auth_utils::AuthUser,
    Path(id): Path<Uuid>,
) -> Result<Json<Vec<RunTemplateResponse>>, AppError> {
    let workflow_repo = &state.repos().workflows;

    // Verify workflow exists and user owns it
    let workflow = workflow_repo
        .get_workflow(id)
        .await?
        .ok_or(AppError::not_found("Workflow"))?;
    if workflow.user_id != auth.user_id.0 {
        return Err(AppError::not_found("Workflow"));
    }

    let templates = workflow_repo.list_templates(id).await?;

    let response: Vec<RunTemplateResponse> = templates
        .into_iter()
        .map(|t| RunTemplateResponse {
            id: t.id,
            workflow_id: t.workflow_id,
            name: t.name,
            description: t.description,
            created_at: t.created_at,
        })
        .collect();

    Ok(Json(response))
}

/// GET /api/workflows/:id/templates/:template_id - Get a specific run template with snapshot.
#[utoipa::path(
    get,
    path = "/api/workflows/{id}/templates/{template_id}",
    tag = "Workflows",
    security(("bearer_auth" = [])),
    params(
        ("id" = Uuid, Path, description = "Workflow ID"),
        ("template_id" = Uuid, Path, description = "Template ID"),
    ),
    responses(
        (status = 200, description = "Template details", body = RunTemplateDetailResponse),
        (status = 404, description = "Template not found")
    )
)]
pub async fn get_template(
    State(state): State<AppState>,
    auth: auth_utils::AuthUser,
    Path(path): Path<TemplatePath>,
) -> Result<Json<RunTemplateDetailResponse>, AppError> {
    let workflow_repo = &state.repos().workflows;

    // Verify workflow exists and user owns it
    let workflow = workflow_repo
        .get_workflow(path.id)
        .await?
        .ok_or(AppError::not_found("Workflow"))?;
    if workflow.user_id != auth.user_id.0 {
        return Err(AppError::not_found("Workflow"));
    }

    let template = workflow_repo
        .get_template(path.template_id)
        .await?
        .ok_or(AppError::not_found("Template"))?;

    // Verify template belongs to this workflow
    if template.workflow_id != path.id {
        return Err(AppError::not_found("Template"));
    }

    Ok(Json(RunTemplateDetailResponse {
        id: template.id,
        workflow_id: template.workflow_id,
        name: template.name,
        description: template.description,
        snapshot: template.snapshot,
        created_at: template.created_at,
    }))
}

/// DELETE /api/workflows/:id/templates/:template_id - Delete a run template.
#[utoipa::path(
    delete,
    path = "/api/workflows/{id}/templates/{template_id}",
    tag = "Workflows",
    security(("bearer_auth" = [])),
    params(
        ("id" = Uuid, Path, description = "Workflow ID"),
        ("template_id" = Uuid, Path, description = "Template ID"),
    ),
    responses(
        (status = 204, description = "Template deleted"),
        (status = 404, description = "Template not found")
    )
)]
pub async fn delete_template(
    State(state): State<AppState>,
    auth: auth_utils::AuthUser,
    Path(path): Path<TemplatePath>,
) -> Result<(), AppError> {
    let workflow_repo = &state.repos().workflows;

    // Verify workflow exists and user owns it
    let workflow = workflow_repo
        .get_workflow(path.id)
        .await?
        .ok_or(AppError::not_found("Workflow"))?;
    if workflow.user_id != auth.user_id.0 {
        return Err(AppError::not_found("Workflow"));
    }

    // Verify template exists and belongs to this workflow
    let template = workflow_repo
        .get_template(path.template_id)
        .await?
        .ok_or(AppError::not_found("Template"))?;
    if template.workflow_id != path.id {
        return Err(AppError::not_found("Template"));
    }

    workflow_repo.delete_template(path.template_id).await?;

    Ok(())
}
