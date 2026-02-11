//! Step document attachment handlers

use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};

use crate::server::api::AppError;
use crate::server::auth as auth_utils;
use crate::server::state::AppState;

use super::types::{StepDocumentRequest, StepDocumentResponse, WorkflowStepPath};

/// POST /api/workflows/:wid/steps/:sid/documents
#[utoipa::path(
    post,
    path = "/api/workflows/{wid}/steps/{sid}/documents",
    tag = "Step Documents",
    security(("bearer_auth" = [])),
    params(
        ("wid" = Uuid, Path, description = "Workflow ID"),
        ("sid" = Uuid, Path, description = "Step ID")
    ),
    request_body = StepDocumentRequest,
    responses(
        (status = 201, description = "Document added to step"),
        (status = 404, description = "Not found")
    )
)]
pub async fn add_step_document(
    State(state): State<AppState>,
    auth: auth_utils::AuthUser,
    Path(p): Path<WorkflowStepPath>,
    Json(req): Json<StepDocumentRequest>,
) -> Result<StatusCode, AppError> {
    let repo = &state.repos().workflows;
    let wf = repo
        .get_workflow(p.wid)
        .await?
        .ok_or(AppError::not_found("Workflow"))?;
    if wf.user_id != auth.user_id.0 {
        return Err(AppError::not_found("Workflow"));
    }
    let existing = repo
        .get_step(p.sid)
        .await?
        .ok_or(AppError::not_found("Step"))?;
    if existing.workflow_id != p.wid {
        return Err(AppError::not_found("Step"));
    }
    repo.add_step_document(p.sid, req.document_id).await?;
    Ok(StatusCode::CREATED)
}

/// DELETE /api/workflows/:wid/steps/:sid/documents
#[utoipa::path(
    delete,
    path = "/api/workflows/{wid}/steps/{sid}/documents",
    tag = "Step Documents",
    security(("bearer_auth" = [])),
    params(
        ("wid" = Uuid, Path, description = "Workflow ID"),
        ("sid" = Uuid, Path, description = "Step ID")
    ),
    request_body = StepDocumentRequest,
    responses(
        (status = 204, description = "Document removed from step"),
        (status = 404, description = "Not found")
    )
)]
pub async fn remove_step_document(
    State(state): State<AppState>,
    auth: auth_utils::AuthUser,
    Path(p): Path<WorkflowStepPath>,
    Json(req): Json<StepDocumentRequest>,
) -> Result<StatusCode, AppError> {
    let repo = &state.repos().workflows;
    let wf = repo
        .get_workflow(p.wid)
        .await?
        .ok_or(AppError::not_found("Workflow"))?;
    if wf.user_id != auth.user_id.0 {
        return Err(AppError::not_found("Workflow"));
    }
    let existing = repo
        .get_step(p.sid)
        .await?
        .ok_or(AppError::not_found("Step"))?;
    if existing.workflow_id != p.wid {
        return Err(AppError::not_found("Step"));
    }
    repo.remove_step_document(p.sid, req.document_id).await?;
    Ok(StatusCode::NO_CONTENT)
}

/// GET /api/workflows/:wid/steps/:sid/documents
#[utoipa::path(
    get,
    path = "/api/workflows/{wid}/steps/{sid}/documents",
    tag = "Step Documents",
    security(("bearer_auth" = [])),
    params(
        ("wid" = Uuid, Path, description = "Workflow ID"),
        ("sid" = Uuid, Path, description = "Step ID")
    ),
    responses(
        (status = 200, description = "List of step documents", body = Vec<StepDocumentResponse>),
        (status = 404, description = "Not found")
    )
)]
pub async fn list_step_documents(
    State(state): State<AppState>,
    auth: auth_utils::AuthUser,
    Path(p): Path<WorkflowStepPath>,
) -> Result<Json<Vec<StepDocumentResponse>>, AppError> {
    let repo = &state.repos().workflows;
    let wf = repo
        .get_workflow(p.wid)
        .await?
        .ok_or(AppError::not_found("Workflow"))?;
    if wf.user_id != auth.user_id.0 {
        return Err(AppError::not_found("Workflow"));
    }
    let rows = repo.list_step_documents(p.sid).await?;
    Ok(Json(
        rows.into_iter()
            .map(|r| StepDocumentResponse {
                step_id: r.step_id,
                document_id: r.document_id,
            })
            .collect(),
    ))
}
