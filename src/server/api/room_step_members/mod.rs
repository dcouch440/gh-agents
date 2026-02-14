//! Room step member listing endpoint (design-time configuration)

use axum::{
    extract::{Path, State},
    Json,
};
use serde::Serialize;
use uuid::Uuid;

use super::AppError;
use crate::server::auth as auth_utils;
use crate::server::state::AppState;

mod tests;

#[derive(Serialize, utoipa::ToSchema)]
pub struct RoomStepMemberResponse {
    pub id: String,
    pub name: String,
    pub role: String,
    pub perspective: String,
    pub display_order: i32,
}

impl RoomStepMemberResponse {
    fn from_row(row: crate::db::RoomStepMemberRow) -> Self {
        Self {
            id: row.id.to_string(),
            name: row.name,
            role: row.role,
            perspective: row.perspective,
            display_order: row.display_order,
        }
    }
}

/// GET /api/workflows/:wid/steps/:sid/room-members
#[utoipa::path(
    get,
    path = "/api/workflows/{wid}/steps/{sid}/room-members",
    tag = "Room Step Members",
    security(("bearer_auth" = [])),
    params(
        ("wid" = Uuid, Path, description = "Workflow ID"),
        ("sid" = Uuid, Path, description = "Step ID"),
    ),
    responses(
        (status = 200, description = "List of room step members", body = Vec<RoomStepMemberResponse>),
        (status = 404, description = "Not found")
    )
)]
pub async fn list_room_step_members(
    State(state): State<AppState>,
    auth: auth_utils::AuthUser,
    Path((wid, sid)): Path<(Uuid, Uuid)>,
) -> Result<Json<Vec<RoomStepMemberResponse>>, AppError> {
    let repo = &state.repos().workflows;
    let wf = repo
        .get_workflow(wid)
        .await?
        .ok_or(AppError::not_found("Workflow"))?;
    if wf.user_id != auth.user_id.0 {
        return Err(AppError::not_found("Workflow"));
    }
    let step = repo
        .get_step(sid)
        .await?
        .ok_or(AppError::not_found("Step"))?;
    if step.workflow_id != wid {
        return Err(AppError::not_found("Step"));
    }

    let members = repo.list_room_step_members(sid).await?;
    Ok(Json(
        members
            .into_iter()
            .map(RoomStepMemberResponse::from_row)
            .collect(),
    ))
}
