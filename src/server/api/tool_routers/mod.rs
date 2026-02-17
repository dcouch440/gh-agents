//! Tool router CRUD and tool assignment endpoints

use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use serde::Deserialize;
use uuid::Uuid;

use super::AppError;
use crate::server::api::tools::ToolResponse;
use crate::server::auth as auth_utils;
use crate::server::services::tool_routers;
use crate::server::state::AppState;

/// Request body for creating a tool router.
#[derive(Deserialize, utoipa::ToSchema)]
pub struct CreateToolRouterRequest {
    pub name: String,
    pub description: Option<String>,
    pub system_prompt: String,
    pub model_id: String,
}

/// Request body for updating a tool router.
#[derive(Deserialize, utoipa::ToSchema)]
pub struct UpdateToolRouterRequest {
    pub name: Option<String>,
    pub description: Option<String>,
    pub system_prompt: Option<String>,
    pub model_id: Option<String>,
    pub is_active: Option<bool>,
}

/// Request body for setting router tools.
#[derive(Deserialize, utoipa::ToSchema)]
pub struct SetRouterToolsRequest {
    pub tool_ids: Vec<Uuid>,
}

/// GET /api/tool-routers - List all tool routers for the authenticated user.
#[utoipa::path(
    get,
    path = "/api/tool-routers",
    tag = "Tool Routers",
    security(("bearer_auth" = [])),
    responses(
        (status = 200, description = "List of tool routers")
    )
)]
pub async fn list_tool_routers(
    State(state): State<AppState>,
    auth: auth_utils::AuthUser,
) -> Result<Json<Vec<crate::db::ToolRouterRow>>, AppError> {
    let rows = tool_routers::list_tool_routers(state.repos().tool_routers.as_ref(), auth.user_id.0)
        .await?;
    Ok(Json(rows))
}

/// POST /api/tool-routers - Create a new tool router.
#[utoipa::path(
    post,
    path = "/api/tool-routers",
    tag = "Tool Routers",
    security(("bearer_auth" = [])),
    request_body = CreateToolRouterRequest,
    responses(
        (status = 201, description = "Tool router created"),
        (status = 400, description = "Invalid request")
    )
)]
pub async fn create_tool_router(
    State(state): State<AppState>,
    auth: auth_utils::AuthUser,
    Json(request): Json<CreateToolRouterRequest>,
) -> Result<(StatusCode, Json<crate::db::ToolRouterRow>), AppError> {
    let row = tool_routers::create_tool_router(
        state.repos().tool_routers.as_ref(),
        auth.user_id.0,
        &request.name,
        request.description,
        &request.system_prompt,
        &request.model_id,
    )
    .await?;
    Ok((StatusCode::CREATED, Json(row)))
}

/// GET /api/tool-routers/:id - Get a tool router by ID.
#[utoipa::path(
    get,
    path = "/api/tool-routers/{id}",
    tag = "Tool Routers",
    security(("bearer_auth" = [])),
    params(("id" = Uuid, Path, description = "Tool router ID")),
    responses(
        (status = 200, description = "Tool router found"),
        (status = 404, description = "Not found")
    )
)]
pub async fn get_tool_router(
    State(state): State<AppState>,
    auth: auth_utils::AuthUser,
    Path(id): Path<Uuid>,
) -> Result<Json<crate::db::ToolRouterRow>, AppError> {
    let row =
        tool_routers::get_tool_router(state.repos().tool_routers.as_ref(), auth.user_id.0, id)
            .await?;
    Ok(Json(row))
}

/// PUT /api/tool-routers/:id - Update a tool router.
#[utoipa::path(
    put,
    path = "/api/tool-routers/{id}",
    tag = "Tool Routers",
    security(("bearer_auth" = [])),
    params(("id" = Uuid, Path, description = "Tool router ID")),
    request_body = UpdateToolRouterRequest,
    responses(
        (status = 200, description = "Updated tool router"),
        (status = 404, description = "Not found")
    )
)]
pub async fn update_tool_router(
    State(state): State<AppState>,
    auth: auth_utils::AuthUser,
    Path(id): Path<Uuid>,
    Json(request): Json<UpdateToolRouterRequest>,
) -> Result<Json<crate::db::ToolRouterRow>, AppError> {
    let row = tool_routers::update_tool_router(
        state.repos().tool_routers.as_ref(),
        auth.user_id.0,
        id,
        request.name,
        request.description,
        request.system_prompt,
        request.model_id,
        request.is_active,
    )
    .await?;
    Ok(Json(row))
}

/// DELETE /api/tool-routers/:id - Delete a tool router.
#[utoipa::path(
    delete,
    path = "/api/tool-routers/{id}",
    tag = "Tool Routers",
    security(("bearer_auth" = [])),
    params(("id" = Uuid, Path, description = "Tool router ID")),
    responses(
        (status = 204, description = "Deleted successfully"),
        (status = 404, description = "Not found")
    )
)]
pub async fn delete_tool_router(
    State(state): State<AppState>,
    auth: auth_utils::AuthUser,
    Path(id): Path<Uuid>,
) -> Result<StatusCode, AppError> {
    tool_routers::delete_tool_router(state.repos().tool_routers.as_ref(), auth.user_id.0, id)
        .await?;
    Ok(StatusCode::NO_CONTENT)
}

/// GET /api/tool-routers/:id/tools - Get tools assigned to a router.
#[utoipa::path(
    get,
    path = "/api/tool-routers/{id}/tools",
    tag = "Tool Routers",
    security(("bearer_auth" = [])),
    params(("id" = Uuid, Path, description = "Tool router ID")),
    responses(
        (status = 200, description = "List of tools assigned to router", body = Vec<ToolResponse>),
        (status = 404, description = "Router not found")
    )
)]
pub async fn get_router_tools(
    State(state): State<AppState>,
    auth: auth_utils::AuthUser,
    Path(id): Path<Uuid>,
) -> Result<Json<Vec<ToolResponse>>, AppError> {
    let tools =
        tool_routers::get_router_tools(state.repos().tool_routers.as_ref(), auth.user_id.0, id)
            .await?;
    let tools = tools.into_iter().map(ToolResponse::from_row).collect();
    Ok(Json(tools))
}

/// PUT /api/tool-routers/:id/tools - Set tools for a router.
#[utoipa::path(
    put,
    path = "/api/tool-routers/{id}/tools",
    tag = "Tool Routers",
    security(("bearer_auth" = [])),
    params(("id" = Uuid, Path, description = "Tool router ID")),
    request_body = SetRouterToolsRequest,
    responses(
        (status = 204, description = "Router tools updated"),
        (status = 404, description = "Router not found")
    )
)]
pub async fn set_router_tools(
    State(state): State<AppState>,
    auth: auth_utils::AuthUser,
    Path(id): Path<Uuid>,
    Json(request): Json<SetRouterToolsRequest>,
) -> Result<StatusCode, AppError> {
    tool_routers::set_router_tools(
        state.repos().tool_routers.as_ref(),
        auth.user_id.0,
        id,
        &request.tool_ids,
    )
    .await?;
    Ok(StatusCode::NO_CONTENT)
}

#[cfg(test)]
mod tests;
