//! Tool management and agent-tool assignment endpoints

use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::server::auth as auth_utils;
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
    auth: auth_utils::AuthUser,
) -> Result<Json<Vec<ToolResponse>>, StatusCode> {
    let rows = state
        .repo
        .list_tools(auth.user_id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let tools = rows.into_iter().map(ToolResponse::from_row).collect();
    Ok(Json(tools))
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
) -> Result<(StatusCode, Json<ToolResponse>), StatusCode> {
    if request.name.trim().is_empty() {
        return Err(StatusCode::BAD_REQUEST);
    }

    let name = request.name.trim().to_string();
    let display_name = request.display_name.unwrap_or_else(|| name.clone());

    let row = crate::db::ToolRow {
        id: Uuid::new_v4(),
        user_id: auth.user_id.0,
        name,
        display_name,
        description: request.description.unwrap_or_default(),
        parameters: request.parameters.unwrap_or_else(|| serde_json::json!({})),
        created_at: chrono::Utc::now(),
        version: 1,
    };

    state
        .repo
        .upsert_tool(auth.user_id, row.clone())
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

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
) -> Result<Json<ToolResponse>, StatusCode> {
    let row = state
        .repo
        .get_tool(id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::NOT_FOUND)?;

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
) -> Result<Json<ToolResponse>, StatusCode> {
    let existing = state
        .repo
        .get_tool(id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::NOT_FOUND)?;

    let updated = crate::db::ToolRow {
        id: existing.id,
        user_id: existing.user_id,
        name: request.name.unwrap_or(existing.name),
        display_name: request.display_name.unwrap_or(existing.display_name),
        description: request.description.unwrap_or(existing.description),
        parameters: request.parameters.unwrap_or(existing.parameters),
        created_at: existing.created_at,
        version: existing.version,
    };

    state
        .repo
        .upsert_tool(auth.user_id, updated.clone())
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(ToolResponse::from_row(updated)))
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
    _auth: auth_utils::AuthUser,
    Path(id): Path<Uuid>,
) -> Result<StatusCode, StatusCode> {
    state
        .repo
        .delete_tool(id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

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
    _auth: auth_utils::AuthUser,
    Path(agent_id): Path<Uuid>,
) -> Result<Json<AgentToolsResponse>, StatusCode> {
    let rows = state
        .repo
        .get_agent_tools(agent_id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let tools = rows.into_iter().map(ToolResponse::from_row).collect();

    Ok(Json(AgentToolsResponse {
        agent_id: agent_id.to_string(),
        tools,
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
    _auth: auth_utils::AuthUser,
    Path(agent_id): Path<Uuid>,
    Json(request): Json<SetAgentToolsRequest>,
) -> Result<Json<AgentToolsResponse>, StatusCode> {
    let tool_ids: Result<Vec<Uuid>, _> = request
        .tool_ids
        .iter()
        .map(|s| Uuid::parse_str(s))
        .collect();

    let tool_ids = tool_ids.map_err(|_| StatusCode::BAD_REQUEST)?;

    state
        .repo
        .set_agent_tools(agent_id, tool_ids)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let rows = state
        .repo
        .get_agent_tools(agent_id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let tools = rows.into_iter().map(ToolResponse::from_row).collect();

    Ok(Json(AgentToolsResponse {
        agent_id: agent_id.to_string(),
        tools,
    }))
}
#[cfg(test)]
mod tests;
