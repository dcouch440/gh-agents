//! Session context store and router request endpoints

use axum::{
    extract::{Path, State},
    Json,
};
use uuid::Uuid;

use super::AppError;
use crate::server::auth as auth_utils;
use crate::server::state::AppState;

/// GET /api/sessions/:session_id/context - Get context entries for a session.
#[utoipa::path(
    get,
    path = "/api/sessions/{session_id}/context",
    tag = "Session Context",
    security(("bearer_auth" = [])),
    params(("session_id" = Uuid, Path, description = "Session ID")),
    responses(
        (status = 200, description = "Session context entries")
    )
)]
pub async fn get_session_context(
    State(state): State<AppState>,
    _auth: auth_utils::AuthUser,
    Path(session_id): Path<Uuid>,
) -> Result<Json<Vec<crate::db::ContextStoreRow>>, AppError> {
    let rows = state
        .repos()
        .context_store
        .get_active_context(session_id, 100)
        .await?;
    Ok(Json(rows))
}

/// GET /api/sessions/:session_id/requests - List router requests for a session.
#[utoipa::path(
    get,
    path = "/api/sessions/{session_id}/requests",
    tag = "Session Context",
    security(("bearer_auth" = [])),
    params(("session_id" = Uuid, Path, description = "Session ID")),
    responses(
        (status = 200, description = "List of router requests")
    )
)]
pub async fn list_session_requests(
    State(state): State<AppState>,
    _auth: auth_utils::AuthUser,
    Path(session_id): Path<Uuid>,
) -> Result<Json<Vec<crate::db::RouterRequestRow>>, AppError> {
    let rows = state
        .repos()
        .router_requests
        .list_session_requests(session_id)
        .await?;
    Ok(Json(rows))
}

#[cfg(test)]
mod tests;
