//! Step input and output port management endpoints

use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::AppError;
use crate::db::traits::CreateStepInputPort;
use crate::server::auth as auth_utils;
use crate::server::state::AppState;

// ============================================================================
// Types
// ============================================================================

#[derive(Deserialize, utoipa::ToSchema)]
pub struct CreateStepInputRequest {
    pub port_name: String,
    #[serde(default = "default_port_type")]
    pub port_type: String,
    #[serde(default)]
    pub required: bool,
    pub default_value: Option<serde_json::Value>,
    pub description: Option<String>,
    pub json_schema: Option<serde_json::Value>,
}

#[derive(Deserialize, utoipa::ToSchema)]
pub struct CreateStepOutputRequest {
    pub port_name: String,
    #[serde(default = "default_port_type")]
    pub port_type: String,
    pub json_path: String,
    pub description: Option<String>,
    pub json_schema: Option<serde_json::Value>,
}

fn default_port_type() -> String {
    "string".to_string()
}

#[derive(Serialize, utoipa::ToSchema)]
pub struct StepInputResponse {
    pub id: Uuid,
    pub port_name: String,
    pub port_type: String,
    pub required: bool,
    pub default_value: Option<serde_json::Value>,
    pub description: Option<String>,
    pub json_schema: Option<serde_json::Value>,
}

#[derive(Serialize, utoipa::ToSchema)]
pub struct StepOutputResponse {
    pub id: Uuid,
    pub port_name: String,
    pub port_type: String,
    pub json_path: String,
    pub description: Option<String>,
    pub json_schema: Option<serde_json::Value>,
}

#[derive(Deserialize)]
pub struct StepPortPath {
    pub wid: Uuid,
    pub sid: Uuid,
    pub pid: Uuid,
}

// ============================================================================
// Helpers
// ============================================================================

/// Verify workflow ownership and step membership. Returns step ID on success.
async fn verify_step_access(
    state: &AppState,
    wid: Uuid,
    sid: Uuid,
    user_id: Uuid,
) -> Result<(), AppError> {
    let repo = &state.repos().workflows;
    let wf = repo
        .get_workflow(wid)
        .await?
        .ok_or(AppError::not_found("Workflow"))?;
    if wf.user_id != user_id {
        return Err(AppError::not_found("Workflow"));
    }
    let step = repo
        .get_step(sid)
        .await?
        .ok_or(AppError::not_found("Step"))?;
    if step.workflow_id != wid {
        return Err(AppError::not_found("Step"));
    }
    Ok(())
}

// ============================================================================
// Input Port Handlers
// ============================================================================

/// GET /api/workflows/:wid/steps/:sid/inputs
#[utoipa::path(
    get,
    path = "/api/workflows/{wid}/steps/{sid}/inputs",
    tag = "Step Ports",
    security(("bearer_auth" = [])),
    params(
        ("wid" = Uuid, Path, description = "Workflow ID"),
        ("sid" = Uuid, Path, description = "Step ID")
    ),
    responses(
        (status = 200, description = "List of input ports", body = Vec<StepInputResponse>),
        (status = 404, description = "Not found")
    )
)]
pub async fn list_step_inputs(
    State(state): State<AppState>,
    auth: auth_utils::AuthUser,
    Path((wid, sid)): Path<(Uuid, Uuid)>,
) -> Result<Json<Vec<StepInputResponse>>, AppError> {
    verify_step_access(&state, wid, sid, auth.user_id.0).await?;
    let repo = &state.repos().workflows;
    let rows = repo.get_step_inputs(sid).await?;
    Ok(Json(
        rows.into_iter()
            .map(|r| StepInputResponse {
                id: r.id,
                port_name: r.port_name,
                port_type: r.port_type,
                required: r.required,
                default_value: r.default_value,
                description: r.description,
                json_schema: r.json_schema,
            })
            .collect(),
    ))
}

/// POST /api/workflows/:wid/steps/:sid/inputs
#[utoipa::path(
    post,
    path = "/api/workflows/{wid}/steps/{sid}/inputs",
    tag = "Step Ports",
    security(("bearer_auth" = [])),
    params(
        ("wid" = Uuid, Path, description = "Workflow ID"),
        ("sid" = Uuid, Path, description = "Step ID")
    ),
    request_body = CreateStepInputRequest,
    responses(
        (status = 201, description = "Input port created", body = StepInputResponse),
        (status = 400, description = "Invalid request"),
        (status = 404, description = "Not found")
    )
)]
pub async fn create_step_input(
    State(state): State<AppState>,
    auth: auth_utils::AuthUser,
    Path((wid, sid)): Path<(Uuid, Uuid)>,
    Json(req): Json<CreateStepInputRequest>,
) -> Result<(StatusCode, Json<StepInputResponse>), AppError> {
    if req.port_name.trim().is_empty() {
        return Err(AppError::bad_request("Port name must not be empty"));
    }
    verify_step_access(&state, wid, sid, auth.user_id.0).await?;
    let repo = &state.repos().workflows;
    let row = repo
        .create_step_input(CreateStepInputPort {
            workflow_step_id: sid,
            port_name: req.port_name,
            port_type: req.port_type,
            required: req.required,
            default_value: req.default_value,
            description: req.description,
            json_schema: req.json_schema,
        })
        .await?;
    Ok((
        StatusCode::CREATED,
        Json(StepInputResponse {
            id: row.id,
            port_name: row.port_name,
            port_type: row.port_type,
            required: row.required,
            default_value: row.default_value,
            description: row.description,
            json_schema: row.json_schema,
        }),
    ))
}

