//! Workflow, step, edge, and document attachment endpoints

use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::constants::MAX_TITLE_LENGTH;
use crate::server::auth as auth_utils;
use crate::server::state::AppState;

// ============================================================================
// Workflow Types
// ============================================================================

#[derive(Serialize, utoipa::ToSchema)]
pub struct WorkflowResponse {
    pub id: Uuid,
    pub name: String,
    pub description: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Deserialize, utoipa::ToSchema)]
pub struct CreateWorkflowRequest {
    pub name: String,
    pub description: Option<String>,
}

#[derive(Deserialize, utoipa::ToSchema)]
pub struct UpdateWorkflowRequest {
    pub name: Option<String>,
    pub description: Option<String>,
}

// ============================================================================
// Workflow Step Types
// ============================================================================

#[derive(Serialize, utoipa::ToSchema)]
pub struct WorkflowStepResponse {
    pub id: Uuid,
    pub workflow_id: Uuid,
    pub agent_id: Uuid,
    pub execution_mode: String,
    pub for_each_ref: Option<String>,
    pub prompt_template_id: Option<Uuid>,
    pub prompt_template: String,
    pub output_schema_id: Option<Uuid>,
    pub output_variable_name: Option<String>,
    pub interactive_agent_id: Option<Uuid>,
    pub for_each_label_field: Option<String>,
    pub display_order: i32,
    pub version: i32,
}

#[derive(Deserialize, utoipa::ToSchema)]
pub struct CreateStepRequest {
    pub agent_id: Uuid,
    pub execution_mode: Option<String>,
    pub for_each_ref: Option<String>,
    pub prompt_template_id: Option<Uuid>,
    pub prompt_template: Option<String>,
    pub output_schema_id: Option<Uuid>,
    pub output_variable_name: Option<String>,
    pub interactive_agent_id: Option<Uuid>,
    pub for_each_label_field: Option<String>,
    pub display_order: Option<i32>,
}

#[derive(Deserialize, utoipa::ToSchema)]
pub struct UpdateStepRequest {
    pub agent_id: Uuid,
    pub execution_mode: Option<String>,
    pub for_each_ref: Option<String>,
    pub prompt_template_id: Option<Uuid>,
    pub prompt_template: Option<String>,
    pub output_schema_id: Option<Uuid>,
    pub output_variable_name: Option<String>,
    pub interactive_agent_id: Option<Uuid>,
    pub for_each_label_field: Option<String>,
    pub display_order: Option<i32>,
}

#[derive(Deserialize, utoipa::ToSchema)]
pub struct WorkflowStepPath {
    pub wid: Uuid,
    pub sid: Uuid,
}

// ============================================================================
// Edge Types
// ============================================================================

#[derive(Deserialize, utoipa::ToSchema)]
pub struct EdgeRequest {
    pub from_step_id: Uuid,
    pub to_step_id: Uuid,
}

#[derive(Serialize, utoipa::ToSchema)]
pub struct EdgeResponse {
    pub from_step_id: Uuid,
    pub to_step_id: Uuid,
}

// ============================================================================
// Step Document Types
// ============================================================================

#[derive(Deserialize, utoipa::ToSchema)]
pub struct StepDocumentRequest {
    pub document_id: Uuid,
}

#[derive(Serialize, utoipa::ToSchema)]
pub struct StepDocumentResponse {
    pub step_id: Uuid,
    pub document_id: Uuid,
}

// ============================================================================
// Helper Functions
// ============================================================================

fn step_response(r: crate::db::WorkflowStepRow) -> WorkflowStepResponse {
    WorkflowStepResponse {
        id: r.id,
        workflow_id: r.workflow_id,
        agent_id: r.agent_id,
        execution_mode: r.execution_mode,
        for_each_ref: r.for_each_ref,
        prompt_template_id: r.prompt_template_id,
        prompt_template: r.prompt_template,
        output_schema_id: r.output_schema_id,
        output_variable_name: r.output_variable_name,
        interactive_agent_id: r.interactive_agent_id,
        for_each_label_field: r.for_each_label_field,
        display_order: r.display_order,
        version: r.version,
    }
}

// ============================================================================
// Workflow Handlers
// ============================================================================

