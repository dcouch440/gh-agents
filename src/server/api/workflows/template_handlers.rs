//! Run template CRUD handlers — promote, list, get, delete.

use axum::{
    extract::{Path, State},
    Json,
};
use uuid::Uuid;

use crate::server::api::AppError;
use crate::server::auth as auth_utils;
use crate::server::hub::dag::templates::{
    capture_workflow_snapshot, restore::restore_workflow_from_snapshot, WorkflowSnapshot,
};
use crate::server::state::AppState;

use super::types::{
    CreateTemplateRequest, RebaseRequest, RebaseResponse, RunTemplateDetailResponse,
    RunTemplateResponse, TemplatePath,
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

/// POST /api/workflows/:id/rebase - Restore workshop from a frozen template snapshot.
///
/// Auto-creates a backup template of the current state before overwriting.
#[utoipa::path(
    post,
    path = "/api/workflows/{id}/rebase",
    tag = "Workflows",
    security(("bearer_auth" = [])),
    params(("id" = Uuid, Path, description = "Workflow ID")),
    request_body(content = RebaseRequest, content_type = "application/json"),
    responses(
        (status = 200, description = "Workshop rebased", body = RebaseResponse),
        (status = 404, description = "Workflow or template not found")
    )
)]
pub async fn rebase_workshop(
    State(state): State<AppState>,
    auth: auth_utils::AuthUser,
    Path(id): Path<Uuid>,
    Json(body): Json<RebaseRequest>,
) -> Result<Json<RebaseResponse>, AppError> {
    let workflow_repo = &state.repos().workflows;

    // Verify workflow exists and user owns it
    let workflow = workflow_repo
        .get_workflow(id)
        .await?
        .ok_or(AppError::not_found("Workflow"))?;
    if workflow.user_id != auth.user_id.0 {
        return Err(AppError::not_found("Workflow"));
    }

    // Load the target template and verify it belongs to this workflow
    let template = workflow_repo
        .get_template(body.template_id)
        .await?
        .ok_or(AppError::not_found("Template"))?;
    if template.workflow_id != id {
        return Err(AppError::not_found("Template"));
    }

    // Deserialize the frozen snapshot
    let snapshot: WorkflowSnapshot = serde_json::from_value(template.snapshot)
        .map_err(|e| AppError::Internal(format!("Failed to deserialize snapshot: {e}")))?;

    // Safety: auto-backup current state before overwriting
    let backup_snapshot = capture_workflow_snapshot(&state, id)
        .await
        .map_err(|e| AppError::Internal(format!("Failed to capture backup: {e}")))?;

    let backup_json = serde_json::to_value(&backup_snapshot)
        .map_err(|e| AppError::Internal(format!("Failed to serialize backup: {e}")))?;

    let timestamp = chrono::Utc::now().format("%Y-%m-%d %H:%M:%S UTC");
    let backup_template = workflow_repo
        .create_template(
            id,
            auth.user_id.0,
            &format!("Pre-rebase backup {timestamp}"),
            Some(format!(
                "Auto-created before rebase from template \"{}\"",
                template.name
            )),
            backup_json,
        )
        .await?;

    // Get PgPool for transaction-based restore
    let pool = state
        .db()
        .ok_or(AppError::Internal("No database pool".to_string()))?;

    // Restore the workflow from the snapshot
    restore_workflow_from_snapshot(pool, id, &snapshot)
        .await
        .map_err(|e| AppError::Internal(format!("Rebase failed: {e}")))?;

    // Mark old workshop as rebased so a new one can be created
    sqlx::query(
        "UPDATE workflow_executions SET execution_mode = 'workshop_rebased' \
         WHERE workflow_id = $1 AND execution_mode = 'workshop'",
    )
    .bind(id)
    .execute(pool)
    .await
    .map_err(|e| AppError::Internal(format!("Failed to reset workshop: {e}")))?;

    Ok(Json(RebaseResponse {
        backup_template_id: backup_template.id,
        template_id: body.template_id,
    }))
}
