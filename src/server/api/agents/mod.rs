//! Agent management endpoints

use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::server::auth as auth_utils;
use crate::server::state::AppState;

use super::{MAX_PROMPT_LENGTH, MAX_TITLE_LENGTH};

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
    pub version: i32,
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
            version: row.version,
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
pub async fn list_agents(State(state): State<AppState>, auth: auth_utils::AuthUser) -> Result<Json<AgentsListResponse>, StatusCode> {
    let config = state.config.read().await;
    let pool_config = &config.pool;

    let rows = state.repo.list_persisted_agents(auth.user_id).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let agents: Vec<AgentResponse> = rows.into_iter().map(AgentResponse::from_row).collect();

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
pub async fn create_agent(State(state): State<AppState>, auth: auth_utils::AuthUser, Json(request): Json<CreateAgentRequest>) -> Result<(StatusCode, Json<AgentResponse>), StatusCode> {
    if request.model_id.trim().is_empty() {
        return Err(StatusCode::BAD_REQUEST);
    }
    if request.name.len() > MAX_TITLE_LENGTH {
        return Err(StatusCode::BAD_REQUEST);
    }
    if let Some(ref prompt) = request.system_prompt {
        if prompt.len() > MAX_PROMPT_LENGTH {
            return Err(StatusCode::BAD_REQUEST);
        }
    }

    let row = crate::db::AgentRow {
        id: Uuid::new_v4(),
        tier: None,
        name: request.name.trim().to_string(),
        system_prompt: request.system_prompt.unwrap_or_default(),
        persona_style: Some(request.persona_style.unwrap_or_else(|| "casual".to_string())),
        model_provider: request.model_provider.unwrap_or_else(|| "anthropic".to_string()),
        model_id: request.model_id.trim().to_string(),
        model_max_tokens: request.model_max_tokens.unwrap_or(4096),
        model_temperature: request.model_temperature.unwrap_or(0.7),
        status: Some("idle".to_string()),
        router_mode: Some(false),
        version: 1,
    };

    state.repo.upsert_agent(auth.user_id, row.clone()).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

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
pub async fn get_agent(State(state): State<AppState>, _auth: auth_utils::AuthUser, Path(id): Path<Uuid>) -> Result<Json<AgentResponse>, StatusCode> {
    let row = state.repo.get_persisted_agent(id).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?.ok_or(StatusCode::NOT_FOUND)?;

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
pub async fn update_agent(State(state): State<AppState>, auth: auth_utils::AuthUser, Path(id): Path<Uuid>, Json(request): Json<UpdateAgentRequest>) -> Result<Json<AgentResponse>, StatusCode> {
    let existing = state.repo.get_persisted_agent(id).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?.ok_or(StatusCode::NOT_FOUND)?;

    let updated = crate::db::AgentRow {
        id: existing.id,
        tier: None,
        name: request.name.unwrap_or(existing.name),
        system_prompt: request.system_prompt.unwrap_or(existing.system_prompt),
        persona_style: request.persona_style.map(Some).unwrap_or(existing.persona_style),
        model_provider: request.model_provider.unwrap_or(existing.model_provider),
        model_id: request.model_id.unwrap_or(existing.model_id),
        model_max_tokens: request.model_max_tokens.unwrap_or(existing.model_max_tokens),
        model_temperature: request.model_temperature.unwrap_or(existing.model_temperature),
        status: existing.status,
        router_mode: existing.router_mode,
        version: existing.version,
    };

    state.repo.upsert_agent(auth.user_id, updated.clone()).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(AgentResponse::from_row(updated)))
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
pub async fn delete_agent(State(state): State<AppState>, _auth: auth_utils::AuthUser, Path(id): Path<Uuid>) -> Result<StatusCode, StatusCode> {
    state.repo.delete_persisted_agent(id).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(StatusCode::NO_CONTENT)
}
mod tests;
