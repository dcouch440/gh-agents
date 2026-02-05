//! Agent execution cancellation endpoint

use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use uuid::Uuid;

use crate::server::auth as auth_utils;
use crate::server::state::AppState;

/// POST /agent-executions/:execution_id/cancel - Cancel a running agent execution.
#[utoipa::path(
    post,
    path = "/agent-executions/{execution_id}/cancel",
    params(("execution_id" = String, Path, description = "Agent execution UUID")),
    responses(
        (status = 200, description = "Execution cancelled"),
        (status = 404, description = "Execution not found or no cancellation token registered")
    )
)]
pub async fn cancel_agent_execution(
    State(state): State<AppState>,
    _user: auth_utils::AuthUser,
    Path(execution_id): Path<String>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let exec_uuid = Uuid::parse_str(&execution_id).map_err(|_| StatusCode::BAD_REQUEST)?;

    let cancelled = state.cancel_execution(exec_uuid);
    if !cancelled {
        return Err(StatusCode::NOT_FOUND);
    }

    // Update execution status in DB
    if let Some(ae_repo) = state.agent_execution_repo() {
        let _ = ae_repo
            .update_agent_execution_status(exec_uuid, "cancelled", None, None)
            .await;
    }

    Ok(Json(serde_json::json!({ "status": "cancelled" })))
}

#[cfg(test)]
mod tests;
