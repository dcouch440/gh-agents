//! Workflow edge handlers

use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use uuid::Uuid;

use crate::server::api::AppError;
use crate::server::auth as auth_utils;
use crate::server::services::edges;
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
    let rows = edges::list_edges(state.repos().workflows.as_ref(), auth.user_id.0, wid).await?;
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
    let edge = edges::add_edge(
        state.repos().workflows.as_ref(),
        auth.user_id.0,
        wid,
        req.from_step_id,
        req.to_step_id,
    )
    .await?;

    state.broadcast_workflow(crate::server::ws::events::WorkflowEvent {
        run_id: None,
        workflow_id: wid,
        user_id: Some(auth.user_id.0),
        kind: crate::server::ws::events::WorkflowEventKind::EdgeCreated {
            edge_id: edge.id,
            from_step_id: edge.from_step_id,
            to_step_id: edge.to_step_id,
        },
    });

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
    let edge = edges::remove_edge(
        state.repos().workflows.as_ref(),
        auth.user_id.0,
        wid,
        req.from_step_id,
        req.to_step_id,
    )
    .await?;

    state.broadcast_workflow(crate::server::ws::events::WorkflowEvent {
        run_id: None,
        workflow_id: wid,
        user_id: Some(auth.user_id.0),
        kind: crate::server::ws::events::WorkflowEventKind::EdgeDeleted {
            edge_id: edge.id,
            from_step_id: edge.from_step_id,
            to_step_id: edge.to_step_id,
        },
    });

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
    let edge =
        edges::delete_edge_by_id(state.repos().workflows.as_ref(), auth.user_id.0, wid, eid)
            .await?;

    state.broadcast_workflow(crate::server::ws::events::WorkflowEvent {
        run_id: None,
        workflow_id: wid,
        user_id: Some(auth.user_id.0),
        kind: crate::server::ws::events::WorkflowEventKind::EdgeDeleted {
            edge_id: edge.id,
            from_step_id: edge.from_step_id,
            to_step_id: edge.to_step_id,
        },
    });

    Ok(StatusCode::NO_CONTENT)
}
