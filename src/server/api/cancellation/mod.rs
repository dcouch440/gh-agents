//! Cancellation endpoints for agent executions, chat messages, and workflow runs

use std::sync::Arc;

use axum::{
    extract::{Path, State},
    Json,
};
use uuid::Uuid;

use super::AppError;
use crate::db::pg_repo::PgRepo;
use crate::db::traits::WorkflowCollectionRepo;
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

/// POST /sessions/:session_id/chat/:message_id/cancel - Cancel an in-progress chat message.
///
/// Idempotent: returns 200 even if the token has already been cleaned up
/// (e.g. execution finished before the cancel request arrived).
#[utoipa::path(
    post,
    path = "/sessions/{session_id}/chat/{message_id}/cancel",
    params(
        ("session_id" = Uuid, Path, description = "Session UUID"),
        ("message_id" = Uuid, Path, description = "Message UUID"),
    ),
    responses(
        (status = 200, description = "Cancellation requested"),
    )
)]
pub async fn cancel_chat_message(
    State(state): State<AppState>,
    _user: auth_utils::AuthUser,
    Path((_session_id, message_id)): Path<(Uuid, Uuid)>,
) -> Result<Json<serde_json::Value>, AppError> {
    let cancelled = state.cancel_execution(message_id);
    if cancelled {
        tracing::info!("Cancelled chat message {}", message_id);
    }
    Ok(Json(serde_json::json!({ "status": "cancelled" })))
}

/// POST /workflow-executions/:execution_id/cancel - Cancel a running workflow execution (Run).
#[utoipa::path(
    post,
    path = "/workflow-executions/{execution_id}/cancel",
    params(("execution_id" = Uuid, Path, description = "Workflow execution UUID")),
    responses(
        (status = 200, description = "Execution cancelled"),
        (status = 404, description = "Execution not found or no cancellation token registered")
    )
)]
pub async fn cancel_workflow_execution(
    State(state): State<AppState>,
    _user: auth_utils::AuthUser,
    Path(execution_id): Path<Uuid>,
) -> Result<Json<serde_json::Value>, AppError> {
    let cancelled = state.cancel_execution(execution_id);
    if !cancelled {
        return Err(AppError::not_found("Execution"));
    }

    let db = state
        .db()
        .ok_or(AppError::Internal("Database not available".into()))?
        .clone();
    let collection_repo: Arc<dyn WorkflowCollectionRepo> = Arc::new(PgRepo::new(db));
    let _ = collection_repo
        .update_workflow_execution_status(execution_id, "cancelled", None, None)
        .await;

    Ok(Json(serde_json::json!({ "status": "cancelled" })))
}

#[cfg(test)]
mod tests;
