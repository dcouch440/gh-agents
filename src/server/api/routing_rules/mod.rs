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
use crate::server::services::routing_rules as svc;
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
    let rows =
        svc::list_routing_rules(state.repos().workflows.as_ref(), auth.user_id.0, wid, sid).await?;
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
    let row = svc::create_routing_rule(
        state.repos().workflows.as_ref(),
        svc::CreateRoutingRuleInput {
            user_id: auth.user_id.0,
            workflow_id: wid,
            step_id: sid,
            label_value: req.label_value,
            agent_id: req.agent_id,
            description: req.description,
            display_order: req.display_order,
        },
    )
    .await?;
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
    let row = svc::update_routing_rule(
        state.repos().workflows.as_ref(),
        svc::UpdateRoutingRuleInput {
            user_id: auth.user_id.0,
            workflow_id: p.wid,
            step_id: p.sid,
            rule_id: p.rid,
            agent_id: req.agent_id,
            description: req.description,
            display_order: req.display_order,
        },
    )
    .await?;
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
    svc::delete_routing_rule(
        state.repos().workflows.as_ref(),
        auth.user_id.0,
        p.wid,
        p.sid,
        p.rid,
    )
    .await?;
    Ok(StatusCode::NO_CONTENT)
}

#[cfg(test)]
mod tests;
