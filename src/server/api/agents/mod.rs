//! Agent management endpoints

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    Json,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::AppError;
use crate::server::auth as auth_utils;
use crate::server::services::agents;
use crate::server::state::AppState;

/// Response for a single agent
#[derive(Serialize, utoipa::ToSchema)]
pub struct AgentResponse {
    pub id: String,
    pub name: String,
    pub system_prompt: String,
    pub persona_style: String,
    pub model_provider: String,
    pub model_id: String,
    pub model_max_tokens: i32,
    pub model_temperature: f32,
    pub status: String,
    pub output_schema_id: Option<String>,
    pub router_id: Option<String>,
    pub version: i32,
    pub default_reasoning_trace: bool,
    pub is_system: bool,
}

/// Query parameters for the agent list endpoint.
#[derive(Deserialize)]
pub struct ListAgentsQuery {
    #[serde(default)]
    pub include_system: bool,
}

impl AgentResponse {
    fn from_row(row: crate::db::AgentRow) -> Self {
        Self {
            id: row.id.to_string(),
            name: row.name,
            system_prompt: row.system_prompt,
            persona_style: row.persona_style.unwrap_or_else(|| "casual".to_string()),
            model_provider: row.model_provider,
            model_id: row.model_id,
            model_max_tokens: row.model_max_tokens,
            model_temperature: row.model_temperature,
            status: row.status.unwrap_or_else(|| "idle".to_string()),
            output_schema_id: row.output_schema_id.map(|id| id.to_string()),
            router_id: row.router_id.map(|id| id.to_string()),
            version: row.version,
            default_reasoning_trace: row.default_reasoning_trace.unwrap_or(false),
            is_system: row.is_system,
        }
    }
}

/// Response for the agents list endpoint
#[derive(Serialize, utoipa::ToSchema)]
pub struct AgentsListResponse {
    pub agents: Vec<AgentResponse>,
    pub stats: AgentPoolStats,
}

/// Agent pool statistics
#[derive(Serialize, utoipa::ToSchema)]
pub struct AgentPoolStats {
    pub total: usize,
    pub available: usize,
    pub max: u8,
}

/// Request to create a new agent
#[derive(Deserialize, utoipa::ToSchema)]
pub struct CreateAgentRequest {
    pub name: String,
    pub system_prompt: Option<String>,
    pub persona_style: Option<String>,
    pub model_provider: Option<String>,
    pub model_id: String,
    pub model_max_tokens: Option<i32>,
    pub model_temperature: Option<f32>,
    #[serde(default)]
    pub output_schema_id: Option<Uuid>,
}

/// Request to update an existing agent
#[derive(Deserialize, utoipa::ToSchema)]
pub struct UpdateAgentRequest {
    pub name: Option<String>,
    pub system_prompt: Option<String>,
    pub persona_style: Option<String>,
    pub model_provider: Option<String>,
    pub model_id: Option<String>,
    pub model_max_tokens: Option<i32>,
    pub model_temperature: Option<f32>,
    #[serde(default)]
    pub output_schema_id: Option<Uuid>,
    #[serde(default)]
    pub router_id: Option<Uuid>,
}

/// List all agents and their status
#[utoipa::path(
    get,
    path = "/api/agents",
    tag = "Agents",
    security(("bearer_auth" = [])),
    responses(
        (status = 200, description = "List of agents with pool stats", body = AgentsListResponse)
    )
)]
pub async fn list_agents(
    State(state): State<AppState>,
    auth: auth_utils::AuthUser,
    Query(query): Query<ListAgentsQuery>,
) -> Result<Json<AgentsListResponse>, AppError> {
    let config = state.config().read().await;
    let pool_config = &config.pool;

    let rows = agents::list_agents(state.repo().as_ref(), auth.user_id).await?;

    let agents: Vec<AgentResponse> = rows
        .into_iter()
        .filter(|r| query.include_system || !r.is_system)
        .map(AgentResponse::from_row)
        .collect();

    let response = AgentsListResponse {
        stats: AgentPoolStats {
            total: agents.len(),
            available: agents.iter().filter(|a| a.status == "idle").count(),
            max: pool_config.max_agents,
        },
        agents,
    };

    Ok(Json(response))
}