/// GET /api/workflows
#[utoipa::path(
    get,
    path = "/api/workflows",
    tag = "Workflows",
    security(("bearer_auth" = [])),
    responses(
        (status = 200, description = "List of workflows", body = Vec<WorkflowResponse>)
    )
)]
pub async fn list_workflows(State(state): State<AppState>, auth: auth_utils::AuthUser) -> Result<Json<Vec<WorkflowResponse>>, StatusCode> {
    let repo = state.workflow_repo.as_ref().ok_or(StatusCode::INTERNAL_SERVER_ERROR)?;
    let rows = repo.list_workflows(auth.user_id.0).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let items = rows
        .into_iter()
        .map(|r| WorkflowResponse {
            id: r.id,
            name: r.name,
            description: r.description,
            created_at: r.created_at,
        })
        .collect();
    Ok(Json(items))
}

/// POST /api/workflows
#[utoipa::path(
    post,
    path = "/api/workflows",
    tag = "Workflows",
    security(("bearer_auth" = [])),
    request_body = CreateWorkflowRequest,
    responses(
        (status = 201, description = "Workflow created", body = WorkflowResponse),
        (status = 400, description = "Invalid request")
    )
)]
pub async fn create_workflow(State(state): State<AppState>, auth: auth_utils::AuthUser, Json(req): Json<CreateWorkflowRequest>) -> Result<(StatusCode, Json<WorkflowResponse>), StatusCode> {
    if req.name.trim().is_empty() || req.name.len() > MAX_TITLE_LENGTH {
        return Err(StatusCode::BAD_REQUEST);
    }
    let repo = state.workflow_repo.as_ref().ok_or(StatusCode::INTERNAL_SERVER_ERROR)?;
    let row = repo
        .create_workflow(auth.user_id.0, req.name, req.description.unwrap_or_default())
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok((
        StatusCode::CREATED,
        Json(WorkflowResponse {
            id: row.id,
            name: row.name,
            description: row.description,
            created_at: row.created_at,
        }),
    ))
}

/// GET /api/workflows/:id
#[utoipa::path(
    get,
    path = "/api/workflows/{id}",
    tag = "Workflows",
    security(("bearer_auth" = [])),
    params(("id" = Uuid, Path, description = "Workflow ID")),
    responses(
        (status = 200, description = "Workflow found", body = WorkflowResponse),
        (status = 404, description = "Not found")
    )
)]
pub async fn get_workflow(State(state): State<AppState>, auth: auth_utils::AuthUser, Path(id): Path<Uuid>) -> Result<Json<WorkflowResponse>, StatusCode> {
    let repo = state.workflow_repo.as_ref().ok_or(StatusCode::INTERNAL_SERVER_ERROR)?;
    let row = repo.get_workflow(id).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?.ok_or(StatusCode::NOT_FOUND)?;
    if row.user_id != auth.user_id.0 {
        return Err(StatusCode::NOT_FOUND);
    }
    Ok(Json(WorkflowResponse {
        id: row.id,
        name: row.name,
        description: row.description,
        created_at: row.created_at,
    }))
}

/// PUT /api/workflows/:id
#[utoipa::path(
    put,
    path = "/api/workflows/{id}",
    tag = "Workflows",
    security(("bearer_auth" = [])),
    params(("id" = Uuid, Path, description = "Workflow ID")),
    request_body = UpdateWorkflowRequest,
    responses(
        (status = 200, description = "Updated workflow", body = WorkflowResponse),
        (status = 404, description = "Not found")
    )
)]
pub async fn update_workflow(State(state): State<AppState>, auth: auth_utils::AuthUser, Path(id): Path<Uuid>, Json(req): Json<UpdateWorkflowRequest>) -> Result<Json<WorkflowResponse>, StatusCode> {
    let repo = state.workflow_repo.as_ref().ok_or(StatusCode::INTERNAL_SERVER_ERROR)?;
    let existing = repo.get_workflow(id).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?.ok_or(StatusCode::NOT_FOUND)?;
    if existing.user_id != auth.user_id.0 {
        return Err(StatusCode::NOT_FOUND);
    }
    if let Some(ref name) = req.name {
        if name.trim().is_empty() || name.len() > MAX_TITLE_LENGTH {
            return Err(StatusCode::BAD_REQUEST);
        }
    }
    let row = repo.update_workflow(id, req.name, req.description).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Json(WorkflowResponse {
        id: row.id,
        name: row.name,
        description: row.description,
        created_at: row.created_at,
    }))
}

