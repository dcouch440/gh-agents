//! Protocol execution observability endpoint.
//!
//! Lists the execution audit trail for a protocol step, showing
//! strategy, research, and write phases with their status and output.

use axum::{
    extract::{Path, State},
    Json,
};
use serde::Serialize;
use uuid::Uuid;

use crate::server::api::AppError;
use crate::server::state::AppState;

#[cfg(test)]
mod tests;

// ============================================================================
// Types
// ============================================================================

#[derive(Serialize)]
pub struct ProtocolExecutionResponse {
    pub id: String,
    pub protocol_step_id: String,
    pub workflow_run_id: Option<String>,
    pub phase: String,
    pub document_def_id: Option<String>,
    pub agent_id: Option<String>,
    pub input_prompt: Option<String>,
    pub output_content: Option<String>,
    pub status: String,
    pub error_message: Option<String>,
    pub tokens_in: Option<i32>,
    pub tokens_out: Option<i32>,
    pub cost_usd: Option<f64>,
    pub model: Option<String>,
    pub capabilities_used: Option<Vec<String>>,
    pub created_at: String,
    pub completed_at: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub agent_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub archetype: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub designer_run_id: Option<String>,
}

impl ProtocolExecutionResponse {
    fn from_row(row: crate::db::ProtocolExecutionRow) -> Self {
        Self {
            id: row.id.to_string(),
            protocol_step_id: row.protocol_step_id.to_string(),
            workflow_run_id: row.workflow_run_id.map(|id| id.to_string()),
            phase: row.phase,
            document_def_id: row.document_def_id.map(|id| id.to_string()),
            agent_id: row.agent_id.map(|id| id.to_string()),
            input_prompt: row.input_prompt,
            output_content: row.output_content,
            status: row.status,
            error_message: row.error_message,
            tokens_in: row.tokens_in,
            tokens_out: row.tokens_out,
            cost_usd: row.cost_usd,
            model: row.model,
            capabilities_used: row.capabilities_used,
            created_at: row.created_at.to_rfc3339(),
            completed_at: row.completed_at.map(|t| t.to_rfc3339()),
            agent_name: row.agent_name,
            archetype: row.archetype,
            designer_run_id: row.designer_run_id.map(|id| id.to_string()),
        }
    }
}

// ============================================================================
// Handlers
// ============================================================================

/// GET /api/protocols/:id/executions
///
/// Lists protocol execution records for a given step (protocol_step_id).
pub async fn list_protocol_executions(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<Vec<ProtocolExecutionResponse>>, AppError> {
    let rows = state
        .repos()
        .protocols
        .list_protocol_executions_by_step(id)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?;
    Ok(Json(
        rows.into_iter()
            .map(ProtocolExecutionResponse::from_row)
            .collect(),
    ))
}
