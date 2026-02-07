//! Routing rule management endpoints for workflow steps

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

// ============================================================================
// Types
// ============================================================================

#[derive(Deserialize, utoipa::ToSchema)]
pub struct CreateRoutingRuleRequest {
    pub label_value: String,
    pub agent_id: Uuid,
    pub description: Option<String>,
    #[serde(default)]
    pub display_order: i32,
}

#[derive(Deserialize, utoipa::ToSchema)]
pub struct UpdateRoutingRuleRequest {
    pub agent_id: Option<Uuid>,
    pub description: Option<String>,
    pub display_order: Option<i32>,
}

#[derive(Serialize, utoipa::ToSchema)]
pub struct RoutingRuleResponse {
    pub id: Uuid,
    pub label_value: String,
    pub description: Option<String>,
    pub agent_id: Uuid,
    pub display_order: i32,
}

#[derive(Deserialize)]
pub struct StepRulePath {
    pub wid: Uuid,
    pub sid: Uuid,
    pub rid: Uuid,
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
        .ok_or(AppError::not_found("Routing rule"))?;
    if wf.user_id != user_id {
        return Err(AppError::not_found("Routing rule"));
    }
    let step = repo
        .get_step(sid)
        .await?
        .ok_or(AppError::not_found("Routing rule"))?;
    if step.workflow_id != wid {
        return Err(AppError::not_found("Routing rule"));
    }
    Ok(())
}

// ============================================================================
// Handlers
// ============================================================================

/// GET /api/workflows/:wid/steps/:sid/routing-rules
#[utoipa::path(
    get,
    path = "/api/workflows/{wid}/steps/{sid}/routing-rules",
    tag = "Routing Rules",
    security(("bearer_auth" = [])),
    params(
        ("wid" = Uuid, Path, description = "Workflow ID"),
        ("sid" = Uuid, Path, description = "Step ID")
    ),
    responses(
        (status = 200, description = "List of routing rules", body = Vec<RoutingRuleResponse>),
        (status = 404, description = "Not found")
    )
)]
pub async fn list_routing_rules(
    State(state): State<AppState>,
    auth: auth_utils::AuthUser,
    Path((wid, sid)): Path<(Uuid, Uuid)>,
) -> Result<Json<Vec<RoutingRuleResponse>>, AppError> {
    verify_step_access(&state, wid, sid, auth.user_id.0).await?;
    let repo = &state.repos().workflows;
    let rows = repo
        .get_step_routing_rules(sid)
        .await
        ?;
    Ok(Json(
        rows.into_iter()
            .map(|r| RoutingRuleResponse {
                id: r.id,
                label_value: r.label_value,
                description: r.description,
                agent_id: r.agent_id,
                display_order: r.display_order,
            })
            .collect(),
    ))
}

/// POST /api/workflows/:wid/steps/:sid/routing-rules
#[utoipa::path(
    post,
    path = "/api/workflows/{wid}/steps/{sid}/routing-rules",
    tag = "Routing Rules",
    security(("bearer_auth" = [])),
    params(
        ("wid" = Uuid, Path, description = "Workflow ID"),
        ("sid" = Uuid, Path, description = "Step ID")
    ),
    request_body = CreateRoutingRuleRequest,
    responses(
        (status = 201, description = "Routing rule created", body = RoutingRuleResponse),
        (status = 400, description = "Invalid request"),
        (status = 404, description = "Not found")
    )
)]
pub async fn create_routing_rule(
    State(state): State<AppState>,
    auth: auth_utils::AuthUser,
    Path((wid, sid)): Path<(Uuid, Uuid)>,
    Json(req): Json<CreateRoutingRuleRequest>,
) -> Result<(StatusCode, Json<RoutingRuleResponse>), AppError> {
    if req.label_value.trim().is_empty() {
        return Err(AppError::bad_request("Label value is required"));
    }
    verify_step_access(&state, wid, sid, auth.user_id.0).await?;
    let repo = &state.repos().workflows;
    let row = repo
        .create_routing_rule(
            sid,
            &req.label_value,
            req.agent_id,
            req.description,
            req.display_order,
        )
        .await
        ?;
    Ok((
        StatusCode::CREATED,
        Json(RoutingRuleResponse {
            id: row.id,
            label_value: row.label_value,
            description: row.description,
            agent_id: row.agent_id,
            display_order: row.display_order,
        }),
    ))
}

/// PUT /api/workflows/:wid/steps/:sid/routing-rules/:rid
#[utoipa::path(
    put,
    path = "/api/workflows/{wid}/steps/{sid}/routing-rules/{rid}",
    tag = "Routing Rules",
    security(("bearer_auth" = [])),
    params(
        ("wid" = Uuid, Path, description = "Workflow ID"),
        ("sid" = Uuid, Path, description = "Step ID"),
        ("rid" = Uuid, Path, description = "Routing rule ID")
    ),
    request_body = UpdateRoutingRuleRequest,
    responses(
        (status = 200, description = "Updated routing rule", body = RoutingRuleResponse),
        (status = 404, description = "Not found")
    )
)]
pub async fn update_routing_rule(
    State(state): State<AppState>,
    auth: auth_utils::AuthUser,
    Path(p): Path<StepRulePath>,
    Json(req): Json<UpdateRoutingRuleRequest>,
) -> Result<Json<RoutingRuleResponse>, AppError> {
    verify_step_access(&state, p.wid, p.sid, auth.user_id.0).await?;
    let repo = &state.repos().workflows;
    let row = repo
        .update_routing_rule(p.rid, req.agent_id, req.description, req.display_order)
        .await
        ?;
    Ok(Json(RoutingRuleResponse {
        id: row.id,
        label_value: row.label_value,
        description: row.description,
        agent_id: row.agent_id,
        display_order: row.display_order,
    }))
}

/// DELETE /api/workflows/:wid/steps/:sid/routing-rules/:rid
#[utoipa::path(
    delete,
    path = "/api/workflows/{wid}/steps/{sid}/routing-rules/{rid}",
    tag = "Routing Rules",
    security(("bearer_auth" = [])),
    params(
        ("wid" = Uuid, Path, description = "Workflow ID"),
        ("sid" = Uuid, Path, description = "Step ID"),
        ("rid" = Uuid, Path, description = "Routing rule ID")
    ),
    responses(
        (status = 204, description = "Deleted successfully"),
        (status = 404, description = "Not found")
    )
)]
pub async fn delete_routing_rule(
    State(state): State<AppState>,
    auth: auth_utils::AuthUser,
    Path(p): Path<StepRulePath>,
) -> Result<StatusCode, AppError> {
    verify_step_access(&state, p.wid, p.sid, auth.user_id.0).await?;
    let repo = &state.repos().workflows;
    repo.delete_routing_rule(p.rid)
        .await
        ?;
    Ok(StatusCode::NO_CONTENT)
}

#[cfg(test)]
mod tests;