/// DELETE /api/workflows/:id
#[utoipa::path(
    delete,
    path = "/api/workflows/{id}",
    tag = "Workflows",
    security(("bearer_auth" = [])),
    params(("id" = Uuid, Path, description = "Workflow ID")),
    responses(
        (status = 204, description = "Deleted successfully"),
        (status = 404, description = "Not found")
    )
)]
pub async fn delete_workflow(State(state): State<AppState>, auth: auth_utils::AuthUser, Path(id): Path<Uuid>) -> Result<StatusCode, StatusCode> {
    let repo = state.workflow_repo.as_ref().ok_or(StatusCode::INTERNAL_SERVER_ERROR)?;
    let existing = repo.get_workflow(id).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?.ok_or(StatusCode::NOT_FOUND)?;
    if existing.user_id != auth.user_id.0 {
        return Err(StatusCode::NOT_FOUND);
    }
    repo.delete_workflow(id).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(StatusCode::NO_CONTENT)
}

// ============================================================================
// Workflow Step Handlers
// ============================================================================

/// POST /api/workflows/:id/steps
#[utoipa::path(
    post,
    path = "/api/workflows/{id}/steps",
    tag = "Workflow Steps",
    security(("bearer_auth" = [])),
    params(("id" = Uuid, Path, description = "Workflow ID")),
    request_body = CreateStepRequest,
    responses(
        (status = 201, description = "Step created", body = WorkflowStepResponse),
        (status = 404, description = "Workflow not found")
    )
)]
pub async fn create_workflow_step(
    State(state): State<AppState>,
    auth: auth_utils::AuthUser,
    Path(wid): Path<Uuid>,
    Json(req): Json<CreateStepRequest>,
) -> Result<(StatusCode, Json<WorkflowStepResponse>), StatusCode> {
    let repo = state.workflow_repo.as_ref().ok_or(StatusCode::INTERNAL_SERVER_ERROR)?;
    let wf = repo.get_workflow(wid).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?.ok_or(StatusCode::NOT_FOUND)?;
    if wf.user_id != auth.user_id.0 {
        return Err(StatusCode::NOT_FOUND);
    }
    let step = crate::db::WorkflowStepRow {
        id: Uuid::new_v4(),
        workflow_id: wid,
        agent_id: req.agent_id,
        execution_mode: req.execution_mode.unwrap_or_else(|| "single".to_string()),
        for_each_ref: req.for_each_ref,
        prompt_template_id: req.prompt_template_id,
        prompt_template: req.prompt_template.unwrap_or_default(),
        output_schema_id: req.output_schema_id,
        output_variable_name: req.output_variable_name,
        interactive_agent_id: req.interactive_agent_id,
        for_each_label_field: req.for_each_label_field,
        room_id: None,
        display_order: req.display_order.unwrap_or(0),
        version: 1,
    };
    let row = repo.create_step(step).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok((StatusCode::CREATED, Json(step_response(row))))
}

/// GET /api/workflows/:id/steps
#[utoipa::path(
    get,
    path = "/api/workflows/{id}/steps",
    tag = "Workflow Steps",
    security(("bearer_auth" = [])),
    params(("id" = Uuid, Path, description = "Workflow ID")),
    responses(
        (status = 200, description = "List of workflow steps", body = Vec<WorkflowStepResponse>),
        (status = 404, description = "Not found")
    )
)]
pub async fn list_workflow_steps(State(state): State<AppState>, auth: auth_utils::AuthUser, Path(wid): Path<Uuid>) -> Result<Json<Vec<WorkflowStepResponse>>, StatusCode> {
    let repo = state.workflow_repo.as_ref().ok_or(StatusCode::INTERNAL_SERVER_ERROR)?;
    let wf = repo.get_workflow(wid).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?.ok_or(StatusCode::NOT_FOUND)?;
    if wf.user_id != auth.user_id.0 {
        return Err(StatusCode::NOT_FOUND);
    }
    let rows = repo.list_steps(wid).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Json(rows.into_iter().map(step_response).collect()))
}

/// GET /api/workflows/:wid/steps/:sid
#[utoipa::path(
    get,
    path = "/api/workflows/{wid}/steps/{sid}",
    tag = "Workflow Steps",
    security(("bearer_auth" = [])),
    params(
        ("wid" = Uuid, Path, description = "Workflow ID"),
        ("sid" = Uuid, Path, description = "Step ID")
    ),
    responses(
        (status = 200, description = "Workflow step found", body = WorkflowStepResponse),
        (status = 404, description = "Not found")
    )
)]
pub async fn get_workflow_step(State(state): State<AppState>, auth: auth_utils::AuthUser, Path(p): Path<(Uuid, Uuid)>) -> Result<Json<WorkflowStepResponse>, StatusCode> {
    let repo = state.workflow_repo.as_ref().ok_or(StatusCode::INTERNAL_SERVER_ERROR)?;
    let wf = repo.get_workflow(p.0).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?.ok_or(StatusCode::NOT_FOUND)?;
    if wf.user_id != auth.user_id.0 {
        return Err(StatusCode::NOT_FOUND);
    }
    let step = repo.get_step(p.1).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?.ok_or(StatusCode::NOT_FOUND)?;
    if step.workflow_id != p.0 {
        return Err(StatusCode::NOT_FOUND);
    }
    Ok(Json(step_response(step)))
}

