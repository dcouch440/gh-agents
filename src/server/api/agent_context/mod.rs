//! Agent context (document linkage) endpoints

use axum::{
    extract::{Path, State},
    Json,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::AppError;
use crate::server::auth as auth_utils;
use crate::server::services::agent_context as svc;
use crate::server::state::AppState;

use super::documents::DocumentListItem;

/// Request to set agent context documents
#[derive(Deserialize, utoipa::ToSchema)]
pub struct SetAgentContextRequest {
    pub document_ids: Vec<String>,
}

/// Response for agent context
#[derive(Serialize, utoipa::ToSchema)]
pub struct AgentContextResponse {
    pub agent_id: String,
    pub documents: Vec<DocumentListItem>,
}

fn rows_to_documents(rows: Vec<crate::db::DocumentRow>) -> Vec<DocumentListItem> {
    rows.into_iter()
        .map(|row| DocumentListItem {
            id: row.id,
            title: row.title,
            summary: row.summary,
            ref_tag: row.ref_tag,
            tags: row.tags,
            doc_type: row.doc_type,
            updated_at: row.updated_at,
        })
        .collect()
}

/// Get context documents assigned to an agent
#[utoipa::path(
    get,
    path = "/api/agents/{id}/context",
    tag = "Agent Context",
    security(("bearer_auth" = [])),
    params(("id" = Uuid, Path, description = "Agent ID")),
    responses(
        (status = 200, description = "Agent context documents", body = AgentContextResponse)
    )
)]
pub async fn get_agent_context(
    State(state): State<AppState>,
    _auth: auth_utils::AuthUser,
    Path(agent_id): Path<Uuid>,
) -> Result<Json<AgentContextResponse>, AppError> {
    let rows = svc::get_agent_context(state.repo().as_ref(), agent_id).await?;
    Ok(Json(AgentContextResponse {
        agent_id: agent_id.to_string(),
        documents: rows_to_documents(rows),
    }))
}

/// Set context documents for an agent (replaces existing)
#[utoipa::path(
    put,
    path = "/api/agents/{id}/context",
    tag = "Agent Context",
    security(("bearer_auth" = [])),
    params(("id" = Uuid, Path, description = "Agent ID")),
    request_body = SetAgentContextRequest,
    responses(
        (status = 200, description = "Agent context updated", body = AgentContextResponse),
        (status = 400, description = "Invalid document IDs")
    )
)]
pub async fn set_agent_context(
    State(state): State<AppState>,
    _auth: auth_utils::AuthUser,
    Path(agent_id): Path<Uuid>,
    Json(request): Json<SetAgentContextRequest>,
) -> Result<Json<AgentContextResponse>, AppError> {
    let rows = svc::set_agent_context(
        state.repo().as_ref(),
        svc::SetAgentContextInput {
            agent_id,
            document_ids: request.document_ids,
        },
    )
    .await?;
    Ok(Json(AgentContextResponse {
        agent_id: agent_id.to_string(),
        documents: rows_to_documents(rows),
    }))
}

#[cfg(test)]
mod tests;
