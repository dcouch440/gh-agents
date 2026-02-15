//! Protocol document definition management endpoints

use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::AppError;
use crate::server::auth as auth_utils;
use crate::server::state::AppState;

#[cfg(test)]
mod tests;

// ============================================================================
// Types
// ============================================================================

#[derive(Deserialize, utoipa::ToSchema)]
pub struct CreateDocumentDefRequest {
    pub name: String,
    #[serde(default)]
    pub description: String,
    #[serde(default = "default_target_length")]
    pub target_length: i32,
    #[serde(default)]
    pub display_order: i32,
}

#[derive(Deserialize, utoipa::ToSchema)]
pub struct UpdateDocumentDefRequest {
    pub name: Option<String>,
    pub description: Option<String>,
    pub target_length: Option<i32>,
}

fn default_target_length() -> i32 {
    2000
}

#[derive(Serialize, utoipa::ToSchema)]
pub struct DocumentDefResponse {
    pub id: String,
    pub step_id: String,
    pub name: String,
    pub description: String,
    pub target_length: i32,
    pub display_order: i32,
    pub created_at: String,
    pub document_id: Option<String>,
}

impl DocumentDefResponse {
    fn from_row(row: crate::db::ProtocolDocumentDefRow) -> Self {
        Self {
            id: row.id.to_string(),
            step_id: row.step_id.map(|id| id.to_string()).unwrap_or_default(),
            name: row.name,
            description: row.description,
            target_length: row.target_length,
            display_order: row.display_order,
            created_at: row.created_at.to_rfc3339(),
            document_id: row.document_id.map(|id| id.to_string()),
        }
    }
}

#[derive(Deserialize)]
pub struct DocDefPath {
    pub wid: Uuid,
    pub sid: Uuid,
    pub did: Uuid,
}

// ============================================================================
// Helpers
// ============================================================================

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
// Handlers
// ============================================================================

/// GET /api/workflows/:wid/steps/:sid/document-defs
#[utoipa::path(
    get,
    path = "/api/workflows/{wid}/steps/{sid}/document-defs",
    tag = "Document Definitions",
    security(("bearer_auth" = [])),
    params(
        ("wid" = Uuid, Path, description = "Workflow ID"),
        ("sid" = Uuid, Path, description = "Step ID"),
    ),
    responses(
        (status = 200, description = "List of document definitions", body = Vec<DocumentDefResponse>),
        (status = 404, description = "Not found")
    )
)]
pub async fn list_document_defs(
    State(state): State<AppState>,
    auth: auth_utils::AuthUser,
    Path((wid, sid)): Path<(Uuid, Uuid)>,
) -> Result<Json<Vec<DocumentDefResponse>>, AppError> {
    verify_step_access(&state, wid, sid, auth.user_id.0).await?;
    let rows = state.repos().workflows.list_document_defs(sid).await?;
    Ok(Json(
        rows.into_iter()
            .map(DocumentDefResponse::from_row)
            .collect(),
    ))
}

/// POST /api/workflows/:wid/steps/:sid/document-defs
#[utoipa::path(
    post,
    path = "/api/workflows/{wid}/steps/{sid}/document-defs",
    tag = "Document Definitions",
    security(("bearer_auth" = [])),
    params(
        ("wid" = Uuid, Path, description = "Workflow ID"),
        ("sid" = Uuid, Path, description = "Step ID"),
    ),
    request_body = CreateDocumentDefRequest,
    responses(
        (status = 201, description = "Document definition created", body = DocumentDefResponse),
        (status = 404, description = "Not found")
    )
)]
pub async fn create_document_def(
    State(state): State<AppState>,
    auth: auth_utils::AuthUser,
    Path((wid, sid)): Path<(Uuid, Uuid)>,
    Json(req): Json<CreateDocumentDefRequest>,
) -> Result<(StatusCode, Json<DocumentDefResponse>), AppError> {
    verify_step_access(&state, wid, sid, auth.user_id.0).await?;

    let def = crate::db::ProtocolDocumentDefRow {
        id: Uuid::new_v4(),
        step_id: Some(sid),
        name: req.name,
        description: req.description,
        target_length: req.target_length,
        display_order: req.display_order,
        created_at: chrono::Utc::now(),
        protocol_id: None,
        document_id: None,
    };

    let row = state.repos().workflows.create_document_def(def).await?;
    Ok((
        StatusCode::CREATED,
        Json(DocumentDefResponse::from_row(row)),
    ))
}

/// PATCH /api/workflows/:wid/steps/:sid/document-defs/:did
#[utoipa::path(
    patch,
    path = "/api/workflows/{wid}/steps/{sid}/document-defs/{did}",
    tag = "Document Definitions",
    security(("bearer_auth" = [])),
    params(
        ("wid" = Uuid, Path, description = "Workflow ID"),
        ("sid" = Uuid, Path, description = "Step ID"),
        ("did" = Uuid, Path, description = "Document Definition ID"),
    ),
    request_body = UpdateDocumentDefRequest,
    responses(
        (status = 200, description = "Document definition updated", body = DocumentDefResponse),
        (status = 404, description = "Not found")
    )
)]
pub async fn update_document_def(
    State(state): State<AppState>,
    auth: auth_utils::AuthUser,
    Path((wid, sid, did)): Path<(Uuid, Uuid, Uuid)>,
    Json(req): Json<UpdateDocumentDefRequest>,
) -> Result<Json<DocumentDefResponse>, AppError> {
    verify_step_access(&state, wid, sid, auth.user_id.0).await?;

    // Fetch existing to merge partial update
    let defs = state.repos().workflows.list_document_defs(sid).await?;
    let existing = defs
        .into_iter()
        .find(|d| d.id == did)
        .ok_or(AppError::not_found("Document Definition"))?;

    let name = req.name.unwrap_or(existing.name);
    let description = req.description.unwrap_or(existing.description);
    let target_length = req.target_length.unwrap_or(existing.target_length);

    let row = state
        .repos()
        .workflows
        .update_document_def(did, name, description, target_length)
        .await?;

    Ok(Json(DocumentDefResponse::from_row(row)))
}

/// DELETE /api/workflows/:wid/steps/:sid/document-defs/:did
#[utoipa::path(
    delete,
    path = "/api/workflows/{wid}/steps/{sid}/document-defs/{did}",
    tag = "Document Definitions",
    security(("bearer_auth" = [])),
    params(
        ("wid" = Uuid, Path, description = "Workflow ID"),
        ("sid" = Uuid, Path, description = "Step ID"),
        ("did" = Uuid, Path, description = "Document Definition ID"),
    ),
    responses(
        (status = 204, description = "Deleted"),
        (status = 404, description = "Not found")
    )
)]
pub async fn delete_document_def(
    State(state): State<AppState>,
    auth: auth_utils::AuthUser,
    Path((wid, sid, did)): Path<(Uuid, Uuid, Uuid)>,
) -> Result<StatusCode, AppError> {
    verify_step_access(&state, wid, sid, auth.user_id.0).await?;
    state.repos().workflows.delete_document_def(did).await?;
    Ok(StatusCode::NO_CONTENT)
}