/// PUT /api/workflows/:wid/steps/:sid
#[utoipa::path(
    put,
    path = "/api/workflows/{wid}/steps/{sid}",
    tag = "Workflow Steps",
    security(("bearer_auth" = [])),
    params(
        ("wid" = Uuid, Path, description = "Workflow ID"),
        ("sid" = Uuid, Path, description = "Step ID")
    ),
    request_body = UpdateStepRequest,
    responses(
        (status = 200, description = "Updated workflow step", body = WorkflowStepResponse),
        (status = 404, description = "Not found")
    )
)]
pub async fn update_workflow_step(
    State(state): State<AppState>,
    auth: auth_utils::AuthUser,
    Path(p): Path<WorkflowStepPath>,
    Json(req): Json<UpdateStepRequest>,
) -> Result<Json<WorkflowStepResponse>, StatusCode> {
    let repo = state.workflow_repo.as_ref().ok_or(StatusCode::INTERNAL_SERVER_ERROR)?;
    let wf = repo.get_workflow(p.wid).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?.ok_or(StatusCode::NOT_FOUND)?;
    if wf.user_id != auth.user_id.0 {
        return Err(StatusCode::NOT_FOUND);
    }
    let existing = repo.get_step(p.sid).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?.ok_or(StatusCode::NOT_FOUND)?;
    if existing.workflow_id != p.wid {
        return Err(StatusCode::NOT_FOUND);
    }
    let step = crate::db::WorkflowStepRow {
        id: p.sid,
        workflow_id: p.wid,
        agent_id: req.agent_id,
        execution_mode: req.execution_mode.unwrap_or(existing.execution_mode),
        for_each_ref: req.for_each_ref,
        prompt_template_id: req.prompt_template_id,
        prompt_template: req.prompt_template.unwrap_or(existing.prompt_template),
        output_schema_id: req.output_schema_id,
        output_variable_name: req.output_variable_name,
        interactive_agent_id: req.interactive_agent_id,
        for_each_label_field: req.for_each_label_field.or(existing.for_each_label_field),
        room_id: existing.room_id,
        display_order: req.display_order.unwrap_or(existing.display_order),
        version: existing.version,
    };
    let row = repo.update_step(step).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Json(step_response(row)))
}

/// DELETE /api/workflows/:wid/steps/:sid
#[utoipa::path(
    delete,
    path = "/api/workflows/{wid}/steps/{sid}",
    tag = "Workflow Steps",
    security(("bearer_auth" = [])),
    params(
        ("wid" = Uuid, Path, description = "Workflow ID"),
        ("sid" = Uuid, Path, description = "Step ID")
    ),
    responses(
        (status = 204, description = "Deleted successfully"),
        (status = 404, description = "Not found")
    )
)]
pub async fn delete_workflow_step(State(state): State<AppState>, auth: auth_utils::AuthUser, Path(p): Path<WorkflowStepPath>) -> Result<StatusCode, StatusCode> {
    let repo = state.workflow_repo.as_ref().ok_or(StatusCode::INTERNAL_SERVER_ERROR)?;
    let wf = repo.get_workflow(p.wid).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?.ok_or(StatusCode::NOT_FOUND)?;
    if wf.user_id != auth.user_id.0 {
        return Err(StatusCode::NOT_FOUND);
    }
    let existing = repo.get_step(p.sid).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?.ok_or(StatusCode::NOT_FOUND)?;
    if existing.workflow_id != p.wid {
        return Err(StatusCode::NOT_FOUND);
    }
    repo.delete_step(p.sid).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(StatusCode::NO_CONTENT)
}

