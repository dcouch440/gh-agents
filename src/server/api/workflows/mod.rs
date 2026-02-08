//! Workflow, step, edge, document attachment, and execution endpoints

use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use uuid::Uuid;

use super::AppError;
use crate::constants::MAX_TITLE_LENGTH;
use crate::db::pg_repo::PgRepo;
use crate::db::traits::WorkflowCollectionRepo;
use crate::server::auth as auth_utils;
use crate::server::hub::dag::{execute_workflow_via_engine, WorkflowExecutionContext};
use crate::server::hub::ExecutionEngine;
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
    pub container_enabled: bool,
    pub target_repo_url: Option<String>,
    pub target_branch: Option<String>,
    pub vpn_enabled: bool,
}

#[derive(Deserialize, utoipa::ToSchema)]
pub struct CreateWorkflowRequest {
    pub name: String,
    pub description: Option<String>,
    pub container_enabled: Option<bool>,
    pub target_repo_url: Option<String>,
    pub target_branch: Option<String>,
    pub vpn_enabled: Option<bool>,
}

#[derive(Deserialize, utoipa::ToSchema)]
pub struct UpdateWorkflowRequest {
    pub name: Option<String>,
    pub description: Option<String>,
    pub container_enabled: Option<bool>,
    pub target_repo_url: Option<Option<String>>,
    pub target_branch: Option<Option<String>>,
    pub vpn_enabled: Option<bool>,
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
    pub reasoning_trace: bool,
    pub verification_agent_ids: Vec<Uuid>,
    pub position_x: Option<f64>,
    pub position_y: Option<f64>,
    pub name: Option<String>,
    pub system_prompt_suffix: Option<String>,
}

#[derive(Deserialize, utoipa::ToSchema)]
pub struct CreateStepRequest {
    pub agent_id: Option<Uuid>,
    pub execution_mode: Option<String>,
    pub for_each_ref: Option<String>,
    pub prompt_template_id: Option<Uuid>,
    pub prompt_template: Option<String>,
    pub output_schema_id: Option<Uuid>,
    pub output_variable_name: Option<String>,
    pub interactive_agent_id: Option<Uuid>,
    pub for_each_label_field: Option<String>,
    pub display_order: Option<i32>,
    pub reasoning_trace: Option<bool>,
    pub verification_agent_ids: Option<Vec<Uuid>>,
    pub position_x: Option<f64>,
    pub position_y: Option<f64>,
    pub name: Option<String>,
    pub system_prompt_suffix: Option<String>,
}

