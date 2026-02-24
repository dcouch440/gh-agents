//! Workforce agent roster management endpoints

use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::AppError;
use crate::server::auth as auth_utils;
use crate::server::services::agent_roster as svc;
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
    pub child_step_id: Option<String>,
    pub depends_on: Vec<String>,
}

impl RosterAgentResponse {
    fn from_row(row: crate::db::TaskAgentRosterRow, depends_on: Vec<String>) -> Self {
        Self {
            id: row.id.to_string(),
            child_step_id: row.child_step_id.map(|id| id.to_string()),
            name: row.name,
            role_description: row.role_description,
            capabilities: row.capabilities,
            execution_order: row.execution_order,
            created_at: row.created_at.to_rfc3339(),
            depends_on,
        }
    }
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
    let agents =
        svc::list_roster_agents(state.repos().workflows.as_ref(), auth.user_id.0, wid, sid).await?;

    let responses: Vec<RosterAgentResponse> = agents
        .into_iter()
        .map(|a| {
            let deps: Vec<String> = a.depends_on.iter().map(|id| id.to_string()).collect();
            RosterAgentResponse::from_row(a.agent, deps)
        })
        .collect();

    Ok(Json(responses))
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
    let row = svc::create_roster_agent(
        state.repos().workflows.as_ref(),
        svc::CreateRosterAgentInput {
            user_id: auth.user_id.0,
            workflow_id: wid,
            step_id: sid,
            name: req.name,
            role_description: req.role_description,
            capabilities: req.capabilities,
            execution_order: req.execution_order,
        },
    )
    .await?;

    state.broadcast_workflow(crate::server::ws::events::WorkflowEvent {
        run_id: None,
        workflow_id: wid,
        user_id: Some(auth.user_id.0),
        kind: crate::server::ws::events::WorkflowEventKind::RosterChanged { step_id: sid },
    });

    Ok((
        StatusCode::CREATED,
        Json(RosterAgentResponse::from_row(row, vec![])),
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
    let _info = svc::delete_roster_agent(
        state.repos().workflows.as_ref(),
        auth.user_id.0,
        wid,
        sid,
        rid,
    )
    .await?;

    state.broadcast_workflow(crate::server::ws::events::WorkflowEvent {
        run_id: None,
        workflow_id: wid,
        user_id: Some(auth.user_id.0),
        kind: crate::server::ws::events::WorkflowEventKind::RosterChanged { step_id: sid },
    });

    Ok(StatusCode::NO_CONTENT)
}