// ============================================================================
// Workflow Edge Handlers
// ============================================================================

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
pub async fn list_workflow_edges(State(state): State<AppState>, auth: auth_utils::AuthUser, Path(wid): Path<Uuid>) -> Result<Json<Vec<EdgeResponse>>, StatusCode> {
    let repo = state.workflow_repo.as_ref().ok_or(StatusCode::INTERNAL_SERVER_ERROR)?;
    let wf = repo.get_workflow(wid).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?.ok_or(StatusCode::NOT_FOUND)?;
    if wf.user_id != auth.user_id.0 {
        return Err(StatusCode::NOT_FOUND);
    }
    let rows = repo.list_edges(wid).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Json(
        rows.into_iter()
            .map(|e| EdgeResponse {
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
        (status = 201, description = "Edge added"),
        (status = 404, description = "Not found")
    )
)]
pub async fn add_workflow_edge(State(state): State<AppState>, auth: auth_utils::AuthUser, Path(wid): Path<Uuid>, Json(req): Json<EdgeRequest>) -> Result<StatusCode, StatusCode> {
    let repo = state.workflow_repo.as_ref().ok_or(StatusCode::INTERNAL_SERVER_ERROR)?;
    let wf = repo.get_workflow(wid).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?.ok_or(StatusCode::NOT_FOUND)?;
    if wf.user_id != auth.user_id.0 {
        return Err(StatusCode::NOT_FOUND);
    }
    repo.add_edge(req.from_step_id, req.to_step_id).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(StatusCode::CREATED)
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
pub async fn remove_workflow_edge(State(state): State<AppState>, auth: auth_utils::AuthUser, Path(wid): Path<Uuid>, Json(req): Json<EdgeRequest>) -> Result<StatusCode, StatusCode> {
    let repo = state.workflow_repo.as_ref().ok_or(StatusCode::INTERNAL_SERVER_ERROR)?;
    let wf = repo.get_workflow(wid).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?.ok_or(StatusCode::NOT_FOUND)?;
    if wf.user_id != auth.user_id.0 {
        return Err(StatusCode::NOT_FOUND);
    }
    repo.remove_edge(req.from_step_id, req.to_step_id).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(StatusCode::NO_CONTENT)
}

// ============================================================================
// Step Document Handlers
// ============================================================================

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
pub async fn add_step_document(State(state): State<AppState>, auth: auth_utils::AuthUser, Path(p): Path<WorkflowStepPath>, Json(req): Json<StepDocumentRequest>) -> Result<StatusCode, StatusCode> {
    let repo = state.workflow_repo.as_ref().ok_or(StatusCode::INTERNAL_SERVER_ERROR)?;
    let wf = repo.get_workflow(p.wid).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?.ok_or(StatusCode::NOT_FOUND)?;
    if wf.user_id != auth.user_id.0 {
        return Err(StatusCode::NOT_FOUND);
    }
    let existing = repo.get_step(p.sid).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?.ok_or(StatusCode::NOT_FOUND)?;
    if existing.workflow_id != p.wid {
        return Err(StatusCode::NOT_FOUND);
    }
    repo.add_step_document(p.sid, req.document_id).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
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
pub async fn remove_step_document(State(state): State<AppState>, auth: auth_utils::AuthUser, Path(p): Path<WorkflowStepPath>, Json(req): Json<StepDocumentRequest>) -> Result<StatusCode, StatusCode> {
    let repo = state.workflow_repo.as_ref().ok_or(StatusCode::INTERNAL_SERVER_ERROR)?;
    let wf = repo.get_workflow(p.wid).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?.ok_or(StatusCode::NOT_FOUND)?;
    if wf.user_id != auth.user_id.0 {
        return Err(StatusCode::NOT_FOUND);
    }
    let existing = repo.get_step(p.sid).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?.ok_or(StatusCode::NOT_FOUND)?;
    if existing.workflow_id != p.wid {
        return Err(StatusCode::NOT_FOUND);
    }
    repo.remove_step_document(p.sid, req.document_id).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
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
pub async fn list_step_documents(State(state): State<AppState>, auth: auth_utils::AuthUser, Path(p): Path<WorkflowStepPath>) -> Result<Json<Vec<StepDocumentResponse>>, StatusCode> {
    let repo = state.workflow_repo.as_ref().ok_or(StatusCode::INTERNAL_SERVER_ERROR)?;
    let wf = repo.get_workflow(p.wid).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?.ok_or(StatusCode::NOT_FOUND)?;
    if wf.user_id != auth.user_id.0 {
        return Err(StatusCode::NOT_FOUND);
    }
    let rows = repo.list_step_documents(p.sid).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Json(
        rows.into_iter()
            .map(|r| StepDocumentResponse {
                step_id: r.step_id,
                document_id: r.document_id,
            })
            .collect(),
    ))
}

#[cfg(test)]
mod tests;
