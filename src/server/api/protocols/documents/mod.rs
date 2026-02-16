//! Protocol-scoped document definition management endpoints.
//!
//! CRUD for document definition templates attached to a protocol
//! (before being applied to any workflow step).

use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::server::api::AppError;
use crate::server::state::AppState;

#[cfg(test)]
mod tests;

// ============================================================================
// Types
// ============================================================================

#[derive(Deserialize)]
pub struct CreateProtocolDocDefRequest {
    pub name: String,
    #[serde(default)]
    pub description: String,
    #[serde(default = "default_target_length")]
    pub target_length: i32,
    #[serde(default)]
    pub display_order: i32,
}

#[derive(Deserialize)]
pub struct UpdateProtocolDocDefRequest {
    pub name: Option<String>,
    pub description: Option<String>,
    pub target_length: Option<i32>,
}

fn default_target_length() -> i32 {
    2000
}

#[derive(Serialize)]
pub struct ProtocolDocDefResponse {
    pub id: String,
    pub protocol_id: String,
    pub name: String,
    pub description: String,
    pub target_length: i32,
    pub display_order: i32,
    pub created_at: String,
}

impl ProtocolDocDefResponse {
    fn from_row(row: crate::db::ProtocolDocumentDefRow) -> Self {
        Self {
            id: row.id.to_string(),
            protocol_id: row.protocol_id.map(|id| id.to_string()).unwrap_or_default(),
            name: row.name,
            description: row.description,
            target_length: row.target_length,
            display_order: row.display_order,
            created_at: row.created_at.to_rfc3339(),
        }
    }
}

// ============================================================================
// Helpers
// ============================================================================

async fn verify_protocol_exists(state: &AppState, protocol_id: Uuid) -> Result<(), AppError> {
    state
        .repos()
        .protocols
        .get_protocol(protocol_id)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?
        .ok_or_else(|| AppError::not_found("protocol"))?;
    Ok(())
}

// ============================================================================
// Handlers
// ============================================================================

/// GET /api/protocols/:id/document-defs
pub async fn list_protocol_document_defs(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<Vec<ProtocolDocDefResponse>>, AppError> {
    verify_protocol_exists(&state, id).await?;
    let rows = state
        .repos()
        .protocols
        .list_protocol_document_defs(id)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?;
    Ok(Json(
        rows.into_iter()
            .map(ProtocolDocDefResponse::from_row)
            .collect(),
    ))
}

/// POST /api/protocols/:id/document-defs
pub async fn create_protocol_document_def(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    Json(req): Json<CreateProtocolDocDefRequest>,
) -> Result<(StatusCode, Json<ProtocolDocDefResponse>), AppError> {
    verify_protocol_exists(&state, id).await?;

    let def = crate::db::ProtocolDocumentDefRow {
        id: Uuid::new_v4(),
        step_id: None,
        name: req.name,
        description: req.description,
        target_length: req.target_length,
        display_order: req.display_order,
        created_at: chrono::Utc::now(),
        protocol_id: Some(id),
        document_id: None,
        agent_roster_entry_id: None,
    };

    let row = state
        .repos()
        .protocols
        .create_protocol_document_def(def)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?;
    Ok((
        StatusCode::CREATED,
        Json(ProtocolDocDefResponse::from_row(row)),
    ))
}

/// PUT /api/protocols/:pid/document-defs/:did
pub async fn update_protocol_document_def(
    State(state): State<AppState>,
    Path((pid, did)): Path<(Uuid, Uuid)>,
    Json(req): Json<UpdateProtocolDocDefRequest>,
) -> Result<Json<ProtocolDocDefResponse>, AppError> {
    verify_protocol_exists(&state, pid).await?;

    // Fetch existing to merge partial update
    let defs = state
        .repos()
        .protocols
        .list_protocol_document_defs(pid)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?;
    let existing = defs
        .into_iter()
        .find(|d| d.id == did)
        .ok_or_else(|| AppError::not_found("document definition"))?;

    let name = req.name.unwrap_or(existing.name);
    let description = req.description.unwrap_or(existing.description);
    let target_length = req.target_length.unwrap_or(existing.target_length);

    let row = state
        .repos()
        .protocols
        .update_protocol_document_def(did, name, description, target_length)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?;

    Ok(Json(ProtocolDocDefResponse::from_row(row)))
}

/// DELETE /api/protocols/:pid/document-defs/:did
pub async fn delete_protocol_document_def(
    State(state): State<AppState>,
    Path((pid, _did)): Path<(Uuid, Uuid)>,
) -> Result<StatusCode, AppError> {
    verify_protocol_exists(&state, pid).await?;
    state
        .repos()
        .protocols
        .delete_protocol_document_def(_did)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?;
    Ok(StatusCode::NO_CONTENT)
}
