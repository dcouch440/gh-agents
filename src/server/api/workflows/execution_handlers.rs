//! Workflow execution history handler

use axum::{
    extract::{Path, State},
    Json,
};
use std::sync::Arc;
use uuid::Uuid;

use crate::db::pg_repo::PgRepo;
use crate::db::traits::WorkflowCollectionRepo;
use crate::server::api::AppError;
use crate::server::auth as auth_utils;
use crate::server::state::AppState;

use super::types::WorkflowExecutionResponse;

/// GET /api/workflows/:id/executions - List executions for a workflow (most recent first).
#[utoipa::path(
    get,
    path = "/api/workflows/{id}/executions",
    tag = "Workflows",
    security(("bearer_auth" = [])),
    params(("id" = Uuid, Path, description = "Workflow ID")),
    responses(
        (status = 200, description = "List of workflow executions", body = Vec<WorkflowExecutionResponse>),
        (status = 404, description = "Workflow not found")
    )
)]
pub async fn list_workflow_executions(
    State(state): State<AppState>,
    auth: auth_utils::AuthUser,
    Path(id): Path<Uuid>,
) -> Result<Json<Vec<WorkflowExecutionResponse>>, AppError> {
    let workflow_repo = &state.repos().workflows;

    // Verify workflow exists and user owns it
    let workflow = workflow_repo
        .get_workflow(id)
        .await?
        .ok_or(AppError::not_found("Workflow"))?;
    if workflow.user_id != auth.user_id.0 {
        return Err(AppError::not_found("Workflow"));
    }

    let db = state
        .db()
        .ok_or(AppError::Internal("Database not available".into()))?
        .clone();
    let collection_repo: Arc<dyn WorkflowCollectionRepo> = Arc::new(PgRepo::new(db));
    let rows = collection_repo
        .list_workflow_executions_by_workflow(id, auth.user_id.0)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?;

    let items = rows
        .into_iter()
        .map(|r| WorkflowExecutionResponse {
            id: r.id,
            workflow_id: r.workflow_id,
            status: r.status,
            started_at: r.started_at,
            completed_at: r.completed_at,
            outputs: r.outputs,
            error: r.error,
            execution_mode: r.execution_mode,
            template_id: r.template_id,
        })
        .collect();

    Ok(Json(items))
}