/// Create a new agent
#[utoipa::path(
    post,
    path = "/api/agents",
    tag = "Agents",
    security(("bearer_auth" = [])),
    request_body = CreateAgentRequest,
    responses(
        (status = 201, description = "Agent created", body = AgentResponse),
        (status = 400, description = "Invalid request")
    )
)]
pub async fn create_agent(
    State(state): State<AppState>,
    auth: auth_utils::AuthUser,
    Json(request): Json<CreateAgentRequest>,
) -> Result<(StatusCode, Json<AgentResponse>), AppError> {
    let row = agents::create_agent(
        state.repo().as_ref(),
        agents::CreateAgentInput {
            user_id: auth.user_id.0,
            name: request.name,
            system_prompt: request.system_prompt,
            persona_style: request.persona_style,
            model_provider: request.model_provider,
            model_id: request.model_id,
            model_max_tokens: request.model_max_tokens,
            model_temperature: request.model_temperature,
            output_schema_id: request.output_schema_id,
        },
    )
    .await?;

    Ok((StatusCode::CREATED, Json(AgentResponse::from_row(row))))
}

/// Get a single agent by ID
#[utoipa::path(
    get,
    path = "/api/agents/{id}",
    tag = "Agents",
    security(("bearer_auth" = [])),
    params(("id" = Uuid, Path, description = "Agent ID")),
    responses(
        (status = 200, description = "Agent found", body = AgentResponse),
        (status = 404, description = "Not found")
    )
)]
pub async fn get_agent(
    State(state): State<AppState>,
    auth: auth_utils::AuthUser,
    Path(id): Path<Uuid>,
) -> Result<Json<AgentResponse>, AppError> {
    let row = agents::get_agent(state.repo().as_ref(), auth.user_id.0, id).await?;
    Ok(Json(AgentResponse::from_row(row)))
}

/// Update an existing agent (partial)
#[utoipa::path(
    patch,
    path = "/api/agents/{id}",
    tag = "Agents",
    security(("bearer_auth" = [])),
    params(("id" = Uuid, Path, description = "Agent ID")),
    request_body = UpdateAgentRequest,
    responses(
        (status = 200, description = "Updated agent", body = AgentResponse),
        (status = 404, description = "Not found")
    )
)]
pub async fn update_agent(
    State(state): State<AppState>,
    auth: auth_utils::AuthUser,
    Path(id): Path<Uuid>,
    Json(request): Json<UpdateAgentRequest>,
) -> Result<Json<AgentResponse>, AppError> {
    let row = agents::update_agent(
        state.repo().as_ref(),
        auth.user_id.0,
        id,
        agents::UpdateAgentInput {
            name: request.name,
            system_prompt: request.system_prompt,
            persona_style: request.persona_style,
            model_provider: request.model_provider,
            model_id: request.model_id,
            model_max_tokens: request.model_max_tokens,
            model_temperature: request.model_temperature,
            output_schema_id: request.output_schema_id,
            router_id: request.router_id,
        },
    )
    .await?;

    Ok(Json(AgentResponse::from_row(row)))
}

/// Delete an agent by ID
#[utoipa::path(
    delete,
    path = "/api/agents/{id}",
    tag = "Agents",
    security(("bearer_auth" = [])),
    params(("id" = Uuid, Path, description = "Agent ID")),
    responses(
        (status = 204, description = "Deleted successfully"),
        (status = 404, description = "Not found")
    )
)]
pub async fn delete_agent(
    State(state): State<AppState>,
    auth: auth_utils::AuthUser,
    Path(id): Path<Uuid>,
) -> Result<StatusCode, AppError> {
    agents::delete_agent(state.repo().as_ref(), auth.user_id.0, id).await?;
    Ok(StatusCode::NO_CONTENT)
}
#[cfg(test)]
mod tests;
