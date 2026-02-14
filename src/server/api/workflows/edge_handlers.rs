//! Workflow edge handlers

use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use uuid::Uuid;

use crate::server::api::AppError;
use crate::server::auth as auth_utils;
use crate::server::state::AppState;

use super::types::{EdgeRequest, EdgeResponse};

/// GET /api/workflows/:id/edges
#[utoipa::path(
    get,
    path = "/api/workflows/{id}/edges",
    tag = "Workflow Edges",
    security(("bearer_auth" = [])),
    params(("id" = Uuid, Path, description = "Workflow ID")),
    responses(
        (status = 200, description = "List of workflow edges", body = Vec<EdgeResponse>),
        (status = 404, description = "Not found")
    )
)]
pub async fn list_workflow_edges(
    State(state): State<AppState>,
    auth: auth_utils::AuthUser,
    Path(wid): Path<Uuid>,
) -> Result<Json<Vec<EdgeResponse>>, AppError> {
    let repo = &state.repos().workflows;
    let wf = repo
        .get_workflow(wid)
        .await?
        .ok_or(AppError::not_found("Workflow"))?;
    if wf.user_id != auth.user_id.0 {
        return Err(AppError::not_found("Workflow"));
    }
    let rows = repo.list_edges(wid).await?;
    Ok(Json(
        rows.into_iter()
            .map(|e| EdgeResponse {
                id: e.id,
                from_step_id: e.from_step_id,
                to_step_id: e.to_step_id,
            })
            .collect(),
    ))
}

/// POST /api/workflows/:id/edges
#[utoipa::path(
    post,
    path = "/api/workflows/{id}/edges",
    tag = "Workflow Edges",
    security(("bearer_auth" = [])),
    params(("id" = Uuid, Path, description = "Workflow ID")),
    request_body = EdgeRequest,
    responses(
        (status = 201, description = "Edge added", body = EdgeResponse),
        (status = 404, description = "Not found")
    )
)]
pub async fn add_workflow_edge(
    State(state): State<AppState>,
    auth: auth_utils::AuthUser,
    Path(wid): Path<Uuid>,
    Json(req): Json<EdgeRequest>,
) -> Result<(StatusCode, Json<EdgeResponse>), AppError> {
    let repo = &state.repos().workflows;
    let wf = repo
        .get_workflow(wid)
        .await?
        .ok_or(AppError::not_found("Workflow"))?;
    if wf.user_id != auth.user_id.0 {
        return Err(AppError::not_found("Workflow"));
    }
    // Context nodes are source-only (cannot be edge targets)
    let to_step = repo
        .get_step(req.to_step_id)
        .await?
        .ok_or(AppError::not_found("Target step"))?;
    if to_step.execution_mode == "context" {
        return Err(AppError::bad_request(
            "Context nodes cannot receive incoming edges",
        ));
    }

    let edge = repo.add_edge(wid, req.from_step_id, req.to_step_id).await?;
    repo.mark_board_context_stale(wid).await?;
    Ok((
        StatusCode::CREATED,
        Json(EdgeResponse {
            id: edge.id,
            from_step_id: edge.from_step_id,
            to_step_id: edge.to_step_id,
        }),
    ))
}

/// DELETE /api/workflows/:id/edges
#[utoipa::path(
    delete,
    path = "/api/workflows/{id}/edges",
    tag = "Workflow Edges",
    security(("bearer_auth" = [])),
    params(("id" = Uuid, Path, description = "Workflow ID")),
    request_body = EdgeRequest,
    responses(
        (status = 204, description = "Edge removed"),
        (status = 404, description = "Not found")
    )
)]
pub async fn remove_workflow_edge(
    State(state): State<AppState>,
    auth: auth_utils::AuthUser,
    Path(wid): Path<Uuid>,
    Json(req): Json<EdgeRequest>,
) -> Result<StatusCode, AppError> {
    let repo = &state.repos().workflows;
    let wf = repo
        .get_workflow(wid)
        .await?
        .ok_or(AppError::not_found("Workflow"))?;
    if wf.user_id != auth.user_id.0 {
        return Err(AppError::not_found("Workflow"));
    }
    repo.remove_edge(req.from_step_id, req.to_step_id).await?;
    repo.mark_board_context_stale(wid).await?;
    Ok(StatusCode::NO_CONTENT)
}

/// DELETE /api/workflows/:wid/edges/:eid
#[utoipa::path(
    delete,
    path = "/api/workflows/{wid}/edges/{eid}",
    tag = "Workflow Edges",
    security(("bearer_auth" = [])),
    params(
        ("wid" = Uuid, Path, description = "Workflow ID"),
        ("eid" = Uuid, Path, description = "Edge ID")
    ),
    responses(
        (status = 204, description = "Edge removed"),
        (status = 404, description = "Not found")
    )
)]
pub async fn delete_workflow_edge_by_id(
    State(state): State<AppState>,
    auth: auth_utils::AuthUser,
    Path((wid, eid)): Path<(Uuid, Uuid)>,
) -> Result<StatusCode, AppError> {
    let repo = &state.repos().workflows;
    let wf = repo
        .get_workflow(wid)
        .await?
        .ok_or(AppError::not_found("Workflow"))?;
    if wf.user_id != auth.user_id.0 {
        return Err(AppError::not_found("Workflow"));
    }
    repo.delete_edge_by_id(eid).await?;
    repo.mark_board_context_stale(wid).await?;
    Ok(StatusCode::NO_CONTENT)
}
