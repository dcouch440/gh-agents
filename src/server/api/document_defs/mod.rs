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
use crate::server::services::document_defs as svc;
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
    pub agent_roster_entry_id: Option<String>,
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
            agent_roster_entry_id: row.agent_roster_entry_id.map(|id| id.to_string()),
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
    let rows =
        svc::list_document_defs(state.repos().workflows.as_ref(), auth.user_id.0, wid, sid).await?;
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
    let row = svc::create_document_def(
        state.repos().workflows.as_ref(),
        svc::CreateDocumentDefInput {
            user_id: auth.user_id.0,
            workflow_id: wid,
            step_id: sid,
            name: req.name,
            description: req.description,
            target_length: req.target_length,
            display_order: req.display_order,
        },
    )
    .await?;
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
    let row = svc::update_document_def(
        state.repos().workflows.as_ref(),
        svc::UpdateDocumentDefInput {
            user_id: auth.user_id.0,
            workflow_id: wid,
            step_id: sid,
            def_id: did,
            name: req.name,
            description: req.description,
            target_length: req.target_length,
        },
    )
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
    let info = svc::delete_document_def(
        state.repos().workflows.as_ref(),
        auth.user_id.0,
        wid,
        sid,
        did,
    )
    .await?;

    // Schedule consistency scan (requires AppState, so stays in handler)
    crate::server::hub::consistency_scanner::schedule_consistency_scan(
        state.clone(),
        wid,
        crate::server::hub::consistency_scanner::DeletedItem {
            item_type: crate::server::hub::consistency_scanner::DeletedItemType::DocumentDef,
            name: info.def_name,
            id: did,
            source_step_id: sid,
            source_step_name: info.step_name,
        },
    );

    Ok(StatusCode::NO_CONTENT)
}
