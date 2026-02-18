//! Tool management and agent-tool assignment endpoints

use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::AppError;
use crate::server::auth as auth_utils;
use crate::server::services::tools as svc;
use crate::server::state::AppState;

/// Response for a single tool
#[derive(Serialize, utoipa::ToSchema)]
pub struct ToolResponse {
    pub id: String,
    pub name: String,
    pub display_name: String,
    pub description: String,
    pub parameters: serde_json::Value,
    pub version: i32,
}

impl ToolResponse {
    pub fn from_row(row: crate::db::ToolRow) -> Self {
        Self {
            id: row.id.to_string(),
            name: row.name,
            display_name: row.display_name,
            description: row.description,
            parameters: row.parameters,
            version: row.version,
        }
    }
}

/// Request to create a new tool
#[derive(Deserialize, utoipa::ToSchema)]
pub struct CreateToolRequest {
    pub name: String,
    pub display_name: Option<String>,
    pub description: Option<String>,
    pub parameters: Option<serde_json::Value>,
}

/// Request to update an existing tool
#[derive(Deserialize, utoipa::ToSchema)]
pub struct UpdateToolRequest {
    pub name: Option<String>,
    pub display_name: Option<String>,
    pub description: Option<String>,
    pub parameters: Option<serde_json::Value>,
}

/// Request to set tools for an agent
#[derive(Deserialize, utoipa::ToSchema)]
pub struct SetAgentToolsRequest {
    pub tool_ids: Vec<String>,
}

/// Response for agent tools
#[derive(Serialize, utoipa::ToSchema)]
pub struct AgentToolsResponse {
    pub agent_id: String,
    pub tools: Vec<ToolResponse>,
}

/// List all tools
#[utoipa::path(
    get,
    path = "/api/tools",
    tag = "Tools",
    security(("bearer_auth" = [])),
    responses(
        (status = 200, description = "List of tools", body = Vec<ToolResponse>)
    )
)]
pub async fn list_tools(
    State(state): State<AppState>,
    _auth: auth_utils::AuthUser,
) -> Result<Json<Vec<ToolResponse>>, AppError> {
    let rows = svc::list_tools(state.repos().tools.as_ref()).await?;
    Ok(Json(rows.into_iter().map(ToolResponse::from_row).collect()))
}

/// Create a new tool
#[utoipa::path(
    post,
    path = "/api/tools",
    tag = "Tools",
    security(("bearer_auth" = [])),
    request_body = CreateToolRequest,
    responses(
        (status = 201, description = "Tool created", body = ToolResponse),
        (status = 400, description = "Invalid request")
    )
)]
pub async fn create_tool(
    State(state): State<AppState>,
    auth: auth_utils::AuthUser,
    Json(request): Json<CreateToolRequest>,
) -> Result<(StatusCode, Json<ToolResponse>), AppError> {
    let row = svc::create_tool(
        state.repos().tools.as_ref(),
        svc::CreateToolInput {
            is_admin: auth.claims.is_admin,
            name: request.name,
            display_name: request.display_name,
            description: request.description,
            parameters: request.parameters,
        },
    )
    .await?;
    Ok((StatusCode::CREATED, Json(ToolResponse::from_row(row))))
}

/// Get a single tool by ID
#[utoipa::path(
    get,
    path = "/api/tools/{id}",
    tag = "Tools",
    security(("bearer_auth" = [])),
    params(("id" = Uuid, Path, description = "Tool ID")),
    responses(
        (status = 200, description = "Tool found", body = ToolResponse),
        (status = 404, description = "Not found")
    )
)]
pub async fn get_tool(
    State(state): State<AppState>,
    _auth: auth_utils::AuthUser,
    Path(id): Path<Uuid>,
) -> Result<Json<ToolResponse>, AppError> {
    let row = svc::get_tool(state.repos().tools.as_ref(), id).await?;
    Ok(Json(ToolResponse::from_row(row)))
}