#[derive(Deserialize, utoipa::ToSchema)]
pub struct UpdateStepRequest {
    pub agent_id: Option<Uuid>,
    pub execution_mode: Option<String>,
    pub for_each_ref: Option<String>,
    pub prompt_template_id: Option<Uuid>,
    pub prompt_template: Option<String>,
    pub output_schema_id: Option<Uuid>,
    pub output_variable_name: Option<String>,
    pub interactive_agent_id: Option<Uuid>,
    pub for_each_label_field: Option<String>,
    pub display_order: Option<i32>,
    pub reasoning_trace: Option<bool>,
    pub verification_agent_ids: Option<Vec<Uuid>>,
    pub position_x: Option<f64>,
    pub position_y: Option<f64>,
    pub name: Option<String>,
    pub system_prompt_suffix: Option<String>,
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
    pub id: Uuid,
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
        reasoning_trace: r.reasoning_trace,
        verification_agent_ids: r
            .verification_agent_ids
            .and_then(|v| serde_json::from_value(v).ok())
            .unwrap_or_default(),
        position_x: r.position_x,
        position_y: r.position_y,
        name: r.name,
        system_prompt_suffix: r.system_prompt_suffix,
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
pub async fn list_workflows(
    State(state): State<AppState>,
    auth: auth_utils::AuthUser,
) -> Result<Json<Vec<WorkflowResponse>>, AppError> {
    let repo = &state.repos().workflows;
    let rows = repo.list_workflows(auth.user_id.0).await?;
    let items = rows
        .into_iter()
        .map(|r| WorkflowResponse {
            id: r.id,
            name: r.name,
            description: r.description,
            created_at: r.created_at,
            container_enabled: r.container_enabled,
            target_repo_url: r.target_repo_url,
            target_branch: r.target_branch,
            vpn_enabled: r.vpn_enabled,
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
pub async fn create_workflow(
    State(state): State<AppState>,
    auth: auth_utils::AuthUser,
    Json(req): Json<CreateWorkflowRequest>,
) -> Result<(StatusCode, Json<WorkflowResponse>), AppError> {
    if req.name.trim().is_empty() || req.name.len() > MAX_TITLE_LENGTH {
        return Err(AppError::bad_request(
            "Workflow name must be non-empty and within length limit",
        ));
    }
    let repo = &state.repos().workflows;
    let row = repo
        .create_workflow(
            auth.user_id.0,
            req.name,
            req.description.unwrap_or_default(),
            req.container_enabled.unwrap_or(false),
            req.target_repo_url,
            req.target_branch,
            req.vpn_enabled.unwrap_or(false),
        )
        .await?;
    Ok((
        StatusCode::CREATED,
        Json(WorkflowResponse {
            id: row.id,
            name: row.name,
            description: row.description,
            created_at: row.created_at,
            container_enabled: row.container_enabled,
            target_repo_url: row.target_repo_url,
            target_branch: row.target_branch,
            vpn_enabled: row.vpn_enabled,
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
pub async fn get_workflow(
    State(state): State<AppState>,
    auth: auth_utils::AuthUser,
    Path(id): Path<Uuid>,
) -> Result<Json<WorkflowResponse>, AppError> {
    let repo = &state.repos().workflows;
    let row = repo
        .get_workflow(id)
        .await?
        .ok_or(AppError::not_found("Workflow"))?;
    if row.user_id != auth.user_id.0 {
        return Err(AppError::not_found("Workflow"));
    }
    Ok(Json(WorkflowResponse {
        id: row.id,
        name: row.name,
        description: row.description,
        created_at: row.created_at,
        container_enabled: row.container_enabled,
        target_repo_url: row.target_repo_url,
        target_branch: row.target_branch,
        vpn_enabled: row.vpn_enabled,
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
pub async fn update_workflow(
    State(state): State<AppState>,
    auth: auth_utils::AuthUser,
    Path(id): Path<Uuid>,
    Json(req): Json<UpdateWorkflowRequest>,
) -> Result<Json<WorkflowResponse>, AppError> {
    let repo = &state.repos().workflows;
    let existing = repo
        .get_workflow(id)
        .await?
        .ok_or(AppError::not_found("Workflow"))?;
    if existing.user_id != auth.user_id.0 {
        return Err(AppError::not_found("Workflow"));
    }
    if let Some(ref name) = req.name {
        if name.trim().is_empty() || name.len() > MAX_TITLE_LENGTH {
            return Err(AppError::bad_request(
                "Workflow name must be non-empty and within length limit",
            ));
        }
    }
    let row = repo
        .update_workflow(
            id,
            req.name,
            req.description,
            req.container_enabled,
            req.target_repo_url,
            req.target_branch,
            req.vpn_enabled,
        )
        .await?;
    Ok(Json(WorkflowResponse {
        id: row.id,
        name: row.name,
        description: row.description,
        created_at: row.created_at,
        container_enabled: row.container_enabled,
        target_repo_url: row.target_repo_url,
        target_branch: row.target_branch,
        vpn_enabled: row.vpn_enabled,
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
pub async fn delete_workflow(
    State(state): State<AppState>,
    auth: auth_utils::AuthUser,
    Path(id): Path<Uuid>,
) -> Result<StatusCode, AppError> {
    let repo = &state.repos().workflows;
    let existing = repo
        .get_workflow(id)
        .await?
        .ok_or(AppError::not_found("Workflow"))?;
    if existing.user_id != auth.user_id.0 {
        return Err(AppError::not_found("Workflow"));
    }
    repo.delete_workflow(id).await?;
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
) -> Result<(StatusCode, Json<WorkflowStepResponse>), AppError> {
    let repo = &state.repos().workflows;
    let wf = repo
        .get_workflow(wid)
        .await?
        .ok_or(AppError::not_found("Workflow"))?;
    if wf.user_id != auth.user_id.0 {
        return Err(AppError::not_found("Workflow"));
    }
    let step = crate::db::WorkflowStepRow {
        id: Uuid::new_v4(),
        workflow_id: wid,
        agent_id: req.agent_id.unwrap_or(crate::constants::DEFAULT_AGENT_ID),
        execution_mode: req.execution_mode.unwrap_or_else(|| "single".to_string()),
        agent_execution_mode: None, // NULL = inherit from workflow
        for_each_ref: req.for_each_ref,
        prompt_template_id: req.prompt_template_id,
        prompt_template: req.prompt_template.unwrap_or_default(),
        output_schema_id: req.output_schema_id,
        output_variable_name: req.output_variable_name,
        interactive_agent_id: req.interactive_agent_id,
        for_each_label_field: req.for_each_label_field,
        room_id: None,
        routing_mode: None,
        routing_field: None,
        display_order: req.display_order.unwrap_or(0),
        version: 1,
        reasoning_trace: req.reasoning_trace.unwrap_or(false),
        verification_agent_ids: req
            .verification_agent_ids
            .map(|ids| serde_json::to_value(ids).unwrap()),
        position_x: req.position_x,
        position_y: req.position_y,
        name: req.name,
        system_prompt_suffix: req.system_prompt_suffix,
    };
    let row = repo.create_step(step).await?;
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
pub async fn list_workflow_steps(
    State(state): State<AppState>,
    auth: auth_utils::AuthUser,
    Path(wid): Path<Uuid>,
) -> Result<Json<Vec<WorkflowStepResponse>>, AppError> {
    let repo = &state.repos().workflows;
    let wf = repo
        .get_workflow(wid)
        .await?
        .ok_or(AppError::not_found("Workflow"))?;
    if wf.user_id != auth.user_id.0 {
        return Err(AppError::not_found("Workflow"));
    }
    let rows = repo.list_steps(wid).await?;
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
pub async fn get_workflow_step(
    State(state): State<AppState>,
    auth: auth_utils::AuthUser,
    Path(p): Path<(Uuid, Uuid)>,
) -> Result<Json<WorkflowStepResponse>, AppError> {
    let repo = &state.repos().workflows;
    let wf = repo
        .get_workflow(p.0)
        .await?
        .ok_or(AppError::not_found("Workflow"))?;
    if wf.user_id != auth.user_id.0 {
        return Err(AppError::not_found("Workflow"));
    }
    let step = repo
        .get_step(p.1)
        .await?
        .ok_or(AppError::not_found("Step"))?;
    if step.workflow_id != p.0 {
        return Err(AppError::not_found("Step"));
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
) -> Result<Json<WorkflowStepResponse>, AppError> {
    let repo = &state.repos().workflows;
    let wf = repo
        .get_workflow(p.wid)
        .await?
        .ok_or(AppError::not_found("Workflow"))?;
    if wf.user_id != auth.user_id.0 {
        return Err(AppError::not_found("Workflow"));
    }
    let existing = repo
        .get_step(p.sid)
        .await?
        .ok_or(AppError::not_found("Step"))?;
    if existing.workflow_id != p.wid {
        return Err(AppError::not_found("Step"));
    }
    let step = crate::db::WorkflowStepRow {
        id: p.sid,
        workflow_id: p.wid,
        agent_id: req.agent_id.unwrap_or(existing.agent_id),
        execution_mode: req.execution_mode.unwrap_or(existing.execution_mode),
        agent_execution_mode: existing.agent_execution_mode, // Preserve existing value
        for_each_ref: req.for_each_ref.or(existing.for_each_ref),
        prompt_template_id: req.prompt_template_id.or(existing.prompt_template_id),
        prompt_template: req.prompt_template.unwrap_or(existing.prompt_template),
        output_schema_id: req.output_schema_id.or(existing.output_schema_id),
        output_variable_name: req.output_variable_name.or(existing.output_variable_name),
        interactive_agent_id: req.interactive_agent_id.or(existing.interactive_agent_id),
        for_each_label_field: req.for_each_label_field.or(existing.for_each_label_field),
        room_id: existing.room_id,
        routing_mode: existing.routing_mode,
        routing_field: existing.routing_field,
        display_order: req.display_order.unwrap_or(existing.display_order),
        version: existing.version,
        reasoning_trace: req.reasoning_trace.unwrap_or(existing.reasoning_trace),
        verification_agent_ids: req
            .verification_agent_ids
            .map(|ids| serde_json::to_value(ids).unwrap())
            .or(existing.verification_agent_ids),
        position_x: req.position_x.or(existing.position_x),
        position_y: req.position_y.or(existing.position_y),
        name: req.name.or(existing.name),
        system_prompt_suffix: req.system_prompt_suffix.or(existing.system_prompt_suffix),
    };
    let row = repo.update_step(step).await?;
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
pub async fn delete_workflow_step(
    State(state): State<AppState>,
    auth: auth_utils::AuthUser,
    Path(p): Path<WorkflowStepPath>,
) -> Result<StatusCode, AppError> {
    let repo = &state.repos().workflows;
    let wf = repo
        .get_workflow(p.wid)
        .await?
        .ok_or(AppError::not_found("Workflow"))?;
    if wf.user_id != auth.user_id.0 {
        return Err(AppError::not_found("Workflow"));
    }
    let existing = repo
        .get_step(p.sid)
        .await?
        .ok_or(AppError::not_found("Step"))?;
    if existing.workflow_id != p.wid {
        return Err(AppError::not_found("Step"));
    }
    repo.delete_step(p.sid).await?;
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
    let edge = repo.add_edge(wid, req.from_step_id, req.to_step_id).await?;
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
pub async fn add_step_document(
    State(state): State<AppState>,
    auth: auth_utils::AuthUser,
    Path(p): Path<WorkflowStepPath>,
    Json(req): Json<StepDocumentRequest>,
) -> Result<StatusCode, AppError> {
    let repo = &state.repos().workflows;
    let wf = repo
        .get_workflow(p.wid)
        .await?
        .ok_or(AppError::not_found("Workflow"))?;
    if wf.user_id != auth.user_id.0 {
        return Err(AppError::not_found("Workflow"));
    }
    let existing = repo
        .get_step(p.sid)
        .await?
        .ok_or(AppError::not_found("Step"))?;
    if existing.workflow_id != p.wid {
        return Err(AppError::not_found("Step"));
    }
    repo.add_step_document(p.sid, req.document_id).await?;
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
pub async fn remove_step_document(
    State(state): State<AppState>,
    auth: auth_utils::AuthUser,
    Path(p): Path<WorkflowStepPath>,
    Json(req): Json<StepDocumentRequest>,
) -> Result<StatusCode, AppError> {
    let repo = &state.repos().workflows;
    let wf = repo
        .get_workflow(p.wid)
        .await?
        .ok_or(AppError::not_found("Workflow"))?;
    if wf.user_id != auth.user_id.0 {
        return Err(AppError::not_found("Workflow"));
    }
    let existing = repo
        .get_step(p.sid)
        .await?
        .ok_or(AppError::not_found("Step"))?;
    if existing.workflow_id != p.wid {
        return Err(AppError::not_found("Step"));
    }
    repo.remove_step_document(p.sid, req.document_id).await?;
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
pub async fn list_step_documents(
    State(state): State<AppState>,
    auth: auth_utils::AuthUser,
    Path(p): Path<WorkflowStepPath>,
) -> Result<Json<Vec<StepDocumentResponse>>, AppError> {
    let repo = &state.repos().workflows;
    let wf = repo
        .get_workflow(p.wid)
        .await?
        .ok_or(AppError::not_found("Workflow"))?;
    if wf.user_id != auth.user_id.0 {
        return Err(AppError::not_found("Workflow"));
    }
    let rows = repo.list_step_documents(p.sid).await?;
    Ok(Json(
        rows.into_iter()
            .map(|r| StepDocumentResponse {
                step_id: r.step_id,
                document_id: r.document_id,
            })
            .collect(),
    ))
}

// ============================================================================
// Run Workflow Types
// ============================================================================

#[derive(Deserialize, utoipa::ToSchema)]
pub struct RunWorkflowRequest {
    pub initial_input: Option<String>,
}

#[derive(Serialize, utoipa::ToSchema)]
pub struct WorkflowRunResponse {
    pub execution_id: Uuid,
    pub workflow_id: Uuid,
    pub status: String,
}

// ============================================================================
// Run Workflow Handler
// ============================================================================

/// POST /api/workflows/:id/run - Execute a workflow directly (without a collection).
#[utoipa::path(
    post,
    path = "/api/workflows/{id}/run",
    tag = "Workflows",
    security(("bearer_auth" = [])),
    params(("id" = Uuid, Path, description = "Workflow ID")),
    request_body(content = Option<RunWorkflowRequest>, content_type = "application/json"),
    responses(
        (status = 202, description = "Workflow execution started", body = WorkflowRunResponse),
        (status = 404, description = "Workflow not found")
    )
)]
pub async fn run_workflow(
    State(state): State<AppState>,
    auth: auth_utils::AuthUser,
    Path(id): Path<Uuid>,
    body: Option<Json<RunWorkflowRequest>>,
) -> Result<(StatusCode, Json<WorkflowRunResponse>), AppError> {
    let workflow_repo = &state.repos().workflows;

    // Verify workflow exists and user owns it
    let workflow = workflow_repo
        .get_workflow(id)
        .await?
        .ok_or(AppError::not_found("Workflow"))?;
    if workflow.user_id != auth.user_id.0 {
        return Err(AppError::not_found("Workflow"));
    }

    // Create standalone workflow execution row
    let db = state
        .db()
        .ok_or(AppError::Internal("Database not available".into()))?
        .clone();
    let collection_repo: Arc<dyn WorkflowCollectionRepo> = Arc::new(PgRepo::new(db));
    let execution = collection_repo
        .create_standalone_workflow_execution(id, auth.user_id.0)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?;

    let execution_id = execution.id;

    // Load steps + edges
    let steps = workflow_repo.list_steps(id).await?;
    let edges = workflow_repo.list_edges(id).await?;

    if steps.is_empty() {
        return Err(AppError::bad_request("Workflow has no steps"));
    }

    // Build execution engine
    let provider = state
        .provider()
        .ok_or(AppError::Internal("LLM provider not configured".into()))?
        .clone();
    let engine = ExecutionEngine::new(provider);

    let initial_input = body
        .and_then(|b| b.0.initial_input)
        .unwrap_or_default();

    let mut prior_outputs = HashMap::new();
    if !initial_input.is_empty() {
        prior_outputs.insert(
            "input".to_string(),
            serde_json::Value::String(initial_input.clone()),
        );
    }

    let ctx = WorkflowExecutionContext {
        stage_execution_id: execution_id,
        run_id: execution_id,
        user_id: auth.user_id.0,
        initial_input,
        prior_outputs,
        execution_context: None,
        container_config: None,
        wg_client: None,
    };

    // Spawn execution in background (non-blocking, return 202)
    let bg_state = state.clone();
    let bg_collection_repo = collection_repo.clone();
    tokio::spawn(async move {
        // Mark as running
        let _ = bg_collection_repo
            .update_workflow_execution_status(execution_id, "running", None, None)
            .await;

        match execute_workflow_via_engine(&engine, &bg_state, &ctx, &steps, &edges, None).await {
            Ok(result) => {
                // Aggregate outputs
                let mut aggregated = serde_json::Map::new();
                for output in result.outputs.values() {
                    if let Some(structured) = &output.structured_output {
                        aggregated.insert(output.variable_name.clone(), structured.clone());
                    }
                }
                let outputs_json = serde_json::Value::Object(aggregated);

                let _ = bg_collection_repo
                    .update_workflow_execution_status(
                        execution_id,
                        "completed",
                        Some(outputs_json),
                        None,
                    )
                    .await;
            }
            Err(e) => {
                let _ = bg_collection_repo
                    .update_workflow_execution_status(
                        execution_id,
                        "failed",
                        None,
                        Some(e.to_string()),
                    )
                    .await;
            }
        }
    });

    Ok((
        StatusCode::ACCEPTED,
        Json(WorkflowRunResponse {
            execution_id,
            workflow_id: id,
            status: "pending".to_string(),
        }),
    ))
}

#[cfg(test)]
mod tests;