/// DELETE /api/workflows/:wid/steps/:sid/inputs/:pid
#[utoipa::path(
    delete,
    path = "/api/workflows/{wid}/steps/{sid}/inputs/{pid}",
    tag = "Step Ports",
    security(("bearer_auth" = [])),
    params(
        ("wid" = Uuid, Path, description = "Workflow ID"),
        ("sid" = Uuid, Path, description = "Step ID"),
        ("pid" = Uuid, Path, description = "Port ID")
    ),
    responses(
        (status = 204, description = "Deleted successfully"),
        (status = 404, description = "Not found")
    )
)]
pub async fn delete_step_input(
    State(state): State<AppState>,
    auth: auth_utils::AuthUser,
    Path(p): Path<StepPortPath>,
) -> Result<StatusCode, AppError> {
    verify_step_access(&state, p.wid, p.sid, auth.user_id.0).await?;
    let repo = &state.repos().workflows;
    repo.delete_step_input(p.pid).await?;
    Ok(StatusCode::NO_CONTENT)
}

// ============================================================================
// Output Port Handlers
// ============================================================================

/// GET /api/workflows/:wid/steps/:sid/outputs
#[utoipa::path(
    get,
    path = "/api/workflows/{wid}/steps/{sid}/outputs",
    tag = "Step Ports",
    security(("bearer_auth" = [])),
    params(
        ("wid" = Uuid, Path, description = "Workflow ID"),
        ("sid" = Uuid, Path, description = "Step ID")
    ),
    responses(
        (status = 200, description = "List of output ports", body = Vec<StepOutputResponse>),
        (status = 404, description = "Not found")
    )
)]
pub async fn list_step_outputs(
    State(state): State<AppState>,
    auth: auth_utils::AuthUser,
    Path((wid, sid)): Path<(Uuid, Uuid)>,
) -> Result<Json<Vec<StepOutputResponse>>, AppError> {
    verify_step_access(&state, wid, sid, auth.user_id.0).await?;
    let repo = &state.repos().workflows;
    let rows = repo.get_step_outputs(sid).await?;
    Ok(Json(
        rows.into_iter()
            .map(|r| StepOutputResponse {
                id: r.id,
                port_name: r.port_name,
                port_type: r.port_type,
                json_path: r.json_path,
                description: r.description,
                json_schema: r.json_schema,
            })
            .collect(),
    ))
}

/// POST /api/workflows/:wid/steps/:sid/outputs
#[utoipa::path(
    post,
    path = "/api/workflows/{wid}/steps/{sid}/outputs",
    tag = "Step Ports",
    security(("bearer_auth" = [])),
    params(
        ("wid" = Uuid, Path, description = "Workflow ID"),
        ("sid" = Uuid, Path, description = "Step ID")
    ),
    request_body = CreateStepOutputRequest,
    responses(
        (status = 201, description = "Output port created", body = StepOutputResponse),
        (status = 400, description = "Invalid request"),
        (status = 404, description = "Not found")
    )
)]
pub async fn create_step_output(
    State(state): State<AppState>,
    auth: auth_utils::AuthUser,
    Path((wid, sid)): Path<(Uuid, Uuid)>,
    Json(req): Json<CreateStepOutputRequest>,
) -> Result<(StatusCode, Json<StepOutputResponse>), AppError> {
    if req.port_name.trim().is_empty() || req.json_path.trim().is_empty() {
        return Err(AppError::bad_request(
            "Port name and json_path must not be empty",
        ));
    }
    verify_step_access(&state, wid, sid, auth.user_id.0).await?;
    let repo = &state.repos().workflows;
    let row = repo
        .create_step_output(
            sid,
            &req.port_name,
            &req.port_type,
            &req.json_path,
            req.description,
            req.json_schema,
        )
        .await?;
    Ok((
        StatusCode::CREATED,
        Json(StepOutputResponse {
            id: row.id,
            port_name: row.port_name,
            port_type: row.port_type,
            json_path: row.json_path,
            description: row.description,
            json_schema: row.json_schema,
        }),
    ))
}

/// DELETE /api/workflows/:wid/steps/:sid/outputs/:pid
#[utoipa::path(
    delete,
    path = "/api/workflows/{wid}/steps/{sid}/outputs/{pid}",
    tag = "Step Ports",
    security(("bearer_auth" = [])),
    params(
        ("wid" = Uuid, Path, description = "Workflow ID"),
        ("sid" = Uuid, Path, description = "Step ID"),
        ("pid" = Uuid, Path, description = "Port ID")
    ),
    responses(
        (status = 204, description = "Deleted successfully"),
        (status = 404, description = "Not found")
    )
)]
pub async fn delete_step_output(
    State(state): State<AppState>,
    auth: auth_utils::AuthUser,
    Path(p): Path<StepPortPath>,
) -> Result<StatusCode, AppError> {
    verify_step_access(&state, p.wid, p.sid, auth.user_id.0).await?;
    let repo = &state.repos().workflows;
    repo.delete_step_output(p.pid).await?;
    Ok(StatusCode::NO_CONTENT)
}

#[cfg(test)]
mod tests;