/// Update an existing tool (partial)
#[utoipa::path(
    patch,
    path = "/api/tools/{id}",
    tag = "Tools",
    security(("bearer_auth" = [])),
    params(("id" = Uuid, Path, description = "Tool ID")),
    request_body = UpdateToolRequest,
    responses(
        (status = 200, description = "Updated tool", body = ToolResponse),
        (status = 404, description = "Not found")
    )
)]
pub async fn update_tool(
    State(state): State<AppState>,
    auth: auth_utils::AuthUser,
    Path(id): Path<Uuid>,
    Json(request): Json<UpdateToolRequest>,
) -> Result<Json<ToolResponse>, AppError> {
    let row = svc::update_tool(
        state.repos().tools.as_ref(),
        svc::UpdateToolInput {
            is_admin: auth.claims.is_admin,
            tool_id: id,
            name: request.name,
            display_name: request.display_name,
            description: request.description,
            parameters: request.parameters,
        },
    )
    .await?;
    Ok(Json(ToolResponse::from_row(row)))
}

/// Delete a tool by ID
#[utoipa::path(
    delete,
    path = "/api/tools/{id}",
    tag = "Tools",
    security(("bearer_auth" = [])),
    params(("id" = Uuid, Path, description = "Tool ID")),
    responses(
        (status = 204, description = "Deleted successfully"),
        (status = 404, description = "Not found")
    )
)]
pub async fn delete_tool(
    State(state): State<AppState>,
    auth: auth_utils::AuthUser,
    Path(id): Path<Uuid>,
) -> Result<StatusCode, AppError> {
    svc::delete_tool(state.repos().tools.as_ref(), auth.claims.is_admin, id).await?;
    Ok(StatusCode::NO_CONTENT)
}

/// Get tools assigned to an agent
#[utoipa::path(
    get,
    path = "/api/agents/{id}/tools",
    tag = "Tools",
    security(("bearer_auth" = [])),
    params(("id" = Uuid, Path, description = "Agent ID")),
    responses(
        (status = 200, description = "Agent tools", body = AgentToolsResponse)
    )
)]
pub async fn get_agent_tools(
    State(state): State<AppState>,
    auth: auth_utils::AuthUser,
    Path(agent_id): Path<Uuid>,
) -> Result<Json<AgentToolsResponse>, AppError> {
    let tools = svc::get_agent_tools(
        state.repos().tools.as_ref(),
        state.repos().agents.as_ref(),
        auth.user_id.0,
        agent_id,
    )
    .await?;
    Ok(Json(AgentToolsResponse {
        agent_id: agent_id.to_string(),
        tools: tools.into_iter().map(ToolResponse::from_row).collect(),
    }))
}

/// Set tools for an agent (replaces existing)
#[utoipa::path(
    put,
    path = "/api/agents/{id}/tools",
    tag = "Tools",
    security(("bearer_auth" = [])),
    params(("id" = Uuid, Path, description = "Agent ID")),
    request_body = SetAgentToolsRequest,
    responses(
        (status = 200, description = "Agent tools updated", body = AgentToolsResponse),
        (status = 400, description = "Invalid tool IDs")
    )
)]
pub async fn set_agent_tools(
    State(state): State<AppState>,
    auth: auth_utils::AuthUser,
    Path(agent_id): Path<Uuid>,
    Json(request): Json<SetAgentToolsRequest>,
) -> Result<Json<AgentToolsResponse>, AppError> {
    let tools = svc::set_agent_tools(
        state.repos().tools.as_ref(),
        state.repos().agents.as_ref(),
        svc::SetAgentToolsInput {
            user_id: auth.user_id.0,
            agent_id,
            tool_ids: request.tool_ids,
        },
    )
    .await?;
    Ok(Json(AgentToolsResponse {
        agent_id: agent_id.to_string(),
        tools: tools.into_iter().map(ToolResponse::from_row).collect(),
    }))
}

#[cfg(test)]
mod tests;
