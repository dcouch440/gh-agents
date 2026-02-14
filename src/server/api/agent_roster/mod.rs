//! Task force agent roster management endpoints

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
pub struct CreateRosterAgentRequest {
    pub name: String,
    #[serde(default)]
    pub role_description: String,
    #[serde(default)]
    pub capabilities: Vec<String>,
    #[serde(default)]
    pub execution_order: i32,
}

#[derive(Serialize, utoipa::ToSchema)]
pub struct RosterAgentResponse {
    pub id: String,
    pub name: String,
    pub role_description: String,
    pub capabilities: Vec<String>,
    pub execution_order: i32,
    pub created_at: String,
}

impl RosterAgentResponse {
    fn from_row(row: crate::db::TaskAgentRosterRow) -> Self {
        Self {
            id: row.id.to_string(),
            name: row.name,
            role_description: row.role_description,
            capabilities: row.capabilities,
            execution_order: row.execution_order,
            created_at: row.created_at.to_rfc3339(),
        }
    }
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

/// GET /api/workflows/:wid/steps/:sid/agent-roster
#[utoipa::path(
    get,
    path = "/api/workflows/{wid}/steps/{sid}/agent-roster",
    tag = "Agent Roster",
    security(("bearer_auth" = [])),
    params(
        ("wid" = Uuid, Path, description = "Workflow ID"),
        ("sid" = Uuid, Path, description = "Step ID"),
    ),
    responses(
        (status = 200, description = "List of roster agents", body = Vec<RosterAgentResponse>),
        (status = 404, description = "Not found")
    )
)]
pub async fn list_roster_agents(
    State(state): State<AppState>,
    auth: auth_utils::AuthUser,
    Path((wid, sid)): Path<(Uuid, Uuid)>,
) -> Result<Json<Vec<RosterAgentResponse>>, AppError> {
    verify_step_access(&state, wid, sid, auth.user_id.0).await?;

    let repo = &state.repos().workflows;
    let brief = repo.get_mission_brief(sid).await?;

    let agents = match brief {
        Some(b) => repo.list_agent_roster(b.id).await?,
        None => vec![],
    };

    Ok(Json(
        agents
            .into_iter()
            .map(RosterAgentResponse::from_row)
            .collect(),
    ))
}

/// POST /api/workflows/:wid/steps/:sid/agent-roster
#[utoipa::path(
    post,
    path = "/api/workflows/{wid}/steps/{sid}/agent-roster",
    tag = "Agent Roster",
    security(("bearer_auth" = [])),
    params(
        ("wid" = Uuid, Path, description = "Workflow ID"),
        ("sid" = Uuid, Path, description = "Step ID"),
    ),
    request_body = CreateRosterAgentRequest,
    responses(
        (status = 201, description = "Roster agent created", body = RosterAgentResponse),
        (status = 404, description = "Not found")
    )
)]
pub async fn create_roster_agent(
    State(state): State<AppState>,
    auth: auth_utils::AuthUser,
    Path((wid, sid)): Path<(Uuid, Uuid)>,
    Json(req): Json<CreateRosterAgentRequest>,
) -> Result<(StatusCode, Json<RosterAgentResponse>), AppError> {
    verify_step_access(&state, wid, sid, auth.user_id.0).await?;

    let repo = &state.repos().workflows;

    // Ensure a mission brief exists — upsert with defaults if not
    let brief = repo
        .upsert_mission_brief(sid, "", &[], "fail_fast", None)
        .await?;

    let row = repo
        .add_roster_agent(
            brief.id,
            &req.name,
            &req.role_description,
            &req.capabilities,
            req.execution_order,
        )
        .await?;

    Ok((
        StatusCode::CREATED,
        Json(RosterAgentResponse::from_row(row)),
    ))
}

/// DELETE /api/workflows/:wid/steps/:sid/agent-roster/:rid
#[utoipa::path(
    delete,
    path = "/api/workflows/{wid}/steps/{sid}/agent-roster/{rid}",
    tag = "Agent Roster",
    security(("bearer_auth" = [])),
    params(
        ("wid" = Uuid, Path, description = "Workflow ID"),
        ("sid" = Uuid, Path, description = "Step ID"),
        ("rid" = Uuid, Path, description = "Roster Agent ID"),
    ),
    responses(
        (status = 204, description = "Deleted"),
        (status = 404, description = "Not found")
    )
)]
pub async fn delete_roster_agent(
    State(state): State<AppState>,
    auth: auth_utils::AuthUser,
    Path((wid, sid, rid)): Path<(Uuid, Uuid, Uuid)>,
) -> Result<StatusCode, AppError> {
    verify_step_access(&state, wid, sid, auth.user_id.0).await?;
    state.repos().workflows.remove_roster_agent(rid).await?;
    Ok(StatusCode::NO_CONTENT)
}
