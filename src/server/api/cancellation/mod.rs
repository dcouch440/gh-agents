//! Agent execution cancellation endpoint

use axum::{
    extract::{Path, State},
    Json,
};
use uuid::Uuid;

use super::AppError;
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
) -> Result<Json<serde_json::Value>, AppError> {
    let exec_uuid =
        Uuid::parse_str(&execution_id).map_err(|e| AppError::BadRequest(e.to_string()))?;

    let cancelled = state.cancel_execution(exec_uuid);
    if !cancelled {
        return Err(AppError::not_found("Execution"));
    }

    // Update execution status in DB
    let ae_repo = &state.repos().agent_executions;
    let _ = ae_repo
        .update_agent_execution_status(exec_uuid, "cancelled", None, None)
        .await;

    Ok(Json(serde_json::json!({ "status": "cancelled" })))
}

#[cfg(test)]
mod tests;
