//! API handlers for workflow version checkpoints.

use axum::extract::{Json, Path, State};
use uuid::Uuid;

use crate::server::api::AppError;
use crate::server::auth as auth_utils;
use crate::server::services::workflow_agent::versions;
use crate::server::state::AppState;

/// POST /workflows/:id/versions — save a version checkpoint.
pub async fn save_workflow_version(
    State(state): State<AppState>,
    auth: auth_utils::AuthUser,
    Path(workflow_id): Path<Uuid>,
    Json(req): Json<super::types::SaveVersionRequest>,
) -> Result<Json<super::types::VersionResponse>, AppError> {
    // Verify ownership
    let wf = state
        .repos()
        .workflows
        .get_workflow(workflow_id)
        .await?
        .ok_or(AppError::not_found("Workflow"))?;
    if wf.user_id != auth.user_id.0 {
        return Err(AppError::not_found("Workflow"));
    }

    let version = versions::save_version(workflow_id, auth.user_id.0, req.label, "user", &state)
        .await
        .map_err(|e| AppError::Internal(format!("{e}")))?;

    Ok(Json(super::types::VersionResponse::from(version)))
}

/// GET /workflows/:id/versions — list all version checkpoints.
pub async fn list_workflow_versions(
    State(state): State<AppState>,
    auth: auth_utils::AuthUser,
    Path(workflow_id): Path<Uuid>,
) -> Result<Json<Vec<super::types::VersionResponse>>, AppError> {
    let wf = state
        .repos()
        .workflows
        .get_workflow(workflow_id)
        .await?
        .ok_or(AppError::not_found("Workflow"))?;
    if wf.user_id != auth.user_id.0 {
        return Err(AppError::not_found("Workflow"));
    }

    let versions = versions::list_versions(workflow_id, &state)
        .await
        .map_err(|e| AppError::Internal(format!("{e}")))?;

    Ok(Json(
        versions
            .into_iter()
            .map(super::types::VersionResponse::from)
            .collect(),
    ))
}

/// POST /workflows/:id/versions/:vid/restore — rebase to a version.
pub async fn restore_workflow_version(
    State(state): State<AppState>,
    auth: auth_utils::AuthUser,
    Path((workflow_id, version_id)): Path<(Uuid, Uuid)>,
) -> Result<Json<super::types::RestoreResponse>, AppError> {
    let wf = state
        .repos()
        .workflows
        .get_workflow(workflow_id)
        .await?
        .ok_or(AppError::not_found("Workflow"))?;
    if wf.user_id != auth.user_id.0 {
        return Err(AppError::not_found("Workflow"));
    }

    let auto_checkpoint =
        versions::restore_version(workflow_id, version_id, auth.user_id.0, &state)
            .await
            .map_err(|e| AppError::Internal(format!("{e}")))?;

    Ok(Json(super::types::RestoreResponse {
        auto_checkpoint_id: auto_checkpoint.id,
        auto_checkpoint_version: auto_checkpoint.version_number,
        message: "Workflow restored. Auto-checkpoint saved for undo.".to_string(),
    }))
}
