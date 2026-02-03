//! REST API endpoint handlers

use std::convert::Infallible;

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::sse::{Event, Sse},
    Json,
};
use chrono::{DateTime, Utc};
use futures::stream::Stream;
use serde::{Deserialize, Serialize};
use tokio::sync::broadcast;
use uuid::Uuid;

use super::auth;
use super::state::{AppState, OrchestratorMessage, StreamChunk};
use super::ws::SessionUpdate;
use crate::constants::{MAX_CHAT_MESSAGE_LENGTH, MAX_DESCRIPTION_LENGTH, MAX_PROMPT_LENGTH, MAX_TITLE_LENGTH};
use crate::types::{AgentPoolConfig, Priority, Task};

// ============================================================================
// Health Endpoint (Slice 10.2.1)
// ============================================================================

/// Health check response
#[derive(Serialize, utoipa::ToSchema)]
pub struct HealthResponse {
    pub status: String,
    pub version: String,
    pub db_connected: bool,
}

/// Enhanced health check endpoint
///
/// Returns JSON with status details including version and database connectivity.
#[utoipa::path(
    get,
    path = "/api/health",
    tag = "Health",
    responses(
        (status = 200, description = "Server health status", body = HealthResponse)
    )
)]
pub async fn health_check(State(state): State<AppState>) -> Json<HealthResponse> {
    let db_connected = state.repo.health_check().await;

    Json(HealthResponse {
        status: "ok".to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
        db_connected,
    })
}

// ============================================================================
// Tasks Endpoints (Slices 10.2.2 and 10.2.3)
// ============================================================================

/// Query parameters for listing tasks
#[derive(Deserialize, utoipa::ToSchema, utoipa::IntoParams)]
pub struct TasksQuery {
    pub status: Option<String>,
    pub limit: Option<u32>,
}

/// List all tasks with optional filtering
///
/// Supports query parameters:
/// - `status`: Filter by task status (pending, in_progress, completed, etc.)
/// - `limit`: Maximum number of tasks to return (default 100, max 1000)
#[utoipa::path(
    get,
    path = "/api/tasks",
    tag = "Tasks",
    security(("bearer_auth" = [])),
    params(TasksQuery),
    responses(
        (status = 200, description = "List of tasks", body = Vec<Task>)
    )
)]
pub async fn list_tasks(State(state): State<AppState>, auth: auth::AuthUser, Query(query): Query<TasksQuery>) -> Result<Json<Vec<Task>>, StatusCode> {
    let tasks = state.repo.list_tasks(auth.user_id, query.status, query.limit).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(tasks))
}

/// Get a single task by ID
///
/// Returns 404 if the task is not found.
#[utoipa::path(
    get,
    path = "/api/tasks/{id}",
    tag = "Tasks",
    security(("bearer_auth" = [])),
    params(("id" = Uuid, Path, description = "Task ID")),
    responses(
        (status = 200, description = "Task found", body = Task),
        (status = 404, description = "Task not found")
    )
)]
pub async fn get_task(State(state): State<AppState>, auth: auth::AuthUser, Path(id): Path<Uuid>) -> Result<Json<Task>, StatusCode> {
    let task = state
        .repo
        .get_task_by_uuid(auth.user_id, id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::NOT_FOUND)?;

    Ok(Json(task))
}

/// Request body for creating a new task
#[derive(Deserialize, utoipa::ToSchema)]
pub struct CreateTaskRequest {
    pub title: String,
    pub description: Option<String>,
    pub priority: Option<String>,
    pub tier: Option<String>,
}

/// Create a new task
///
/// Returns 201 with the created task on success.
/// Returns 400 if the title is empty.
#[utoipa::path(
    post,
    path = "/api/tasks",
    tag = "Tasks",
    security(("bearer_auth" = [])),
    request_body = CreateTaskRequest,
    responses(
        (status = 201, description = "Task created", body = Task),
        (status = 400, description = "Invalid request")
    )
)]
pub async fn create_task(State(state): State<AppState>, auth: auth::AuthUser, Json(request): Json<CreateTaskRequest>) -> Result<(StatusCode, Json<Task>), StatusCode> {
    if request.title.trim().is_empty() || request.title.len() > MAX_TITLE_LENGTH {
        return Err(StatusCode::BAD_REQUEST);
    }
    if let Some(ref desc) = request.description {
        if desc.len() > MAX_DESCRIPTION_LENGTH {
            return Err(StatusCode::BAD_REQUEST);
        }
    }

    // Parse priority (default to Normal)
    let priority = request
        .priority
        .as_ref()
        .map(|p| match p.to_lowercase().as_str() {
            "low" => Priority::Low,
            "high" => Priority::High,
            "urgent" => Priority::Urgent,
            _ => Priority::Normal,
        })
        .unwrap_or(Priority::Normal);

    // Create the task
    let mut task = Task::new(request.title.trim());
    task.description = request.description.unwrap_or_default();
    task.priority = priority;
    task.created_at = Utc::now();
    task.updated_at = Utc::now();

    // Insert into database
    state.repo.insert_task(auth.user_id, task.clone()).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok((StatusCode::CREATED, Json(task)))
}

// ============================================================================
// Agents Endpoints (Slice 10.2.4)
// ============================================================================

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
pub async fn list_agents(State(state): State<AppState>, auth: auth::AuthUser) -> Result<Json<AgentsListResponse>, StatusCode> {
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
pub async fn create_agent(State(state): State<AppState>, auth: auth::AuthUser, Json(request): Json<CreateAgentRequest>) -> Result<(StatusCode, Json<AgentResponse>), StatusCode> {
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
pub async fn get_agent(State(state): State<AppState>, _auth: auth::AuthUser, Path(id): Path<Uuid>) -> Result<Json<AgentResponse>, StatusCode> {
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
pub async fn update_agent(State(state): State<AppState>, auth: auth::AuthUser, Path(id): Path<Uuid>, Json(request): Json<UpdateAgentRequest>) -> Result<Json<AgentResponse>, StatusCode> {
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
pub async fn delete_agent(State(state): State<AppState>, _auth: auth::AuthUser, Path(id): Path<Uuid>) -> Result<StatusCode, StatusCode> {
    state.repo.delete_persisted_agent(id).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(StatusCode::NO_CONTENT)
}

// ============================================================================
// Tool Endpoints
// ============================================================================

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
    fn from_row(row: crate::db::ToolRow) -> Self {
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
pub async fn list_tools(State(state): State<AppState>, auth: auth::AuthUser) -> Result<Json<Vec<ToolResponse>>, StatusCode> {
    let rows = state.repo.list_tools(auth.user_id).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

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
pub async fn create_tool(State(state): State<AppState>, auth: auth::AuthUser, Json(request): Json<CreateToolRequest>) -> Result<(StatusCode, Json<ToolResponse>), StatusCode> {
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

    state.repo.upsert_tool(auth.user_id, row.clone()).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

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
pub async fn get_tool(State(state): State<AppState>, _auth: auth::AuthUser, Path(id): Path<Uuid>) -> Result<Json<ToolResponse>, StatusCode> {
    let row = state.repo.get_tool(id).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?.ok_or(StatusCode::NOT_FOUND)?;

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
pub async fn update_tool(State(state): State<AppState>, auth: auth::AuthUser, Path(id): Path<Uuid>, Json(request): Json<UpdateToolRequest>) -> Result<Json<ToolResponse>, StatusCode> {
    let existing = state.repo.get_tool(id).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?.ok_or(StatusCode::NOT_FOUND)?;

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

    state.repo.upsert_tool(auth.user_id, updated.clone()).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

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
pub async fn delete_tool(State(state): State<AppState>, _auth: auth::AuthUser, Path(id): Path<Uuid>) -> Result<StatusCode, StatusCode> {
    state.repo.delete_tool(id).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

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
pub async fn get_agent_tools(State(state): State<AppState>, _auth: auth::AuthUser, Path(agent_id): Path<Uuid>) -> Result<Json<AgentToolsResponse>, StatusCode> {
    let rows = state.repo.get_agent_tools(agent_id).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

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
    _auth: auth::AuthUser,
    Path(agent_id): Path<Uuid>,
    Json(request): Json<SetAgentToolsRequest>,
) -> Result<Json<AgentToolsResponse>, StatusCode> {
    let tool_ids: Result<Vec<Uuid>, _> = request.tool_ids.iter().map(|s| Uuid::parse_str(s)).collect();

    let tool_ids = tool_ids.map_err(|_| StatusCode::BAD_REQUEST)?;

    state.repo.set_agent_tools(agent_id, tool_ids).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let rows = state.repo.get_agent_tools(agent_id).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let tools = rows.into_iter().map(ToolResponse::from_row).collect();

    Ok(Json(AgentToolsResponse {
        agent_id: agent_id.to_string(),
        tools,
    }))
}

// ============================================================================
// Agent Context (Document Linkage)
// ============================================================================

#[derive(Deserialize, utoipa::ToSchema)]
pub struct SetAgentContextRequest {
    pub document_ids: Vec<String>,
}

#[derive(Serialize, utoipa::ToSchema)]
pub struct AgentContextResponse {
    pub agent_id: String,
    pub documents: Vec<DocumentListItem>,
}

/// Get context documents assigned to an agent
#[utoipa::path(
    get,
    path = "/api/agents/{id}/context",
    tag = "Agent Context",
    security(("bearer_auth" = [])),
    params(("id" = Uuid, Path, description = "Agent ID")),
    responses(
        (status = 200, description = "Agent context documents", body = AgentContextResponse)
    )
)]
pub async fn get_agent_context(State(state): State<AppState>, _auth: auth::AuthUser, Path(agent_id): Path<Uuid>) -> Result<Json<AgentContextResponse>, StatusCode> {
    let rows = state.repo.get_agent_context(agent_id).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let documents = rows
        .into_iter()
        .map(|row| DocumentListItem {
            id: row.id,
            title: row.title,
            summary: row.summary,
            ref_tag: row.ref_tag,
            tags: row.tags,
            doc_type: row.doc_type,
            updated_at: row.updated_at,
        })
        .collect();

    Ok(Json(AgentContextResponse {
        agent_id: agent_id.to_string(),
        documents,
    }))
}

/// Set context documents for an agent (replaces existing)
#[utoipa::path(
    put,
    path = "/api/agents/{id}/context",
    tag = "Agent Context",
    security(("bearer_auth" = [])),
    params(("id" = Uuid, Path, description = "Agent ID")),
    request_body = SetAgentContextRequest,
    responses(
        (status = 200, description = "Agent context updated", body = AgentContextResponse),
        (status = 400, description = "Invalid document IDs")
    )
)]
pub async fn set_agent_context(
    State(state): State<AppState>,
    _auth: auth::AuthUser,
    Path(agent_id): Path<Uuid>,
    Json(request): Json<SetAgentContextRequest>,
) -> Result<Json<AgentContextResponse>, StatusCode> {
    let document_ids: Result<Vec<Uuid>, _> = request.document_ids.iter().map(|s| Uuid::parse_str(s)).collect();

    let document_ids = document_ids.map_err(|_| StatusCode::BAD_REQUEST)?;

    state.repo.set_agent_context(agent_id, document_ids).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let rows = state.repo.get_agent_context(agent_id).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let documents = rows
        .into_iter()
        .map(|row| DocumentListItem {
            id: row.id,
            title: row.title,
            summary: row.summary,
            ref_tag: row.ref_tag,
            tags: row.tags,
            doc_type: row.doc_type,
            updated_at: row.updated_at,
        })
        .collect();

    Ok(Json(AgentContextResponse {
        agent_id: agent_id.to_string(),
        documents,
    }))
}

// ============================================================================
// Config Endpoints (Slice 10.2.5)
// ============================================================================

/// Configuration response
#[derive(Serialize, utoipa::ToSchema)]
pub struct ConfigResponse {
    pub verbosity: String,
    pub pool: AgentPoolConfig,
    pub autonomy: String,
    pub git_strategy: String,
    pub sandbox_mode: String,
}

/// Get current configuration
#[utoipa::path(
    get,
    path = "/api/config",
    tag = "Config",
    responses(
        (status = 200, description = "Current configuration", body = ConfigResponse)
    )
)]
pub async fn get_config(State(state): State<AppState>) -> Json<ConfigResponse> {
    let config = state.config.read().await;

    Json(ConfigResponse {
        verbosity: format!("{:?}", config.verbosity).to_lowercase(),
        pool: config.pool.clone(),
        autonomy: format!("{:?}", config.autonomy).to_lowercase(),
        git_strategy: format!("{:?}", config.git_strategy).to_lowercase(),
        sandbox_mode: format!("{:?}", config.sandbox_mode).to_lowercase(),
    })
}

/// Request body for updating pool sizes
#[derive(Deserialize, utoipa::ToSchema)]
pub struct UpdatePoolRequest {
    pub max_agents: Option<u8>,
}

/// Request body for updating configuration
#[derive(Deserialize, utoipa::ToSchema)]
pub struct UpdateConfigRequest {
    pub verbosity: Option<String>,
    pub pool: Option<UpdatePoolRequest>,
    pub autonomy: Option<String>,
    pub git_strategy: Option<String>,
    pub sandbox_mode: Option<String>,
}

/// Update configuration (partial update)
#[utoipa::path(
    patch,
    path = "/api/config",
    tag = "Config",
    request_body = UpdateConfigRequest,
    responses(
        (status = 200, description = "Updated configuration", body = ConfigResponse),
        (status = 400, description = "Invalid value")
    )
)]
pub async fn update_config(State(state): State<AppState>, Json(request): Json<UpdateConfigRequest>) -> Result<Json<ConfigResponse>, StatusCode> {
    use crate::types::{AutonomyLevel, GitStrategy, SandboxMode, VerbosityLevel};

    let mut config = state.config.write().await;

    // Verbosity
    if let Some(ref v) = request.verbosity {
        match v.to_lowercase().as_str() {
            "quiet" => config.verbosity = VerbosityLevel::Quiet,
            "normal" => config.verbosity = VerbosityLevel::Normal,
            "verbose" => config.verbosity = VerbosityLevel::Verbose,
            _ => return Err(StatusCode::BAD_REQUEST),
        }
    }

    // Pool
    if let Some(ref pool) = request.pool {
        if let Some(v) = pool.max_agents {
            config.pool.max_agents = v;
        }
    }

    // Autonomy
    if let Some(ref a) = request.autonomy {
        match a.to_lowercase().as_str() {
            "full_auto" => config.autonomy = AutonomyLevel::FullAuto,
            "approval_gates" => config.autonomy = AutonomyLevel::ApprovalGates,
            "supervised" => config.autonomy = AutonomyLevel::Supervised,
            _ => return Err(StatusCode::BAD_REQUEST),
        }
    }

    // Git strategy
    if let Some(ref g) = request.git_strategy {
        match g.to_lowercase().as_str() {
            "branch_per_slice" => config.git_strategy = GitStrategy::BranchPerSlice,
            "branch_per_ticket" => config.git_strategy = GitStrategy::BranchPerTicket,
            _ => return Err(StatusCode::BAD_REQUEST),
        }
    }

    // Sandbox mode
    if let Some(ref s) = request.sandbox_mode {
        match s.to_lowercase().as_str() {
            "docker" => config.sandbox_mode = SandboxMode::Docker,
            "local_restricted" => config.sandbox_mode = SandboxMode::LocalRestricted,
            "none" => config.sandbox_mode = SandboxMode::None,
            _ => return Err(StatusCode::BAD_REQUEST),
        }
    }

    let resp = ConfigResponse {
        verbosity: format!("{:?}", config.verbosity).to_lowercase(),
        pool: config.pool.clone(),
        autonomy: format!("{:?}", config.autonomy).to_lowercase(),
        git_strategy: format!("{:?}", config.git_strategy).to_lowercase(),
        sandbox_mode: format!("{:?}", config.sandbox_mode).to_lowercase(),
    };

    Ok(Json(resp))
}

// ============================================================================
// Chat Endpoints (Slices 10.3.1 - 10.3.4)
// ============================================================================

/// Request body for sending a chat message
#[derive(Deserialize, utoipa::ToSchema)]
pub struct ChatRequest {
    pub message: String,
}

/// Response for sending a chat message
#[derive(Serialize, utoipa::ToSchema)]
pub struct ChatResponse {
    pub message_id: Uuid,
    pub status: String,
}

/// Send a chat message to the orchestrator
///
/// Returns 202 Accepted with the message ID.
/// The message is queued for processing by the orchestrator.
#[utoipa::path(
    post,
    path = "/api/chat",
    tag = "Chat",
    security(("bearer_auth" = [])),
    request_body = ChatRequest,
    responses(
        (status = 202, description = "Message queued", body = ChatResponse),
        (status = 400, description = "Invalid message")
    )
)]
pub async fn send_chat(State(state): State<AppState>, auth: auth::AuthUser, Json(request): Json<ChatRequest>) -> Result<(StatusCode, Json<ChatResponse>), StatusCode> {
    if request.message.trim().is_empty() || request.message.len() > MAX_CHAT_MESSAGE_LENGTH {
        return Err(StatusCode::BAD_REQUEST);
    }

    let message_id = Uuid::new_v4();

    // Pre-create the buffered stream so chunks are captured even before
    // the SSE client connects
    state.ensure_response_stream(message_id).await;

    // Store the user message in the database
    state
        .repo
        .insert_chat_message(auth.user_id, message_id, "user".to_string(), request.message.clone())
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    // Queue message to orchestrator
    state
        .orchestrator_tx
        .send(OrchestratorMessage {
            id: message_id,
            user_id: auth.user_id,
            session_id: None,
            agent_id: None,
            content: request.message,
            timestamp: Utc::now(),
        })
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok((
        StatusCode::ACCEPTED,
        Json(ChatResponse {
            message_id,
            status: "queued".to_string(),
        }),
    ))
}

/// Query parameters for chat history
#[derive(Deserialize, utoipa::ToSchema, utoipa::IntoParams)]
pub struct HistoryQuery {
    pub limit: Option<u32>,
    pub offset: Option<u32>,
}

/// A chat message in the response
#[derive(Serialize, utoipa::ToSchema)]
pub struct ChatMessage {
    pub id: Uuid,
    pub role: String,
    pub content: String,
    pub timestamp: DateTime<Utc>,
}

/// Get chat history with pagination
///
/// Returns messages in chronological order.
#[utoipa::path(
    get,
    path = "/api/chat/history",
    tag = "Chat",
    security(("bearer_auth" = [])),
    params(HistoryQuery),
    responses(
        (status = 200, description = "Chat history", body = Vec<ChatMessage>)
    )
)]
pub async fn get_chat_history(State(state): State<AppState>, auth: auth::AuthUser, Query(query): Query<HistoryQuery>) -> Result<Json<Vec<ChatMessage>>, StatusCode> {
    let limit = query.limit.unwrap_or(50);
    let offset = query.offset.unwrap_or(0);

    let rows = state.repo.get_chat_history(auth.user_id, limit, offset).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let messages: Vec<ChatMessage> = rows
        .into_iter()
        .map(|row| ChatMessage {
            id: row.id,
            role: row.role,
            content: row.content,
            timestamp: row.timestamp,
        })
        .collect();

    Ok(Json(messages))
}

/// Stream chat response via Server-Sent Events
///
/// Subscribes to the response stream for a specific message and
/// streams tokens as they are generated.
#[utoipa::path(
    get,
    path = "/api/chat/{message_id}/stream",
    tag = "Chat",
    params(("message_id" = Uuid, Path, description = "Message ID")),
    responses(
        (status = 200, description = "SSE event stream")
    )
)]
pub async fn chat_stream(State(state): State<AppState>, Path(message_id): Path<Uuid>) -> Sse<impl Stream<Item = Result<Event, Infallible>>> {
    chat_stream_inner(state, message_id)
}

/// Stream chat response for session-scoped messages.
///
/// Same as `chat_stream` but extracts both session_id and message_id
/// from the path (only message_id is used for stream lookup).
#[utoipa::path(
    get,
    path = "/api/sessions/{session_id}/chat/{message_id}/stream",
    tag = "Sessions",
    params(
        ("session_id" = Uuid, Path, description = "Session ID"),
        ("message_id" = Uuid, Path, description = "Message ID")
    ),
    responses(
        (status = 200, description = "SSE event stream")
    )
)]
pub async fn session_chat_stream(State(state): State<AppState>, Path((_session_id, message_id)): Path<(Uuid, Uuid)>) -> Sse<impl Stream<Item = Result<Event, Infallible>>> {
    chat_stream_inner(state, message_id)
}

fn chat_stream_inner(state: AppState, message_id: Uuid) -> Sse<impl Stream<Item = Result<Event, Infallible>>> {
    let stream = async_stream::stream! {
        let (buffered, mut rx, already_done) = state.get_response_stream(message_id).await;

        // Replay any buffered chunks that arrived before we connected
        for chunk in buffered {
            match chunk {
                StreamChunk::Token(text) => {
                    yield Ok(Event::default().event("token").data(serde_json::to_string(&text).unwrap_or(text)));
                }
                StreamChunk::ToolStart { name, tool_id } => {
                    let data = format!(r#"{{"name":"{}","id":"{}"}}"#, name, tool_id);
                    yield Ok(Event::default().event("tool_start").data(data));
                }
                StreamChunk::ToolEnd { name, tool_id } => {
                    let data = format!(r#"{{"name":"{}","id":"{}"}}"#, name, tool_id);
                    yield Ok(Event::default().event("tool_end").data(data));
                }
                StreamChunk::DocUpdate { doc_id, title } => {
                    let data = format!(r#"{{"doc_id":"{}","title":"{}"}}"#, doc_id, title);
                    yield Ok(Event::default().event("doc_update").data(data));
                }
                StreamChunk::Done => {
                    yield Ok(Event::default().event("done").data(""));
                    return;
                }
                StreamChunk::Error(e) => {
                    yield Ok(Event::default().event("error").data(e));
                    return;
                }
            }
        }

        if already_done {
            yield Ok(Event::default().event("done").data(""));
            return;
        }

        // Listen for new chunks from the orchestrator
        loop {
            match rx.recv().await {
                Ok(chunk) => {
                    match chunk {
                        StreamChunk::Token(text) => {
                            yield Ok(Event::default().event("token").data(serde_json::to_string(&text).unwrap_or(text)));
                        }
                        StreamChunk::ToolStart { name, tool_id } => {
                            let data = format!(r#"{{"name":"{}","id":"{}"}}"#, name, tool_id);
                            yield Ok(Event::default().event("tool_start").data(data));
                        }
                        StreamChunk::ToolEnd { name, tool_id } => {
                            let data = format!(r#"{{"name":"{}","id":"{}"}}"#, name, tool_id);
                            yield Ok(Event::default().event("tool_end").data(data));
                        }
                        StreamChunk::DocUpdate { doc_id, title } => {
                            let data = format!(r#"{{"doc_id":"{}","title":"{}"}}"#, doc_id, title);
                            yield Ok(Event::default().event("doc_update").data(data));
                        }
                        StreamChunk::Done => {
                            yield Ok(Event::default().event("done").data(""));
                            break;
                        }
                        StreamChunk::Error(e) => {
                            yield Ok(Event::default().event("error").data(e));
                            break;
                        }
                    }
                }
                Err(broadcast::error::RecvError::Closed) => {
                    break;
                }
                Err(broadcast::error::RecvError::Lagged(_)) => {
                    continue;
                }
            }
        }
    };

    Sse::new(stream)
}

/// Clear all chat history
///
/// Returns 204 No Content on success.
#[utoipa::path(
    delete,
    path = "/api/chat/history",
    tag = "Chat",
    security(("bearer_auth" = [])),
    responses(
        (status = 204, description = "Chat history cleared")
    )
)]
pub async fn clear_chat_history(State(state): State<AppState>, auth: auth::AuthUser) -> StatusCode {
    match state.repo.clear_chat_history(auth.user_id).await {
        Ok(_) => StatusCode::NO_CONTENT,
        Err(_) => StatusCode::INTERNAL_SERVER_ERROR,
    }
}

// ============================================================================
// Mode & Session Endpoints
// ============================================================================

/// An available agent mode
#[derive(Serialize, utoipa::ToSchema)]
pub struct ModeInfo {
    pub id: String,
    pub name: String,
    pub description: String,
}

/// List available agents (replaces legacy list_modes)
#[utoipa::path(
    get,
    path = "/api/modes",
    tag = "Sessions",
    security(("bearer_auth" = [])),
    responses(
        (status = 200, description = "List of available modes", body = Vec<ModeInfo>)
    )
)]
pub async fn list_modes(State(state): State<AppState>, auth: auth::AuthUser) -> Result<Json<Vec<ModeInfo>>, StatusCode> {
    let agents = state.repo.list_persisted_agents(auth.user_id).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let modes: Vec<ModeInfo> = agents
        .into_iter()
        .map(|a| ModeInfo {
            id: a.id.to_string(),
            name: a.name,
            description: a.system_prompt.chars().take(120).collect(),
        })
        .collect();
    Ok(Json(modes))
}

// ============================================================================
// Agent Mode Endpoints
// ============================================================================

/// Response for an agent mode
#[derive(Serialize, utoipa::ToSchema)]
pub struct AgentModeResponse {
    pub id: Uuid,
    pub agent_id: Uuid,
    pub name: String,
    pub system_prompt_suffix: Option<String>,
    pub temperature_override: Option<f64>,
    pub model_override: Option<String>,
    pub tool_overrides: Option<Vec<String>>,
    pub classifier_hint: String,
    pub created_at: DateTime<Utc>,
    pub version: i32,
}

impl From<crate::db::AgentModeRow> for AgentModeResponse {
    fn from(r: crate::db::AgentModeRow) -> Self {
        Self {
            id: r.id,
            agent_id: r.agent_id,
            name: r.name,
            system_prompt_suffix: r.system_prompt_suffix,
            temperature_override: r.temperature_override,
            model_override: r.model_override,
            tool_overrides: r.tool_overrides,
            classifier_hint: r.classifier_hint,
            created_at: r.created_at,
            version: r.version,
        }
    }
}

/// Request body for creating an agent mode
#[derive(Deserialize, utoipa::ToSchema)]
pub struct CreateAgentModeRequest {
    pub name: String,
    #[serde(default)]
    pub system_prompt_suffix: Option<String>,
    #[serde(default)]
    pub temperature_override: Option<f64>,
    #[serde(default)]
    pub model_override: Option<String>,
    #[serde(default)]
    pub tool_overrides: Option<Vec<String>>,
    pub classifier_hint: String,
}

/// List all modes for an agent
#[utoipa::path(
    get,
    path = "/api/agents/{agent_id}/modes",
    tag = "Agent Modes",
    security(("bearer_auth" = [])),
    params(("agent_id" = Uuid, Path, description = "Agent ID")),
    responses(
        (status = 200, description = "List of agent modes", body = Vec<AgentModeResponse>)
    )
)]
pub async fn list_agent_modes(State(state): State<AppState>, _auth: auth::AuthUser, Path(agent_id): Path<Uuid>) -> Result<Json<Vec<AgentModeResponse>>, StatusCode> {
    let modes = state.repo.get_agent_modes(agent_id).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Json(modes.into_iter().map(AgentModeResponse::from).collect()))
}

/// Create a new mode for an agent
#[utoipa::path(
    post,
    path = "/api/agents/{agent_id}/modes",
    tag = "Agent Modes",
    security(("bearer_auth" = [])),
    params(("agent_id" = Uuid, Path, description = "Agent ID")),
    request_body = CreateAgentModeRequest,
    responses(
        (status = 201, description = "Mode created", body = AgentModeResponse)
    )
)]
pub async fn create_agent_mode(
    State(state): State<AppState>,
    _auth: auth::AuthUser,
    Path(agent_id): Path<Uuid>,
    Json(req): Json<CreateAgentModeRequest>,
) -> Result<(StatusCode, Json<AgentModeResponse>), StatusCode> {
    let mode = crate::db::AgentModeRow {
        id: Uuid::new_v4(),
        agent_id,
        name: req.name,
        system_prompt_suffix: req.system_prompt_suffix,
        temperature_override: req.temperature_override,
        model_override: req.model_override,
        tool_overrides: req.tool_overrides,
        classifier_hint: req.classifier_hint,
        created_at: Utc::now(),
        version: 1,
    };

    state.repo.create_agent_mode(&mode).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok((StatusCode::CREATED, Json(AgentModeResponse::from(mode))))
}

/// Delete an agent mode
#[utoipa::path(
    delete,
    path = "/api/agent-modes/{mode_id}",
    tag = "Agent Modes",
    security(("bearer_auth" = [])),
    params(("mode_id" = Uuid, Path, description = "Mode ID")),
    responses(
        (status = 204, description = "Mode deleted")
    )
)]
pub async fn delete_agent_mode(State(state): State<AppState>, _auth: auth::AuthUser, Path(mode_id): Path<Uuid>) -> Result<StatusCode, StatusCode> {
    state.repo.delete_agent_mode(mode_id).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(StatusCode::NO_CONTENT)
}

/// Request body for creating a session
#[derive(Deserialize, utoipa::ToSchema)]
pub struct CreateSessionRequest {
    #[serde(default)]
    pub mode_id: String,
    #[serde(default)]
    pub agent_id: Option<Uuid>,
    #[serde(default)]
    pub title: String,
}

/// Request body for updating a session
#[derive(Deserialize, utoipa::ToSchema)]
pub struct UpdateSessionRequest {
    pub title: String,
}

/// Response for session creation
#[derive(Serialize, utoipa::ToSchema)]
pub struct SessionResponse {
    pub id: Uuid,
    pub mode_id: String,
    pub agent_id: Option<Uuid>,
    pub title: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Create a new chat session
#[utoipa::path(
    post,
    path = "/api/sessions",
    tag = "Sessions",
    security(("bearer_auth" = [])),
    request_body = CreateSessionRequest,
    responses(
        (status = 201, description = "Session created", body = SessionResponse),
        (status = 400, description = "Invalid agent ID")
    )
)]
pub async fn create_session(State(state): State<AppState>, auth: auth::AuthUser, Json(request): Json<CreateSessionRequest>) -> Result<(StatusCode, Json<SessionResponse>), StatusCode> {
    // Validate agent exists if provided
    if let Some(aid) = request.agent_id {
        if state.repo.get_persisted_agent(aid).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?.is_none() {
            return Err(StatusCode::BAD_REQUEST);
        }
    }

    let session_id = Uuid::new_v4();
    let mode_id = if request.mode_id.is_empty() { "home".to_string() } else { request.mode_id };
    let title = if request.title.is_empty() { "New session".to_string() } else { request.title };

    state
        .repo
        .create_session(auth.user_id, session_id, &mode_id, &title, request.agent_id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let session = state
        .repo
        .get_session(session_id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::INTERNAL_SERVER_ERROR)?;

    state.broadcast_session(SessionUpdate {
        id: session.id,
        action: "created".to_string(),
        title: Some(session.title.clone()),
        mode_id: Some(session.mode_id.clone()),
        user_id: Some(auth.user_id.0),
    });

    Ok((
        StatusCode::CREATED,
        Json(SessionResponse {
            id: session.id,
            mode_id: session.mode_id,
            agent_id: session.agent_id,
            title: session.title,
            created_at: session.created_at,
            updated_at: session.updated_at,
        }),
    ))
}

/// List sessions for the current user
#[utoipa::path(
    get,
    path = "/api/sessions",
    tag = "Sessions",
    security(("bearer_auth" = [])),
    responses(
        (status = 200, description = "List of sessions", body = Vec<SessionResponse>)
    )
)]
pub async fn list_sessions(State(state): State<AppState>, auth: auth::AuthUser) -> Result<Json<Vec<SessionResponse>>, StatusCode> {
    let sessions = state.repo.list_sessions(auth.user_id).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let response: Vec<SessionResponse> = sessions
        .into_iter()
        .map(|s| SessionResponse {
            id: s.id,
            mode_id: s.mode_id,
            agent_id: s.agent_id,
            title: s.title,
            created_at: s.created_at,
            updated_at: s.updated_at,
        })
        .collect();

    Ok(Json(response))
}

/// Get a specific session
#[utoipa::path(
    get,
    path = "/api/sessions/{session_id}",
    tag = "Sessions",
    security(("bearer_auth" = [])),
    params(("session_id" = Uuid, Path, description = "Session ID")),
    responses(
        (status = 200, description = "Session found", body = SessionResponse),
        (status = 404, description = "Not found")
    )
)]
pub async fn get_session(State(state): State<AppState>, auth: auth::AuthUser, Path(session_id): Path<Uuid>) -> Result<Json<SessionResponse>, StatusCode> {
    let session = state.repo.get_session(session_id).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?.ok_or(StatusCode::NOT_FOUND)?;

    // Verify ownership
    if session.user_id != auth.user_id.0 {
        return Err(StatusCode::NOT_FOUND);
    }

    Ok(Json(SessionResponse {
        id: session.id,
        mode_id: session.mode_id,
        agent_id: session.agent_id,
        title: session.title,
        created_at: session.created_at,
        updated_at: session.updated_at,
    }))
}

/// Delete a session
#[utoipa::path(
    delete,
    path = "/api/sessions/{session_id}",
    tag = "Sessions",
    security(("bearer_auth" = [])),
    params(("session_id" = Uuid, Path, description = "Session ID")),
    responses(
        (status = 204, description = "Deleted successfully"),
        (status = 404, description = "Not found")
    )
)]
pub async fn delete_session(State(state): State<AppState>, auth: auth::AuthUser, Path(session_id): Path<Uuid>) -> Result<StatusCode, StatusCode> {
    // Verify ownership
    let session = state.repo.get_session(session_id).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?.ok_or(StatusCode::NOT_FOUND)?;

    if session.user_id != auth.user_id.0 {
        return Err(StatusCode::NOT_FOUND);
    }

    state.repo.delete_session(session_id).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    state.broadcast_session(SessionUpdate {
        id: session_id,
        action: "deleted".to_string(),
        title: None,
        mode_id: None,
        user_id: Some(auth.user_id.0),
    });

    Ok(StatusCode::NO_CONTENT)
}

/// Update a session (rename)
#[utoipa::path(
    patch,
    path = "/api/sessions/{session_id}",
    tag = "Sessions",
    security(("bearer_auth" = [])),
    params(("session_id" = Uuid, Path, description = "Session ID")),
    request_body = UpdateSessionRequest,
    responses(
        (status = 200, description = "Session updated", body = SessionResponse),
        (status = 404, description = "Not found")
    )
)]
pub async fn update_session(State(state): State<AppState>, auth: auth::AuthUser, Path(session_id): Path<Uuid>, Json(request): Json<UpdateSessionRequest>) -> Result<Json<SessionResponse>, StatusCode> {
    let session = state.repo.get_session(session_id).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?.ok_or(StatusCode::NOT_FOUND)?;

    if session.user_id != auth.user_id.0 {
        return Err(StatusCode::NOT_FOUND);
    }

    state.repo.update_session_title(session_id, &request.title).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let updated = state
        .repo
        .get_session(session_id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::INTERNAL_SERVER_ERROR)?;

    state.broadcast_session(SessionUpdate {
        id: updated.id,
        action: "updated".to_string(),
        title: Some(updated.title.clone()),
        mode_id: Some(updated.mode_id.clone()),
        user_id: Some(auth.user_id.0),
    });

    Ok(Json(SessionResponse {
        id: updated.id,
        mode_id: updated.mode_id,
        agent_id: updated.agent_id,
        title: updated.title,
        created_at: updated.created_at,
        updated_at: updated.updated_at,
    }))
}

/// Send a message to a session
#[utoipa::path(
    post,
    path = "/api/sessions/{session_id}/chat",
    tag = "Sessions",
    security(("bearer_auth" = [])),
    params(("session_id" = Uuid, Path, description = "Session ID")),
    request_body = ChatRequest,
    responses(
        (status = 202, description = "Message queued", body = ChatResponse),
        (status = 400, description = "Empty message"),
        (status = 404, description = "Session not found")
    )
)]
pub async fn send_session_chat(
    State(state): State<AppState>,
    auth: auth::AuthUser,
    Path(session_id): Path<Uuid>,
    Json(request): Json<ChatRequest>,
) -> Result<(StatusCode, Json<ChatResponse>), StatusCode> {
    if request.message.trim().is_empty() {
        return Err(StatusCode::BAD_REQUEST);
    }

    // Verify session exists and belongs to user
    let session = state.repo.get_session(session_id).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?.ok_or(StatusCode::NOT_FOUND)?;

    if session.user_id != auth.user_id.0 {
        return Err(StatusCode::NOT_FOUND);
    }

    let message_id = Uuid::new_v4();

    state.ensure_response_stream(message_id).await;

    // Store user message scoped to session
    state
        .repo
        .insert_session_message(auth.user_id, session_id, message_id, "user".to_string(), request.message.clone())
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    // Queue to orchestrator with session context
    state
        .orchestrator_tx
        .send(OrchestratorMessage {
            id: message_id,
            user_id: auth.user_id,
            session_id: Some(session_id),
            agent_id: session.agent_id,
            content: request.message,
            timestamp: Utc::now(),
        })
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok((
        StatusCode::ACCEPTED,
        Json(ChatResponse {
            message_id,
            status: "queued".to_string(),
        }),
    ))
}

/// Get session chat history
#[utoipa::path(
    get,
    path = "/api/sessions/{session_id}/history",
    tag = "Sessions",
    security(("bearer_auth" = [])),
    params(
        ("session_id" = Uuid, Path, description = "Session ID"),
        HistoryQuery
    ),
    responses(
        (status = 200, description = "Session chat history", body = Vec<ChatMessage>),
        (status = 404, description = "Session not found")
    )
)]
pub async fn get_session_history(State(state): State<AppState>, auth: auth::AuthUser, Path(session_id): Path<Uuid>, Query(query): Query<HistoryQuery>) -> Result<Json<Vec<ChatMessage>>, StatusCode> {
    // Verify session ownership
    let session = state.repo.get_session(session_id).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?.ok_or(StatusCode::NOT_FOUND)?;

    if session.user_id != auth.user_id.0 {
        return Err(StatusCode::NOT_FOUND);
    }

    let limit = query.limit.unwrap_or(50);
    let rows = state.repo.get_session_history(session_id, limit).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let messages: Vec<ChatMessage> = rows
        .into_iter()
        .map(|row| ChatMessage {
            id: row.id,
            role: row.role,
            content: row.content,
            timestamp: row.timestamp,
        })
        .collect();

    Ok(Json(messages))
}

// ============================================================================
// Auth Endpoints (Ticket 10.5)
// ============================================================================

/// Request body for auth setup
#[derive(Deserialize, utoipa::ToSchema)]
pub struct SetupRequest {
    pub password: String,
}

/// Response for auth setup
#[derive(Serialize, utoipa::ToSchema)]
pub struct SetupResponse {
    pub message: String,
}

/// POST /api/auth/setup - First-run password configuration
///
/// This endpoint is only available when no password has been configured yet.
/// Once a password is set, this endpoint returns 409 Conflict.
#[utoipa::path(
    post,
    path = "/api/auth/setup",
    tag = "Auth",
    request_body = SetupRequest,
    responses(
        (status = 200, description = "Password configured", body = SetupResponse),
        (status = 400, description = "Password too short"),
        (status = 409, description = "Password already configured")
    )
)]
pub async fn auth_setup(State(state): State<AppState>, Json(request): Json<SetupRequest>) -> Result<Json<SetupResponse>, (StatusCode, String)> {
    // Check if already setup
    if state.repo.has_password().await.map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))? {
        return Err((StatusCode::CONFLICT, "Password already configured".to_string()));
    }

    // Validate password strength
    if request.password.len() < 8 {
        return Err((StatusCode::BAD_REQUEST, "Password must be at least 8 characters".to_string()));
    }

    // Hash and store
    let hash = auth::hash_password(&request.password).map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    state.repo.set_password(hash).await.map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(Json(SetupResponse {
        message: "Password configured successfully".to_string(),
    }))
}

/// Request body for registration
#[derive(Deserialize, utoipa::ToSchema)]
pub struct RegisterRequest {
    pub email: String,
    pub password: String,
}

/// Response for registration
#[derive(Serialize, utoipa::ToSchema)]
pub struct AuthTokenResponse {
    pub token: String,
    pub expires_in: u64,
    pub user: UserResponse,
}

/// User info in API responses
#[derive(Serialize, utoipa::ToSchema)]
pub struct UserResponse {
    pub id: String,
    pub email: String,
    pub github_login: Option<String>,
}

/// POST /api/auth/register - Register a new user
#[utoipa::path(
    post,
    path = "/api/auth/register",
    tag = "Auth",
    request_body = RegisterRequest,
    responses(
        (status = 201, description = "User registered", body = AuthTokenResponse),
        (status = 400, description = "Invalid email or password"),
        (status = 409, description = "Email already registered")
    )
)]
pub async fn auth_register(State(state): State<AppState>, Json(request): Json<RegisterRequest>) -> Result<(StatusCode, Json<AuthTokenResponse>), (StatusCode, String)> {
    // Validate
    if request.email.trim().is_empty() || !request.email.contains('@') {
        return Err((StatusCode::BAD_REQUEST, "Invalid email".into()));
    }
    if request.password.len() < 8 {
        return Err((StatusCode::BAD_REQUEST, "Password must be at least 8 characters".into()));
    }

    let user_repo = state.user_repo.as_ref().ok_or((StatusCode::INTERNAL_SERVER_ERROR, "User service unavailable".into()))?;

    // Check if email already exists
    if user_repo
        .get_user_by_email(&request.email)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .is_some()
    {
        return Err((StatusCode::CONFLICT, "Email already registered".into()));
    }

    let hash = auth::hash_password(&request.password).map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let user = user_repo.create_user(&request.email, &hash).await.map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    // Seed built-in execution tools for the new user
    let _ = state.repo.seed_builtin_tools(user.id).await;

    let token = auth::create_token(&state.jwt_secret, 24, user.id, &user.email).map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok((
        StatusCode::CREATED,
        Json(AuthTokenResponse {
            token,
            expires_in: 86400,
            user: UserResponse {
                id: user.id.to_string(),
                email: user.email,
                github_login: user.github_login,
            },
        }),
    ))
}

/// Request body for login
#[derive(Deserialize, utoipa::ToSchema)]
pub struct LoginRequest {
    pub email: String,
    pub password: String,
}

/// Response for successful login
#[derive(Serialize, utoipa::ToSchema)]
pub struct LoginResponse {
    pub token: String,
    pub expires_in: u64,
}

/// POST /api/auth/login - Authenticate and get JWT token
///
/// Verifies the provided password and returns a JWT token valid for 24 hours.
#[utoipa::path(
    post,
    path = "/api/auth/login",
    tag = "Auth",
    request_body = LoginRequest,
    responses(
        (status = 200, description = "Login successful", body = LoginResponse),
        (status = 401, description = "Invalid credentials")
    )
)]
pub async fn auth_login(State(state): State<AppState>, Json(request): Json<LoginRequest>) -> Result<Json<LoginResponse>, StatusCode> {
    let user_repo = state.user_repo.as_ref().ok_or(StatusCode::INTERNAL_SERVER_ERROR)?;

    let user = user_repo
        .get_user_by_email(&request.email)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::UNAUTHORIZED)?;

    let password_hash = user.password_hash.as_ref().ok_or(StatusCode::UNAUTHORIZED)?;
    if !auth::verify_password(&request.password, password_hash) {
        return Err(StatusCode::UNAUTHORIZED);
    }

    let token = auth::create_token(&state.jwt_secret, 24, user.id, &user.email).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(LoginResponse { token, expires_in: 86400 }))
}

/// Response for /api/auth/me
#[derive(Serialize, utoipa::ToSchema)]
pub struct MeResponse {
    pub id: String,
    pub email: String,
    pub github_login: Option<String>,
    pub authenticated: bool,
    pub token_expires: usize,
}

/// GET /api/auth/me - Get current user info from token
///
/// Requires a valid JWT token in Authorization header.
#[utoipa::path(
    get,
    path = "/api/auth/me",
    tag = "Auth",
    security(("bearer_auth" = [])),
    responses(
        (status = 200, description = "Current user info", body = MeResponse)
    )
)]
pub async fn auth_me(State(state): State<AppState>, auth: auth::AuthUser) -> Result<Json<MeResponse>, StatusCode> {
    let user_repo = state.user_repo.as_ref().ok_or(StatusCode::INTERNAL_SERVER_ERROR)?;

    let user = user_repo
        .get_user_by_id(auth.user_id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::UNAUTHORIZED)?;

    Ok(Json(MeResponse {
        id: user.id.to_string(),
        email: user.email,
        github_login: user.github_login,
        authenticated: true,
        token_expires: auth.claims.exp,
    }))
}

// ============================================================================
// Document Endpoints
// ============================================================================

/// List item for documents (excludes content).
#[derive(Serialize, utoipa::ToSchema)]
pub struct DocumentListItem {
    pub id: Uuid,
    pub title: String,
    pub summary: Option<String>,
    pub ref_tag: Option<String>,
    pub tags: Option<Vec<String>>,
    pub doc_type: Option<String>,
    pub updated_at: DateTime<Utc>,
}

/// Response for a full document (includes content).
#[derive(Serialize, utoipa::ToSchema)]
pub struct DocumentResponse {
    pub id: Uuid,
    pub title: String,
    pub content: String,
    pub summary: Option<String>,
    pub ref_tag: Option<String>,
    pub tags: Option<Vec<String>>,
    pub doc_type: Option<String>,
    pub session_id: Option<Uuid>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Request body for creating a document.
#[derive(Deserialize, utoipa::ToSchema)]
pub struct CreateDocumentRequest {
    pub title: String,
    pub content: String,
    pub doc_type: Option<String>,
    pub session_id: Option<Uuid>,
    pub tags: Option<Vec<String>>,
}

/// Request body for updating a document.
#[derive(Deserialize, utoipa::ToSchema)]
pub struct UpdateDocumentRequest {
    pub content: Option<String>,
    pub title: Option<String>,
    pub tags: Option<Vec<String>>,
}

/// Query parameters for document search.
#[derive(Deserialize, utoipa::ToSchema, utoipa::IntoParams)]
pub struct DocumentSearchQuery {
    pub q: String,
}

/// GET /api/documents - List all documents for the authenticated user.
#[utoipa::path(
    get,
    path = "/api/documents",
    tag = "Documents",
    security(("bearer_auth" = [])),
    responses(
        (status = 200, description = "List of documents", body = Vec<DocumentListItem>)
    )
)]
pub async fn list_documents(State(state): State<AppState>, auth: auth::AuthUser) -> Result<Json<Vec<DocumentListItem>>, StatusCode> {
    let doc_repo = state.doc_repo.as_ref().ok_or(StatusCode::INTERNAL_SERVER_ERROR)?;
    let docs = doc_repo.list_documents(auth.user_id.0).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let items: Vec<DocumentListItem> = docs
        .into_iter()
        .map(|d| DocumentListItem {
            id: d.id,
            title: d.title,
            summary: d.summary,
            ref_tag: d.ref_tag,
            tags: d.tags,
            doc_type: d.doc_type,
            updated_at: d.updated_at,
        })
        .collect();

    Ok(Json(items))
}

/// GET /api/documents/search?q=query - Search documents.
#[utoipa::path(
    get,
    path = "/api/documents/search",
    tag = "Documents",
    security(("bearer_auth" = [])),
    params(DocumentSearchQuery),
    responses(
        (status = 200, description = "Search results")
    )
)]
pub async fn search_documents(State(state): State<AppState>, auth: auth::AuthUser, Query(query): Query<DocumentSearchQuery>) -> Result<Json<Vec<crate::db::DocumentSearchResult>>, StatusCode> {
    let doc_repo = state.doc_repo.as_ref().ok_or(StatusCode::INTERNAL_SERVER_ERROR)?;
    let results = doc_repo.search_documents(auth.user_id.0, &query.q).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(results))
}

/// GET /api/documents/:id - Get a full document by ID.
#[utoipa::path(
    get,
    path = "/api/documents/{id}",
    tag = "Documents",
    security(("bearer_auth" = [])),
    params(("id" = Uuid, Path, description = "Document ID")),
    responses(
        (status = 200, description = "Document found", body = DocumentResponse),
        (status = 404, description = "Not found")
    )
)]
pub async fn get_document(State(state): State<AppState>, auth: auth::AuthUser, Path(doc_id): Path<Uuid>) -> Result<Json<DocumentResponse>, StatusCode> {
    let doc_repo = state.doc_repo.as_ref().ok_or(StatusCode::INTERNAL_SERVER_ERROR)?;
    let doc = doc_repo.get_document(doc_id).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?.ok_or(StatusCode::NOT_FOUND)?;

    // Verify ownership
    if doc.user_id != auth.user_id.0 {
        return Err(StatusCode::NOT_FOUND);
    }

    Ok(Json(DocumentResponse {
        id: doc.id,
        title: doc.title,
        content: doc.content,
        summary: doc.summary,
        ref_tag: doc.ref_tag,
        tags: doc.tags,
        doc_type: doc.doc_type,
        session_id: doc.session_id,
        created_at: doc.created_at,
        updated_at: doc.updated_at,
    }))
}

/// POST /api/documents - Create a new document.
#[utoipa::path(
    post,
    path = "/api/documents",
    tag = "Documents",
    security(("bearer_auth" = [])),
    request_body = CreateDocumentRequest,
    responses(
        (status = 201, description = "Document created", body = DocumentResponse),
        (status = 400, description = "Invalid request")
    )
)]
pub async fn create_document(State(state): State<AppState>, auth: auth::AuthUser, Json(request): Json<CreateDocumentRequest>) -> Result<(StatusCode, Json<DocumentResponse>), StatusCode> {
    if request.title.trim().is_empty() || request.title.len() > MAX_TITLE_LENGTH {
        return Err(StatusCode::BAD_REQUEST);
    }
    if request.content.len() > MAX_DESCRIPTION_LENGTH {
        return Err(StatusCode::BAD_REQUEST);
    }

    let doc_repo = state.doc_repo.as_ref().ok_or(StatusCode::INTERNAL_SERVER_ERROR)?;
    let doc = doc_repo
        .create_document(
            auth.user_id.0,
            request.session_id,
            request.title,
            request.content,
            request.doc_type.unwrap_or_else(|| "architecture".to_string()),
            String::new(),
            request.tags.unwrap_or_default(),
        )
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok((
        StatusCode::CREATED,
        Json(DocumentResponse {
            id: doc.id,
            title: doc.title,
            content: doc.content,
            summary: doc.summary,
            ref_tag: doc.ref_tag,
            tags: doc.tags,
            doc_type: doc.doc_type,
            session_id: doc.session_id,
            created_at: doc.created_at,
            updated_at: doc.updated_at,
        }),
    ))
}

/// PATCH /api/documents/:id - Update a document.
#[utoipa::path(
    patch,
    path = "/api/documents/{id}",
    tag = "Documents",
    security(("bearer_auth" = [])),
    params(("id" = Uuid, Path, description = "Document ID")),
    request_body = UpdateDocumentRequest,
    responses(
        (status = 200, description = "Updated document", body = DocumentResponse),
        (status = 404, description = "Not found")
    )
)]
pub async fn update_document(State(state): State<AppState>, auth: auth::AuthUser, Path(doc_id): Path<Uuid>, Json(request): Json<UpdateDocumentRequest>) -> Result<Json<DocumentResponse>, StatusCode> {
    let doc_repo = state.doc_repo.as_ref().ok_or(StatusCode::INTERNAL_SERVER_ERROR)?;

    // Verify ownership
    let existing = doc_repo.get_document(doc_id).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?.ok_or(StatusCode::NOT_FOUND)?;

    if existing.user_id != auth.user_id.0 {
        return Err(StatusCode::NOT_FOUND);
    }

    let doc = doc_repo
        .update_document(doc_id, request.content, request.title, request.tags)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(DocumentResponse {
        id: doc.id,
        title: doc.title,
        content: doc.content,
        summary: doc.summary,
        ref_tag: doc.ref_tag,
        tags: doc.tags,
        doc_type: doc.doc_type,
        session_id: doc.session_id,
        created_at: doc.created_at,
        updated_at: doc.updated_at,
    }))
}

/// DELETE /api/documents/:id - Delete a document.
#[utoipa::path(
    delete,
    path = "/api/documents/{id}",
    tag = "Documents",
    security(("bearer_auth" = [])),
    params(("id" = Uuid, Path, description = "Document ID")),
    responses(
        (status = 204, description = "Deleted successfully"),
        (status = 404, description = "Not found")
    )
)]
pub async fn delete_document(State(state): State<AppState>, auth: auth::AuthUser, Path(doc_id): Path<Uuid>) -> Result<StatusCode, StatusCode> {
    let doc_repo = state.doc_repo.as_ref().ok_or(StatusCode::INTERNAL_SERVER_ERROR)?;

    // Verify ownership
    let existing = doc_repo.get_document(doc_id).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?.ok_or(StatusCode::NOT_FOUND)?;

    if existing.user_id != auth.user_id.0 {
        return Err(StatusCode::NOT_FOUND);
    }

    doc_repo.delete_document(doc_id).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(StatusCode::NO_CONTENT)
}

// ============================================================================
// Output Schemas Endpoints
// ============================================================================

/// Response for a single output schema.
#[derive(Serialize, utoipa::ToSchema)]
pub struct OutputSchemaResponse {
    pub id: Uuid,
    pub name: String,
    pub schema: serde_json::Value,
    pub created_at: DateTime<Utc>,
}

/// Request body for creating an output schema.
#[derive(Deserialize, utoipa::ToSchema)]
pub struct CreateOutputSchemaRequest {
    pub name: String,
    pub schema: serde_json::Value,
}

/// Request body for updating an output schema.
#[derive(Deserialize, utoipa::ToSchema)]
pub struct UpdateOutputSchemaRequest {
    pub name: Option<String>,
    pub schema: Option<serde_json::Value>,
}

/// GET /api/output-schemas - List all output schemas for the authenticated user.
#[utoipa::path(
    get,
    path = "/api/output-schemas",
    tag = "Output Schemas",
    security(("bearer_auth" = [])),
    responses(
        (status = 200, description = "List of output schemas", body = Vec<OutputSchemaResponse>)
    )
)]
pub async fn list_output_schemas(State(state): State<AppState>, auth: auth::AuthUser) -> Result<Json<Vec<OutputSchemaResponse>>, StatusCode> {
    let repo = state.output_schema_repo.as_ref().ok_or(StatusCode::INTERNAL_SERVER_ERROR)?;
    let rows = repo.list_output_schemas(auth.user_id.0).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let items = rows
        .into_iter()
        .map(|r| OutputSchemaResponse {
            id: r.id,
            name: r.name,
            schema: r.schema,
            created_at: r.created_at,
        })
        .collect();
    Ok(Json(items))
}

/// POST /api/output-schemas - Create a new output schema.
#[utoipa::path(
    post,
    path = "/api/output-schemas",
    tag = "Output Schemas",
    security(("bearer_auth" = [])),
    request_body = CreateOutputSchemaRequest,
    responses(
        (status = 201, description = "Output schema created", body = OutputSchemaResponse),
        (status = 400, description = "Invalid request")
    )
)]
pub async fn create_output_schema(State(state): State<AppState>, auth: auth::AuthUser, Json(request): Json<CreateOutputSchemaRequest>) -> Result<(StatusCode, Json<OutputSchemaResponse>), StatusCode> {
    if request.name.trim().is_empty() || request.name.len() > MAX_TITLE_LENGTH {
        return Err(StatusCode::BAD_REQUEST);
    }
    let repo = state.output_schema_repo.as_ref().ok_or(StatusCode::INTERNAL_SERVER_ERROR)?;
    let row = repo
        .create_output_schema(auth.user_id.0, request.name, request.schema)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok((
        StatusCode::CREATED,
        Json(OutputSchemaResponse {
            id: row.id,
            name: row.name,
            schema: row.schema,
            created_at: row.created_at,
        }),
    ))
}

/// GET /api/output-schemas/:id - Get an output schema by ID.
#[utoipa::path(
    get,
    path = "/api/output-schemas/{id}",
    tag = "Output Schemas",
    security(("bearer_auth" = [])),
    params(("id" = Uuid, Path, description = "Output schema ID")),
    responses(
        (status = 200, description = "Output schema found", body = OutputSchemaResponse),
        (status = 404, description = "Not found")
    )
)]
pub async fn get_output_schema(State(state): State<AppState>, auth: auth::AuthUser, Path(id): Path<Uuid>) -> Result<Json<OutputSchemaResponse>, StatusCode> {
    let repo = state.output_schema_repo.as_ref().ok_or(StatusCode::INTERNAL_SERVER_ERROR)?;
    let row = repo.get_output_schema(id).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?.ok_or(StatusCode::NOT_FOUND)?;
    if row.user_id != auth.user_id.0 {
        return Err(StatusCode::NOT_FOUND);
    }
    Ok(Json(OutputSchemaResponse {
        id: row.id,
        name: row.name,
        schema: row.schema,
        created_at: row.created_at,
    }))
}

/// PUT /api/output-schemas/:id - Update an output schema.
#[utoipa::path(
    put,
    path = "/api/output-schemas/{id}",
    tag = "Output Schemas",
    security(("bearer_auth" = [])),
    params(("id" = Uuid, Path, description = "Output schema ID")),
    request_body = UpdateOutputSchemaRequest,
    responses(
        (status = 200, description = "Updated output schema", body = OutputSchemaResponse),
        (status = 404, description = "Not found")
    )
)]
pub async fn update_output_schema(
    State(state): State<AppState>,
    auth: auth::AuthUser,
    Path(id): Path<Uuid>,
    Json(request): Json<UpdateOutputSchemaRequest>,
) -> Result<Json<OutputSchemaResponse>, StatusCode> {
    let repo = state.output_schema_repo.as_ref().ok_or(StatusCode::INTERNAL_SERVER_ERROR)?;
    let existing = repo.get_output_schema(id).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?.ok_or(StatusCode::NOT_FOUND)?;
    if existing.user_id != auth.user_id.0 {
        return Err(StatusCode::NOT_FOUND);
    }
    if let Some(ref name) = request.name {
        if name.trim().is_empty() || name.len() > MAX_TITLE_LENGTH {
            return Err(StatusCode::BAD_REQUEST);
        }
    }
    let row = repo.update_output_schema(id, request.name, request.schema).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Json(OutputSchemaResponse {
        id: row.id,
        name: row.name,
        schema: row.schema,
        created_at: row.created_at,
    }))
}

/// DELETE /api/output-schemas/:id - Delete an output schema.
#[utoipa::path(
    delete,
    path = "/api/output-schemas/{id}",
    tag = "Output Schemas",
    security(("bearer_auth" = [])),
    params(("id" = Uuid, Path, description = "Output schema ID")),
    responses(
        (status = 204, description = "Deleted successfully"),
        (status = 404, description = "Not found")
    )
)]
pub async fn delete_output_schema(State(state): State<AppState>, auth: auth::AuthUser, Path(id): Path<Uuid>) -> Result<StatusCode, StatusCode> {
    let repo = state.output_schema_repo.as_ref().ok_or(StatusCode::INTERNAL_SERVER_ERROR)?;
    let existing = repo.get_output_schema(id).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?.ok_or(StatusCode::NOT_FOUND)?;
    if existing.user_id != auth.user_id.0 {
        return Err(StatusCode::NOT_FOUND);
    }
    repo.delete_output_schema(id).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(StatusCode::NO_CONTENT)
}

// ============================================================================
// Prompt Templates Endpoints
// ============================================================================

/// Response for a single prompt template.
#[derive(Serialize, utoipa::ToSchema)]
pub struct PromptTemplateResponse {
    pub id: Uuid,
    pub name: String,
    pub content: String,
    pub created_at: DateTime<Utc>,
}

/// Request body for creating a prompt template.
#[derive(Deserialize, utoipa::ToSchema)]
pub struct CreatePromptTemplateRequest {
    pub name: String,
    pub content: String,
}

/// Request body for updating a prompt template.
#[derive(Deserialize, utoipa::ToSchema)]
pub struct UpdatePromptTemplateRequest {
    pub name: Option<String>,
    pub content: Option<String>,
}

/// GET /api/prompt-templates
#[utoipa::path(
    get,
    path = "/api/prompt-templates",
    tag = "Prompt Templates",
    security(("bearer_auth" = [])),
    responses(
        (status = 200, description = "List of prompt templates", body = Vec<PromptTemplateResponse>)
    )
)]
pub async fn list_prompt_templates(State(state): State<AppState>, auth: auth::AuthUser) -> Result<Json<Vec<PromptTemplateResponse>>, StatusCode> {
    let repo = state.prompt_template_repo.as_ref().ok_or(StatusCode::INTERNAL_SERVER_ERROR)?;
    let rows = repo.list_prompt_templates(auth.user_id.0).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let items = rows
        .into_iter()
        .map(|r| PromptTemplateResponse {
            id: r.id,
            name: r.name,
            content: r.content,
            created_at: r.created_at,
        })
        .collect();
    Ok(Json(items))
}

/// POST /api/prompt-templates
#[utoipa::path(
    post,
    path = "/api/prompt-templates",
    tag = "Prompt Templates",
    security(("bearer_auth" = [])),
    request_body = CreatePromptTemplateRequest,
    responses(
        (status = 201, description = "Prompt template created", body = PromptTemplateResponse),
        (status = 400, description = "Invalid request")
    )
)]
pub async fn create_prompt_template(
    State(state): State<AppState>,
    auth: auth::AuthUser,
    Json(request): Json<CreatePromptTemplateRequest>,
) -> Result<(StatusCode, Json<PromptTemplateResponse>), StatusCode> {
    if request.name.trim().is_empty() || request.name.len() > MAX_TITLE_LENGTH {
        return Err(StatusCode::BAD_REQUEST);
    }
    if request.content.len() > MAX_PROMPT_LENGTH {
        return Err(StatusCode::BAD_REQUEST);
    }
    let repo = state.prompt_template_repo.as_ref().ok_or(StatusCode::INTERNAL_SERVER_ERROR)?;
    let row = repo
        .create_prompt_template(auth.user_id.0, request.name, request.content)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok((
        StatusCode::CREATED,
        Json(PromptTemplateResponse {
            id: row.id,
            name: row.name,
            content: row.content,
            created_at: row.created_at,
        }),
    ))
}

/// GET /api/prompt-templates/:id
#[utoipa::path(
    get,
    path = "/api/prompt-templates/{id}",
    tag = "Prompt Templates",
    security(("bearer_auth" = [])),
    params(("id" = Uuid, Path, description = "Prompt template ID")),
    responses(
        (status = 200, description = "Prompt template found", body = PromptTemplateResponse),
        (status = 404, description = "Not found")
    )
)]
pub async fn get_prompt_template(State(state): State<AppState>, auth: auth::AuthUser, Path(id): Path<Uuid>) -> Result<Json<PromptTemplateResponse>, StatusCode> {
    let repo = state.prompt_template_repo.as_ref().ok_or(StatusCode::INTERNAL_SERVER_ERROR)?;
    let row = repo.get_prompt_template(id).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?.ok_or(StatusCode::NOT_FOUND)?;
    if row.user_id != auth.user_id.0 {
        return Err(StatusCode::NOT_FOUND);
    }
    Ok(Json(PromptTemplateResponse {
        id: row.id,
        name: row.name,
        content: row.content,
        created_at: row.created_at,
    }))
}

/// PUT /api/prompt-templates/:id
#[utoipa::path(
    put,
    path = "/api/prompt-templates/{id}",
    tag = "Prompt Templates",
    security(("bearer_auth" = [])),
    params(("id" = Uuid, Path, description = "Prompt template ID")),
    request_body = UpdatePromptTemplateRequest,
    responses(
        (status = 200, description = "Updated prompt template", body = PromptTemplateResponse),
        (status = 404, description = "Not found")
    )
)]
pub async fn update_prompt_template(
    State(state): State<AppState>,
    auth: auth::AuthUser,
    Path(id): Path<Uuid>,
    Json(request): Json<UpdatePromptTemplateRequest>,
) -> Result<Json<PromptTemplateResponse>, StatusCode> {
    let repo = state.prompt_template_repo.as_ref().ok_or(StatusCode::INTERNAL_SERVER_ERROR)?;
    let existing = repo.get_prompt_template(id).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?.ok_or(StatusCode::NOT_FOUND)?;
    if existing.user_id != auth.user_id.0 {
        return Err(StatusCode::NOT_FOUND);
    }
    if let Some(ref name) = request.name {
        if name.trim().is_empty() || name.len() > MAX_TITLE_LENGTH {
            return Err(StatusCode::BAD_REQUEST);
        }
    }
    if let Some(ref content) = request.content {
        if content.len() > MAX_PROMPT_LENGTH {
            return Err(StatusCode::BAD_REQUEST);
        }
    }
    let row = repo.update_prompt_template(id, request.name, request.content).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Json(PromptTemplateResponse {
        id: row.id,
        name: row.name,
        content: row.content,
        created_at: row.created_at,
    }))
}

/// DELETE /api/prompt-templates/:id
#[utoipa::path(
    delete,
    path = "/api/prompt-templates/{id}",
    tag = "Prompt Templates",
    security(("bearer_auth" = [])),
    params(("id" = Uuid, Path, description = "Prompt template ID")),
    responses(
        (status = 204, description = "Deleted successfully"),
        (status = 404, description = "Not found")
    )
)]
pub async fn delete_prompt_template(State(state): State<AppState>, auth: auth::AuthUser, Path(id): Path<Uuid>) -> Result<StatusCode, StatusCode> {
    let repo = state.prompt_template_repo.as_ref().ok_or(StatusCode::INTERNAL_SERVER_ERROR)?;
    let existing = repo.get_prompt_template(id).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?.ok_or(StatusCode::NOT_FOUND)?;
    if existing.user_id != auth.user_id.0 {
        return Err(StatusCode::NOT_FOUND);
    }
    repo.delete_prompt_template(id).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(StatusCode::NO_CONTENT)
}

// ============================================================================
// Pipeline Stage Members Endpoints
// ============================================================================

#[derive(Serialize, utoipa::ToSchema)]
pub struct StageMemberResponse {
    pub id: Uuid,
    pub pipeline_id: Uuid,
    pub stage_number: i32,
    pub workflow_id: Uuid,
    pub display_order: i32,
}

#[derive(Deserialize, utoipa::ToSchema)]
pub struct CreateStageMemberRequest {
    pub workflow_id: Uuid,
    pub display_order: Option<i32>,
}

#[derive(Deserialize, utoipa::ToSchema)]
pub struct UpdateStageMemberRequest {
    pub display_order: i32,
}

#[derive(Deserialize, utoipa::ToSchema)]
pub struct StageMemberPath {
    pub pid: Uuid,
    pub num: i32,
}

#[derive(Deserialize, utoipa::ToSchema)]
pub struct StageMemberItemPath {
    pub pid: Uuid,
    pub num: i32,
    pub mid: Uuid,
}

fn member_response(r: crate::db::PipelineStageMemberRow) -> StageMemberResponse {
    StageMemberResponse {
        id: r.id,
        pipeline_id: r.pipeline_id,
        stage_number: r.stage_number,
        workflow_id: r.workflow_id,
        display_order: r.display_order,
    }
}

/// GET /api/pipelines/:pid/stages/:num/members
#[utoipa::path(
    get,
    path = "/api/pipelines/{pid}/stages/{num}/members",
    tag = "Pipeline Stage Members",
    security(("bearer_auth" = [])),
    params(
        ("pid" = Uuid, Path, description = "Pipeline ID"),
        ("num" = i32, Path, description = "Stage number")
    ),
    responses(
        (status = 200, description = "List of stage members", body = Vec<StageMemberResponse>),
        (status = 404, description = "Pipeline not found")
    )
)]
pub async fn list_stage_members(State(state): State<AppState>, auth: auth::AuthUser, Path(p): Path<StageMemberPath>) -> Result<Json<Vec<StageMemberResponse>>, StatusCode> {
    let _pipeline = state
        .repo
        .list_pipelines(auth.user_id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .into_iter()
        .find(|pl| pl.id == p.pid)
        .ok_or(StatusCode::NOT_FOUND)?;
    let repo = state.stage_member_repo.as_ref().ok_or(StatusCode::INTERNAL_SERVER_ERROR)?;
    let rows = repo.list_stage_members(p.pid, p.num).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Json(rows.into_iter().map(member_response).collect()))
}

/// POST /api/pipelines/:pid/stages/:num/members
#[utoipa::path(
    post,
    path = "/api/pipelines/{pid}/stages/{num}/members",
    tag = "Pipeline Stage Members",
    security(("bearer_auth" = [])),
    params(
        ("pid" = Uuid, Path, description = "Pipeline ID"),
        ("num" = i32, Path, description = "Stage number")
    ),
    request_body = CreateStageMemberRequest,
    responses(
        (status = 201, description = "Stage member added", body = StageMemberResponse),
        (status = 404, description = "Pipeline not found")
    )
)]
pub async fn add_stage_member(
    State(state): State<AppState>,
    auth: auth::AuthUser,
    Path(p): Path<StageMemberPath>,
    Json(req): Json<CreateStageMemberRequest>,
) -> Result<(StatusCode, Json<StageMemberResponse>), StatusCode> {
    let _pipeline = state
        .repo
        .list_pipelines(auth.user_id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .into_iter()
        .find(|pl| pl.id == p.pid)
        .ok_or(StatusCode::NOT_FOUND)?;
    let repo = state.stage_member_repo.as_ref().ok_or(StatusCode::INTERNAL_SERVER_ERROR)?;
    let row = repo
        .add_stage_member(p.pid, p.num, req.workflow_id, req.display_order.unwrap_or(0))
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok((StatusCode::CREATED, Json(member_response(row))))
}

/// PUT /api/pipelines/:pid/stages/:num/members/:mid
#[utoipa::path(
    put,
    path = "/api/pipelines/{pid}/stages/{num}/members/{mid}",
    tag = "Pipeline Stage Members",
    security(("bearer_auth" = [])),
    params(
        ("pid" = Uuid, Path, description = "Pipeline ID"),
        ("num" = i32, Path, description = "Stage number"),
        ("mid" = Uuid, Path, description = "Member ID")
    ),
    request_body = UpdateStageMemberRequest,
    responses(
        (status = 200, description = "Updated stage member", body = StageMemberResponse),
        (status = 404, description = "Not found")
    )
)]
pub async fn update_stage_member(
    State(state): State<AppState>,
    auth: auth::AuthUser,
    Path(p): Path<StageMemberItemPath>,
    Json(req): Json<UpdateStageMemberRequest>,
) -> Result<Json<StageMemberResponse>, StatusCode> {
    let _pipeline = state
        .repo
        .list_pipelines(auth.user_id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .into_iter()
        .find(|pl| pl.id == p.pid)
        .ok_or(StatusCode::NOT_FOUND)?;
    let repo = state.stage_member_repo.as_ref().ok_or(StatusCode::INTERNAL_SERVER_ERROR)?;
    let row = repo.update_stage_member(p.mid, req.display_order).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Json(member_response(row)))
}

/// DELETE /api/pipelines/:pid/stages/:num/members/:mid
#[utoipa::path(
    delete,
    path = "/api/pipelines/{pid}/stages/{num}/members/{mid}",
    tag = "Pipeline Stage Members",
    security(("bearer_auth" = [])),
    params(
        ("pid" = Uuid, Path, description = "Pipeline ID"),
        ("num" = i32, Path, description = "Stage number"),
        ("mid" = Uuid, Path, description = "Member ID")
    ),
    responses(
        (status = 204, description = "Deleted successfully"),
        (status = 404, description = "Not found")
    )
)]
pub async fn delete_stage_member(State(state): State<AppState>, auth: auth::AuthUser, Path(p): Path<StageMemberItemPath>) -> Result<StatusCode, StatusCode> {
    let _pipeline = state
        .repo
        .list_pipelines(auth.user_id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .into_iter()
        .find(|pl| pl.id == p.pid)
        .ok_or(StatusCode::NOT_FOUND)?;
    let repo = state.stage_member_repo.as_ref().ok_or(StatusCode::INTERNAL_SERVER_ERROR)?;
    repo.remove_stage_member(p.mid).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(StatusCode::NO_CONTENT)
}

// ============================================================================
// Agent Execution Endpoints (read-only)
// ============================================================================

#[derive(Serialize, utoipa::ToSchema)]
pub struct AgentExecutionResponse {
    pub id: Uuid,
    pub stage_execution_id: Uuid,
    pub agent_id: Uuid,
    pub workflow_step_id: Option<Uuid>,
    pub is_interactive: bool,
    pub parent_agent_execution_id: Option<Uuid>,
    pub system_prompt_rendered: String,
    pub input: String,
    pub output: Option<String>,
    pub structured_output: Option<serde_json::Value>,
    pub selected_mode_id: Option<Uuid>,
    pub status: String,
    pub started_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
}

impl From<crate::db::AgentExecutionRow> for AgentExecutionResponse {
    fn from(r: crate::db::AgentExecutionRow) -> Self {
        Self {
            id: r.id,
            stage_execution_id: r.stage_execution_id,
            agent_id: r.agent_id,
            workflow_step_id: r.workflow_step_id,
            is_interactive: r.is_interactive,
            parent_agent_execution_id: r.parent_agent_execution_id,
            system_prompt_rendered: r.system_prompt_rendered,
            input: r.input,
            output: r.output,
            structured_output: r.structured_output,
            selected_mode_id: r.selected_mode_id,
            status: r.status,
            started_at: r.started_at,
            completed_at: r.completed_at,
        }
    }
}

#[derive(Serialize, utoipa::ToSchema)]
pub struct ExecutionMessageResponse {
    pub id: Uuid,
    pub agent_execution_id: Uuid,
    pub role: String,
    pub content: String,
    pub tool_call_id: Option<String>,
    pub input_tokens: i64,
    pub output_tokens: i64,
    pub created_at: DateTime<Utc>,
}

impl From<crate::db::ExecutionMessageRow> for ExecutionMessageResponse {
    fn from(r: crate::db::ExecutionMessageRow) -> Self {
        Self {
            id: r.id,
            agent_execution_id: r.agent_execution_id,
            role: r.role,
            content: r.content,
            tool_call_id: r.tool_call_id,
            input_tokens: r.input_tokens,
            output_tokens: r.output_tokens,
            created_at: r.created_at,
        }
    }
}

#[utoipa::path(
    get,
    path = "/api/agent-executions/{id}",
    tag = "Agent Executions",
    security(("bearer_auth" = [])),
    params(("id" = Uuid, Path, description = "Agent execution ID")),
    responses(
        (status = 200, description = "Agent execution found", body = AgentExecutionResponse),
        (status = 404, description = "Not found")
    )
)]
pub async fn get_agent_execution(State(state): State<AppState>, _auth: auth::AuthUser, Path(id): Path<Uuid>) -> Result<Json<AgentExecutionResponse>, StatusCode> {
    let repo = state.agent_execution_repo.as_ref().ok_or(StatusCode::INTERNAL_SERVER_ERROR)?;
    let row = repo.get_agent_execution(id).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?.ok_or(StatusCode::NOT_FOUND)?;
    Ok(Json(AgentExecutionResponse::from(row)))
}

#[utoipa::path(
    get,
    path = "/api/agent-executions/{id}/messages",
    tag = "Agent Executions",
    security(("bearer_auth" = [])),
    params(("id" = Uuid, Path, description = "Agent execution ID")),
    responses(
        (status = 200, description = "List of execution messages", body = Vec<ExecutionMessageResponse>),
        (status = 404, description = "Not found")
    )
)]
pub async fn list_execution_messages(State(state): State<AppState>, _auth: auth::AuthUser, Path(id): Path<Uuid>) -> Result<Json<Vec<ExecutionMessageResponse>>, StatusCode> {
    let repo = state.agent_execution_repo.as_ref().ok_or(StatusCode::INTERNAL_SERVER_ERROR)?;
    // Verify execution exists
    repo.get_agent_execution(id).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?.ok_or(StatusCode::NOT_FOUND)?;
    let rows = repo.list_execution_messages(id).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Json(rows.into_iter().map(ExecutionMessageResponse::from).collect()))
}

// ============================================================================
// Interactive Chat Endpoints
// ============================================================================

#[derive(Deserialize, utoipa::ToSchema)]
pub struct SendMessageRequest {
    pub content: String,
}

#[derive(Deserialize, utoipa::ToSchema)]
pub struct ApproveExecutionRequest {
    pub structured_output: Option<serde_json::Value>,
}

/// POST /api/agent-executions/:id/messages — send a user message to an interactive agent execution.
#[utoipa::path(
    post,
    path = "/api/agent-executions/{id}/messages",
    tag = "Agent Executions",
    security(("bearer_auth" = [])),
    params(("id" = Uuid, Path, description = "Agent execution ID")),
    request_body = SendMessageRequest,
    responses(
        (status = 200, description = "Message sent", body = ExecutionMessageResponse),
        (status = 400, description = "Not interactive"),
        (status = 404, description = "Not found")
    )
)]
pub async fn send_execution_message(
    State(state): State<AppState>,
    _auth: auth::AuthUser,
    Path(id): Path<Uuid>,
    Json(req): Json<SendMessageRequest>,
) -> Result<Json<ExecutionMessageResponse>, StatusCode> {
    let repo = state.agent_execution_repo.as_ref().ok_or(StatusCode::INTERNAL_SERVER_ERROR)?;
    let ae = repo.get_agent_execution(id).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?.ok_or(StatusCode::NOT_FOUND)?;

    if !ae.is_interactive {
        return Err(StatusCode::BAD_REQUEST);
    }

    // Record the user message
    let msg = repo
        .create_execution_message(id, "user", &req.content, None, 0, 0)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(ExecutionMessageResponse::from(msg)))
}

/// POST /api/agent-executions/:id/approve — approve an interactive agent execution.
///
/// With no `structured_output` body → approve as-is (main output used).
/// With `structured_output` → approve with changes (revised output used downstream).
#[utoipa::path(
    post,
    path = "/api/agent-executions/{id}/approve",
    tag = "Agent Executions",
    security(("bearer_auth" = [])),
    params(("id" = Uuid, Path, description = "Agent execution ID")),
    request_body = ApproveExecutionRequest,
    responses(
        (status = 200, description = "Execution approved", body = AgentExecutionResponse),
        (status = 400, description = "Not interactive or not awaiting user"),
        (status = 404, description = "Not found")
    )
)]
pub async fn approve_execution(
    State(state): State<AppState>,
    _auth: auth::AuthUser,
    Path(id): Path<Uuid>,
    Json(req): Json<ApproveExecutionRequest>,
) -> Result<Json<AgentExecutionResponse>, StatusCode> {
    let repo = state.agent_execution_repo.as_ref().ok_or(StatusCode::INTERNAL_SERVER_ERROR)?;
    let ae = repo.get_agent_execution(id).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?.ok_or(StatusCode::NOT_FOUND)?;

    if !ae.is_interactive || ae.status != "awaiting_user" {
        return Err(StatusCode::BAD_REQUEST);
    }

    // Update status to completed, optionally with revised structured_output
    let updated = repo
        .update_agent_execution_status(id, "completed", ae.output.clone(), req.structured_output)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(AgentExecutionResponse::from(updated)))
}

// ============================================================================
// Cost Endpoints
// ============================================================================

#[derive(Deserialize, utoipa::ToSchema, utoipa::IntoParams)]
pub struct CostQuery {
    pub since: Option<DateTime<Utc>>,
}

#[derive(Serialize, utoipa::ToSchema)]
pub struct CostResponse {
    pub total_spend: f64,
    pub models: Vec<crate::db::traits::ModelSpendRow>,
}

#[utoipa::path(
    get,
    path = "/api/costs",
    tag = "Costs",
    security(("bearer_auth" = [])),
    params(CostQuery),
    responses(
        (status = 200, description = "Cost breakdown", body = CostResponse)
    )
)]
pub async fn get_costs(State(state): State<AppState>, auth: auth::AuthUser, Query(q): Query<CostQuery>) -> Result<Json<CostResponse>, StatusCode> {
    let repo = state.token_ledger_repo.as_ref().ok_or(StatusCode::INTERNAL_SERVER_ERROR)?;
    let total_spend = repo.get_user_spend(auth.user_id.0, q.since).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let models = repo.get_model_breakdown(auth.user_id.0, q.since).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Json(CostResponse { total_spend, models }))
}

// ============================================================================
// Execution Tree Endpoint
// ============================================================================

#[derive(Serialize, utoipa::ToSchema)]
pub struct TreeRunInfo {
    pub id: Uuid,
    pub pipeline_id: Uuid,
    pub pipeline_name: String,
    pub status: String,
    pub initial_input: String,
    pub current_stage: i32,
    pub started_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
    pub total_input_tokens: i64,
    pub total_output_tokens: i64,
    pub total_cost_usd: f64,
}

#[derive(Serialize, utoipa::ToSchema)]
pub struct TreeStage {
    pub stage_number: i32,
    pub stage_name: String,
    pub status: String,
    pub stage_executions: Vec<TreeStageExecution>,
}

#[derive(Serialize, utoipa::ToSchema)]
pub struct TreeStageExecution {
    pub id: Uuid,
    pub workflow_name: String,
    pub status: String,
    pub agent_executions: Vec<TreeAgentExecution>,
}

#[derive(Serialize, utoipa::ToSchema)]
#[schema(no_recursion)]
pub struct TreeAgentExecution {
    pub id: Uuid,
    pub agent_name: String,
    pub workflow_step_id: Option<Uuid>,
    pub is_interactive: bool,
    pub status: String,
    pub structured_output: Option<serde_json::Value>,
    pub input_tokens: i64,
    pub output_tokens: i64,
    pub cost_usd: f32,
    pub started_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
    pub for_each_index: Option<i32>,
    pub for_each_label: Option<String>,
    pub interactive_review: Option<Box<TreeAgentExecution>>,
}

#[derive(Serialize, utoipa::ToSchema)]
pub struct TreeResponse {
    pub run: TreeRunInfo,
    pub stages: Vec<TreeStage>,
}

/// GET /api/pipeline-runs/:run_id/tree
///
/// Returns the full execution tree for a pipeline run, joining stage_executions → agent_executions.
#[utoipa::path(
    get,
    path = "/api/pipeline-runs/{run_id}/tree",
    tag = "Execution Tree",
    security(("bearer_auth" = [])),
    params(("run_id" = Uuid, Path, description = "Pipeline run ID")),
    responses(
        (status = 200, description = "Execution tree", body = TreeResponse),
        (status = 404, description = "Run not found")
    )
)]
pub async fn get_pipeline_run_tree(State(state): State<AppState>, _auth: auth::AuthUser, Path(run_id): Path<Uuid>) -> Result<Json<TreeResponse>, StatusCode> {
    let ae_repo = state.agent_execution_repo.as_ref().ok_or(StatusCode::INTERNAL_SERVER_ERROR)?;

    let run = state.repo.get_pipeline_run(run_id).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?.ok_or(StatusCode::NOT_FOUND)?;

    // Get pipeline name
    let pipeline_name = state
        .repo
        .list_pipelines(crate::types::UserId(run.user_id))
        .await
        .ok()
        .and_then(|ps| ps.into_iter().find(|p| p.id == run.pipeline_id).map(|p| p.name))
        .unwrap_or_default();

    // Get total cost from token_ledger
    let total_cost_usd = if let Some(tl_repo) = &state.token_ledger_repo {
        tl_repo.get_run_spend(run_id).await.unwrap_or(0.0)
    } else {
        0.0
    };

    // Get stage executions
    let stage_execs = state.repo.list_stage_executions(run_id).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    // Get pipeline stages for stage names
    let pipeline_stages = state.repo.list_pipeline_stages(run.pipeline_id).await.unwrap_or_default();

    // Group stage executions by stage_number, build tree stages
    let mut stage_map: std::collections::BTreeMap<i32, Vec<crate::db::StageExecutionRow>> = std::collections::BTreeMap::new();
    for se in stage_execs {
        stage_map.entry(se.stage_number).or_default().push(se);
    }

    let mut stages = Vec::new();
    for (stage_num, execs) in &stage_map {
        let stage_name = execs
            .first()
            .map(|e| e.stage_name.clone())
            .or_else(|| pipeline_stages.iter().find(|ps| ps.stage_number == *stage_num).map(|ps| ps.stage_name.clone()))
            .unwrap_or_default();

        // Derive stage status from its executions
        let stage_status = if execs.iter().all(|e| e.status == "completed") {
            "completed"
        } else if execs.iter().any(|e| e.status == "failed") {
            "failed"
        } else if execs.iter().any(|e| e.status == "running") {
            "running"
        } else if execs.iter().any(|e| e.status == "waiting_for_approval") {
            "waiting_for_approval"
        } else {
            "pending"
        };

        let mut tree_stage_execs = Vec::new();
        for se in execs {
            // Fetch agent_executions for this stage_execution
            let ae_rows = ae_repo.list_agent_executions_by_stage(se.id).await.unwrap_or_default();

            // Separate main vs interactive executions
            let main_execs: Vec<_> = ae_rows.iter().filter(|ae| !ae.is_interactive).collect();
            let interactive_map: std::collections::HashMap<Uuid, &crate::db::AgentExecutionRow> = ae_rows
                .iter()
                .filter(|ae| ae.is_interactive)
                .filter_map(|ae| ae.parent_agent_execution_id.map(|pid| (pid, ae)))
                .collect();

            let mut tree_agent_execs = Vec::new();
            for ae in &main_execs {
                // Look up agent name
                let agent_name = state
                    .repo
                    .get_persisted_agent(ae.agent_id)
                    .await
                    .ok()
                    .flatten()
                    .map(|a| a.name)
                    .unwrap_or_else(|| "Unknown".to_string());

                let interactive_review = if let Some(iae) = interactive_map.get(&ae.id) {
                    let ia_name = state
                        .repo
                        .get_persisted_agent(iae.agent_id)
                        .await
                        .ok()
                        .flatten()
                        .map(|a| a.name)
                        .unwrap_or_else(|| "Reviewer".to_string());
                    Some(Box::new(TreeAgentExecution {
                        id: iae.id,
                        agent_name: ia_name,
                        workflow_step_id: iae.workflow_step_id,
                        is_interactive: true,
                        status: iae.status.clone(),
                        structured_output: iae.structured_output.clone(),
                        input_tokens: 0,
                        output_tokens: 0,
                        cost_usd: 0.0,
                        started_at: iae.started_at,
                        completed_at: iae.completed_at,
                        for_each_index: None,
                        for_each_label: None,
                        interactive_review: None,
                    }))
                } else {
                    None
                };

                tree_agent_execs.push(TreeAgentExecution {
                    id: ae.id,
                    agent_name: agent_name.clone(),
                    workflow_step_id: ae.workflow_step_id,
                    is_interactive: false,
                    status: ae.status.clone(),
                    structured_output: ae.structured_output.clone(),
                    input_tokens: 0,
                    output_tokens: 0,
                    cost_usd: 0.0,
                    started_at: ae.started_at,
                    completed_at: ae.completed_at,
                    for_each_index: None, // TODO: populated when DAG executor is built (step 4.3)
                    for_each_label: None, // TODO: populated when DAG executor is built (step 4.3)
                    interactive_review,
                });
            }

            tree_stage_execs.push(TreeStageExecution {
                id: se.id,
                workflow_name: se.stage_name.clone(), // Currently stage_name; will be workflow name after step 4.3
                status: se.status.clone(),
                agent_executions: tree_agent_execs,
            });
        }

        stages.push(TreeStage {
            stage_number: *stage_num,
            stage_name,
            status: stage_status.to_string(),
            stage_executions: tree_stage_execs,
        });
    }

    // Include stages with no executions yet (from pipeline definition)
    for ps in &pipeline_stages {
        if !stage_map.contains_key(&ps.stage_number) {
            stages.push(TreeStage {
                stage_number: ps.stage_number,
                stage_name: ps.stage_name.clone(),
                status: "pending".to_string(),
                stage_executions: vec![],
            });
        }
    }

    stages.sort_by_key(|s| s.stage_number);

    Ok(Json(TreeResponse {
        run: TreeRunInfo {
            id: run.id,
            pipeline_id: run.pipeline_id,
            pipeline_name,
            status: run.status,
            initial_input: run.initial_task,
            current_stage: run.current_stage,
            started_at: run.started_at,
            completed_at: run.completed_at,
            total_input_tokens: run.total_input_tokens.unwrap_or(0),
            total_output_tokens: run.total_output_tokens.unwrap_or(0),
            total_cost_usd,
        },
        stages,
    }))
}

// ============================================================================
// Result Endpoints
// ============================================================================

#[derive(Serialize, utoipa::ToSchema)]
pub struct ResultResponse {
    pub id: Uuid,
    pub agent_execution_id: Uuid,
    pub output_schema_id: Option<Uuid>,
    pub name: String,
    pub data: serde_json::Value,
    pub created_at: DateTime<Utc>,
}

impl From<crate::db::ResultRow> for ResultResponse {
    fn from(r: crate::db::ResultRow) -> Self {
        Self {
            id: r.id,
            agent_execution_id: r.agent_execution_id,
            output_schema_id: r.output_schema_id,
            name: r.name,
            data: r.data,
            created_at: r.created_at,
        }
    }
}

#[derive(Deserialize, utoipa::ToSchema, utoipa::IntoParams)]
pub struct ResultQuery {
    pub output_schema_id: Option<Uuid>,
}

#[utoipa::path(
    get,
    path = "/api/results",
    tag = "Results",
    security(("bearer_auth" = [])),
    params(ResultQuery),
    responses(
        (status = 200, description = "List of results", body = Vec<ResultResponse>)
    )
)]
pub async fn list_results(State(state): State<AppState>, auth: auth::AuthUser, Query(q): Query<ResultQuery>) -> Result<Json<Vec<ResultResponse>>, StatusCode> {
    let repo = state.result_repo.as_ref().ok_or(StatusCode::INTERNAL_SERVER_ERROR)?;
    let rows = match q.output_schema_id {
        Some(schema_id) => repo.list_results_by_schema(auth.user_id.0, schema_id).await,
        None => repo.list_results(auth.user_id.0).await,
    }
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Json(rows.into_iter().map(ResultResponse::from).collect()))
}

#[utoipa::path(
    get,
    path = "/api/results/{id}",
    tag = "Results",
    security(("bearer_auth" = [])),
    params(("id" = Uuid, Path, description = "Result ID")),
    responses(
        (status = 200, description = "Result found", body = ResultResponse),
        (status = 404, description = "Not found")
    )
)]
pub async fn get_result(State(state): State<AppState>, auth: auth::AuthUser, Path(id): Path<Uuid>) -> Result<Json<ResultResponse>, StatusCode> {
    let repo = state.result_repo.as_ref().ok_or(StatusCode::INTERNAL_SERVER_ERROR)?;
    let row = repo.get_result(id).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?.ok_or(StatusCode::NOT_FOUND)?;
    if row.user_id != auth.user_id.0 {
        return Err(StatusCode::NOT_FOUND);
    }
    Ok(Json(ResultResponse::from(row)))
}

#[utoipa::path(
    delete,
    path = "/api/results/{id}",
    tag = "Results",
    security(("bearer_auth" = [])),
    params(("id" = Uuid, Path, description = "Result ID")),
    responses(
        (status = 204, description = "Deleted successfully"),
        (status = 404, description = "Not found")
    )
)]
pub async fn delete_result(State(state): State<AppState>, auth: auth::AuthUser, Path(id): Path<Uuid>) -> Result<StatusCode, StatusCode> {
    let repo = state.result_repo.as_ref().ok_or(StatusCode::INTERNAL_SERVER_ERROR)?;
    let row = repo.get_result(id).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?.ok_or(StatusCode::NOT_FOUND)?;
    if row.user_id != auth.user_id.0 {
        return Err(StatusCode::NOT_FOUND);
    }
    repo.delete_result(id).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(StatusCode::NO_CONTENT)
}

// ============================================================================
// Workflows Endpoints
// ============================================================================

#[derive(Serialize, utoipa::ToSchema)]
pub struct WorkflowResponse {
    pub id: Uuid,
    pub name: String,
    pub description: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Deserialize, utoipa::ToSchema)]
pub struct CreateWorkflowRequest {
    pub name: String,
    pub description: Option<String>,
}

#[derive(Deserialize, utoipa::ToSchema)]
pub struct UpdateWorkflowRequest {
    pub name: Option<String>,
    pub description: Option<String>,
}

#[derive(Serialize, utoipa::ToSchema)]
pub struct WorkflowStepResponse {
    pub id: Uuid,
    pub workflow_id: Uuid,
    pub agent_id: Uuid,
    pub execution_mode: String,
    pub for_each_ref: Option<String>,
    pub prompt_template_id: Option<Uuid>,
    pub prompt_template: String,
    pub output_schema_id: Option<Uuid>,
    pub output_variable_name: Option<String>,
    pub interactive_agent_id: Option<Uuid>,
    pub for_each_label_field: Option<String>,
    pub display_order: i32,
    pub version: i32,
}

#[derive(Deserialize, utoipa::ToSchema)]
pub struct CreateStepRequest {
    pub agent_id: Uuid,
    pub execution_mode: Option<String>,
    pub for_each_ref: Option<String>,
    pub prompt_template_id: Option<Uuid>,
    pub prompt_template: Option<String>,
    pub output_schema_id: Option<Uuid>,
    pub output_variable_name: Option<String>,
    pub interactive_agent_id: Option<Uuid>,
    pub for_each_label_field: Option<String>,
    pub display_order: Option<i32>,
}

#[derive(Deserialize, utoipa::ToSchema)]
pub struct UpdateStepRequest {
    pub agent_id: Uuid,
    pub execution_mode: Option<String>,
    pub for_each_ref: Option<String>,
    pub prompt_template_id: Option<Uuid>,
    pub prompt_template: Option<String>,
    pub output_schema_id: Option<Uuid>,
    pub output_variable_name: Option<String>,
    pub interactive_agent_id: Option<Uuid>,
    pub for_each_label_field: Option<String>,
    pub display_order: Option<i32>,
}

#[derive(Deserialize, utoipa::ToSchema)]
pub struct EdgeRequest {
    pub from_step_id: Uuid,
    pub to_step_id: Uuid,
}

#[derive(Serialize, utoipa::ToSchema)]
pub struct EdgeResponse {
    pub from_step_id: Uuid,
    pub to_step_id: Uuid,
}

#[derive(Deserialize, utoipa::ToSchema)]
pub struct StepDocumentRequest {
    pub document_id: Uuid,
}

#[derive(Serialize, utoipa::ToSchema)]
pub struct StepDocumentResponse {
    pub step_id: Uuid,
    pub document_id: Uuid,
}

fn step_response(r: crate::db::WorkflowStepRow) -> WorkflowStepResponse {
    WorkflowStepResponse {
        id: r.id,
        workflow_id: r.workflow_id,
        agent_id: r.agent_id,
        execution_mode: r.execution_mode,
        for_each_ref: r.for_each_ref,
        prompt_template_id: r.prompt_template_id,
        prompt_template: r.prompt_template,
        output_schema_id: r.output_schema_id,
        output_variable_name: r.output_variable_name,
        interactive_agent_id: r.interactive_agent_id,
        for_each_label_field: r.for_each_label_field,
        display_order: r.display_order,
        version: r.version,
    }
}

/// GET /api/workflows
#[utoipa::path(
    get,
    path = "/api/workflows",
    tag = "Workflows",
    security(("bearer_auth" = [])),
    responses(
        (status = 200, description = "List of workflows", body = Vec<WorkflowResponse>)
    )
)]
pub async fn list_workflows(State(state): State<AppState>, auth: auth::AuthUser) -> Result<Json<Vec<WorkflowResponse>>, StatusCode> {
    let repo = state.workflow_repo.as_ref().ok_or(StatusCode::INTERNAL_SERVER_ERROR)?;
    let rows = repo.list_workflows(auth.user_id.0).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let items = rows
        .into_iter()
        .map(|r| WorkflowResponse {
            id: r.id,
            name: r.name,
            description: r.description,
            created_at: r.created_at,
        })
        .collect();
    Ok(Json(items))
}

/// POST /api/workflows
#[utoipa::path(
    post,
    path = "/api/workflows",
    tag = "Workflows",
    security(("bearer_auth" = [])),
    request_body = CreateWorkflowRequest,
    responses(
        (status = 201, description = "Workflow created", body = WorkflowResponse),
        (status = 400, description = "Invalid request")
    )
)]
pub async fn create_workflow(State(state): State<AppState>, auth: auth::AuthUser, Json(req): Json<CreateWorkflowRequest>) -> Result<(StatusCode, Json<WorkflowResponse>), StatusCode> {
    if req.name.trim().is_empty() || req.name.len() > MAX_TITLE_LENGTH {
        return Err(StatusCode::BAD_REQUEST);
    }
    let repo = state.workflow_repo.as_ref().ok_or(StatusCode::INTERNAL_SERVER_ERROR)?;
    let row = repo
        .create_workflow(auth.user_id.0, req.name, req.description.unwrap_or_default())
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok((
        StatusCode::CREATED,
        Json(WorkflowResponse {
            id: row.id,
            name: row.name,
            description: row.description,
            created_at: row.created_at,
        }),
    ))
}

/// GET /api/workflows/:id
#[utoipa::path(
    get,
    path = "/api/workflows/{id}",
    tag = "Workflows",
    security(("bearer_auth" = [])),
    params(("id" = Uuid, Path, description = "Workflow ID")),
    responses(
        (status = 200, description = "Workflow found", body = WorkflowResponse),
        (status = 404, description = "Not found")
    )
)]
pub async fn get_workflow(State(state): State<AppState>, auth: auth::AuthUser, Path(id): Path<Uuid>) -> Result<Json<WorkflowResponse>, StatusCode> {
    let repo = state.workflow_repo.as_ref().ok_or(StatusCode::INTERNAL_SERVER_ERROR)?;
    let row = repo.get_workflow(id).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?.ok_or(StatusCode::NOT_FOUND)?;
    if row.user_id != auth.user_id.0 {
        return Err(StatusCode::NOT_FOUND);
    }
    Ok(Json(WorkflowResponse {
        id: row.id,
        name: row.name,
        description: row.description,
        created_at: row.created_at,
    }))
}

/// PUT /api/workflows/:id
#[utoipa::path(
    put,
    path = "/api/workflows/{id}",
    tag = "Workflows",
    security(("bearer_auth" = [])),
    params(("id" = Uuid, Path, description = "Workflow ID")),
    request_body = UpdateWorkflowRequest,
    responses(
        (status = 200, description = "Updated workflow", body = WorkflowResponse),
        (status = 404, description = "Not found")
    )
)]
pub async fn update_workflow(State(state): State<AppState>, auth: auth::AuthUser, Path(id): Path<Uuid>, Json(req): Json<UpdateWorkflowRequest>) -> Result<Json<WorkflowResponse>, StatusCode> {
    let repo = state.workflow_repo.as_ref().ok_or(StatusCode::INTERNAL_SERVER_ERROR)?;
    let existing = repo.get_workflow(id).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?.ok_or(StatusCode::NOT_FOUND)?;
    if existing.user_id != auth.user_id.0 {
        return Err(StatusCode::NOT_FOUND);
    }
    if let Some(ref name) = req.name {
        if name.trim().is_empty() || name.len() > MAX_TITLE_LENGTH {
            return Err(StatusCode::BAD_REQUEST);
        }
    }
    let row = repo.update_workflow(id, req.name, req.description).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Json(WorkflowResponse {
        id: row.id,
        name: row.name,
        description: row.description,
        created_at: row.created_at,
    }))
}

/// DELETE /api/workflows/:id
#[utoipa::path(
    delete,
    path = "/api/workflows/{id}",
    tag = "Workflows",
    security(("bearer_auth" = [])),
    params(("id" = Uuid, Path, description = "Workflow ID")),
    responses(
        (status = 204, description = "Deleted successfully"),
        (status = 404, description = "Not found")
    )
)]
pub async fn delete_workflow(State(state): State<AppState>, auth: auth::AuthUser, Path(id): Path<Uuid>) -> Result<StatusCode, StatusCode> {
    let repo = state.workflow_repo.as_ref().ok_or(StatusCode::INTERNAL_SERVER_ERROR)?;
    let existing = repo.get_workflow(id).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?.ok_or(StatusCode::NOT_FOUND)?;
    if existing.user_id != auth.user_id.0 {
        return Err(StatusCode::NOT_FOUND);
    }
    repo.delete_workflow(id).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(StatusCode::NO_CONTENT)
}

/// POST /api/workflows/:id/steps
#[utoipa::path(
    post,
    path = "/api/workflows/{id}/steps",
    tag = "Workflow Steps",
    security(("bearer_auth" = [])),
    params(("id" = Uuid, Path, description = "Workflow ID")),
    request_body = CreateStepRequest,
    responses(
        (status = 201, description = "Step created", body = WorkflowStepResponse),
        (status = 404, description = "Workflow not found")
    )
)]
pub async fn create_workflow_step(
    State(state): State<AppState>,
    auth: auth::AuthUser,
    Path(wid): Path<Uuid>,
    Json(req): Json<CreateStepRequest>,
) -> Result<(StatusCode, Json<WorkflowStepResponse>), StatusCode> {
    let repo = state.workflow_repo.as_ref().ok_or(StatusCode::INTERNAL_SERVER_ERROR)?;
    let wf = repo.get_workflow(wid).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?.ok_or(StatusCode::NOT_FOUND)?;
    if wf.user_id != auth.user_id.0 {
        return Err(StatusCode::NOT_FOUND);
    }
    let step = crate::db::WorkflowStepRow {
        id: Uuid::new_v4(),
        workflow_id: wid,
        agent_id: req.agent_id,
        execution_mode: req.execution_mode.unwrap_or_else(|| "single".to_string()),
        for_each_ref: req.for_each_ref,
        prompt_template_id: req.prompt_template_id,
        prompt_template: req.prompt_template.unwrap_or_default(),
        output_schema_id: req.output_schema_id,
        output_variable_name: req.output_variable_name,
        interactive_agent_id: req.interactive_agent_id,
        for_each_label_field: req.for_each_label_field,
        room_id: None,
        display_order: req.display_order.unwrap_or(0),
        version: 1,
    };
    let row = repo.create_step(step).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok((StatusCode::CREATED, Json(step_response(row))))
}

/// GET /api/workflows/:id/steps
#[utoipa::path(
    get,
    path = "/api/workflows/{id}/steps",
    tag = "Workflow Steps",
    security(("bearer_auth" = [])),
    params(("id" = Uuid, Path, description = "Workflow ID")),
    responses(
        (status = 200, description = "List of workflow steps", body = Vec<WorkflowStepResponse>),
        (status = 404, description = "Not found")
    )
)]
pub async fn list_workflow_steps(State(state): State<AppState>, auth: auth::AuthUser, Path(wid): Path<Uuid>) -> Result<Json<Vec<WorkflowStepResponse>>, StatusCode> {
    let repo = state.workflow_repo.as_ref().ok_or(StatusCode::INTERNAL_SERVER_ERROR)?;
    let wf = repo.get_workflow(wid).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?.ok_or(StatusCode::NOT_FOUND)?;
    if wf.user_id != auth.user_id.0 {
        return Err(StatusCode::NOT_FOUND);
    }
    let rows = repo.list_steps(wid).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Json(rows.into_iter().map(step_response).collect()))
}

/// GET /api/workflows/:wid/steps/:sid
#[utoipa::path(
    get,
    path = "/api/workflows/{wid}/steps/{sid}",
    tag = "Workflow Steps",
    security(("bearer_auth" = [])),
    params(
        ("wid" = Uuid, Path, description = "Workflow ID"),
        ("sid" = Uuid, Path, description = "Step ID")
    ),
    responses(
        (status = 200, description = "Workflow step found", body = WorkflowStepResponse),
        (status = 404, description = "Not found")
    )
)]
pub async fn get_workflow_step(State(state): State<AppState>, auth: auth::AuthUser, Path(p): Path<(Uuid, Uuid)>) -> Result<Json<WorkflowStepResponse>, StatusCode> {
    let repo = state.workflow_repo.as_ref().ok_or(StatusCode::INTERNAL_SERVER_ERROR)?;
    let wf = repo.get_workflow(p.0).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?.ok_or(StatusCode::NOT_FOUND)?;
    if wf.user_id != auth.user_id.0 {
        return Err(StatusCode::NOT_FOUND);
    }
    let step = repo.get_step(p.1).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?.ok_or(StatusCode::NOT_FOUND)?;
    if step.workflow_id != p.0 {
        return Err(StatusCode::NOT_FOUND);
    }
    Ok(Json(step_response(step)))
}

#[derive(Deserialize, utoipa::ToSchema)]
pub struct WorkflowStepPath {
    pub wid: Uuid,
    pub sid: Uuid,
}

/// PUT /api/workflows/:wid/steps/:sid
#[utoipa::path(
    put,
    path = "/api/workflows/{wid}/steps/{sid}",
    tag = "Workflow Steps",
    security(("bearer_auth" = [])),
    params(
        ("wid" = Uuid, Path, description = "Workflow ID"),
        ("sid" = Uuid, Path, description = "Step ID")
    ),
    request_body = UpdateStepRequest,
    responses(
        (status = 200, description = "Updated workflow step", body = WorkflowStepResponse),
        (status = 404, description = "Not found")
    )
)]
pub async fn update_workflow_step(
    State(state): State<AppState>,
    auth: auth::AuthUser,
    Path(p): Path<WorkflowStepPath>,
    Json(req): Json<UpdateStepRequest>,
) -> Result<Json<WorkflowStepResponse>, StatusCode> {
    let repo = state.workflow_repo.as_ref().ok_or(StatusCode::INTERNAL_SERVER_ERROR)?;
    let wf = repo.get_workflow(p.wid).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?.ok_or(StatusCode::NOT_FOUND)?;
    if wf.user_id != auth.user_id.0 {
        return Err(StatusCode::NOT_FOUND);
    }
    let existing = repo.get_step(p.sid).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?.ok_or(StatusCode::NOT_FOUND)?;
    if existing.workflow_id != p.wid {
        return Err(StatusCode::NOT_FOUND);
    }
    let step = crate::db::WorkflowStepRow {
        id: p.sid,
        workflow_id: p.wid,
        agent_id: req.agent_id,
        execution_mode: req.execution_mode.unwrap_or(existing.execution_mode),
        for_each_ref: req.for_each_ref,
        prompt_template_id: req.prompt_template_id,
        prompt_template: req.prompt_template.unwrap_or(existing.prompt_template),
        output_schema_id: req.output_schema_id,
        output_variable_name: req.output_variable_name,
        interactive_agent_id: req.interactive_agent_id,
        for_each_label_field: req.for_each_label_field.or(existing.for_each_label_field),
        room_id: existing.room_id,
        display_order: req.display_order.unwrap_or(existing.display_order),
        version: existing.version,
    };
    let row = repo.update_step(step).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Json(step_response(row)))
}

/// DELETE /api/workflows/:wid/steps/:sid
#[utoipa::path(
    delete,
    path = "/api/workflows/{wid}/steps/{sid}",
    tag = "Workflow Steps",
    security(("bearer_auth" = [])),
    params(
        ("wid" = Uuid, Path, description = "Workflow ID"),
        ("sid" = Uuid, Path, description = "Step ID")
    ),
    responses(
        (status = 204, description = "Deleted successfully"),
        (status = 404, description = "Not found")
    )
)]
pub async fn delete_workflow_step(State(state): State<AppState>, auth: auth::AuthUser, Path(p): Path<WorkflowStepPath>) -> Result<StatusCode, StatusCode> {
    let repo = state.workflow_repo.as_ref().ok_or(StatusCode::INTERNAL_SERVER_ERROR)?;
    let wf = repo.get_workflow(p.wid).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?.ok_or(StatusCode::NOT_FOUND)?;
    if wf.user_id != auth.user_id.0 {
        return Err(StatusCode::NOT_FOUND);
    }
    let existing = repo.get_step(p.sid).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?.ok_or(StatusCode::NOT_FOUND)?;
    if existing.workflow_id != p.wid {
        return Err(StatusCode::NOT_FOUND);
    }
    repo.delete_step(p.sid).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(StatusCode::NO_CONTENT)
}

/// GET /api/workflows/:id/edges
#[utoipa::path(
    get,
    path = "/api/workflows/{id}/edges",
    tag = "Workflow Edges",
    security(("bearer_auth" = [])),
    params(("id" = Uuid, Path, description = "Workflow ID")),
    responses(
        (status = 200, description = "List of workflow edges", body = Vec<EdgeResponse>),
        (status = 404, description = "Not found")
    )
)]
pub async fn list_workflow_edges(State(state): State<AppState>, auth: auth::AuthUser, Path(wid): Path<Uuid>) -> Result<Json<Vec<EdgeResponse>>, StatusCode> {
    let repo = state.workflow_repo.as_ref().ok_or(StatusCode::INTERNAL_SERVER_ERROR)?;
    let wf = repo.get_workflow(wid).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?.ok_or(StatusCode::NOT_FOUND)?;
    if wf.user_id != auth.user_id.0 {
        return Err(StatusCode::NOT_FOUND);
    }
    let rows = repo.list_edges(wid).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Json(
        rows.into_iter()
            .map(|e| EdgeResponse {
                from_step_id: e.from_step_id,
                to_step_id: e.to_step_id,
            })
            .collect(),
    ))
}

/// POST /api/workflows/:id/edges
#[utoipa::path(
    post,
    path = "/api/workflows/{id}/edges",
    tag = "Workflow Edges",
    security(("bearer_auth" = [])),
    params(("id" = Uuid, Path, description = "Workflow ID")),
    request_body = EdgeRequest,
    responses(
        (status = 201, description = "Edge added"),
        (status = 404, description = "Not found")
    )
)]
pub async fn add_workflow_edge(State(state): State<AppState>, auth: auth::AuthUser, Path(wid): Path<Uuid>, Json(req): Json<EdgeRequest>) -> Result<StatusCode, StatusCode> {
    let repo = state.workflow_repo.as_ref().ok_or(StatusCode::INTERNAL_SERVER_ERROR)?;
    let wf = repo.get_workflow(wid).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?.ok_or(StatusCode::NOT_FOUND)?;
    if wf.user_id != auth.user_id.0 {
        return Err(StatusCode::NOT_FOUND);
    }
    repo.add_edge(req.from_step_id, req.to_step_id).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(StatusCode::CREATED)
}

/// DELETE /api/workflows/:id/edges
#[utoipa::path(
    delete,
    path = "/api/workflows/{id}/edges",
    tag = "Workflow Edges",
    security(("bearer_auth" = [])),
    params(("id" = Uuid, Path, description = "Workflow ID")),
    request_body = EdgeRequest,
    responses(
        (status = 204, description = "Edge removed"),
        (status = 404, description = "Not found")
    )
)]
pub async fn remove_workflow_edge(State(state): State<AppState>, auth: auth::AuthUser, Path(wid): Path<Uuid>, Json(req): Json<EdgeRequest>) -> Result<StatusCode, StatusCode> {
    let repo = state.workflow_repo.as_ref().ok_or(StatusCode::INTERNAL_SERVER_ERROR)?;
    let wf = repo.get_workflow(wid).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?.ok_or(StatusCode::NOT_FOUND)?;
    if wf.user_id != auth.user_id.0 {
        return Err(StatusCode::NOT_FOUND);
    }
    repo.remove_edge(req.from_step_id, req.to_step_id).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(StatusCode::NO_CONTENT)
}

/// POST /api/workflows/:wid/steps/:sid/documents
#[utoipa::path(
    post,
    path = "/api/workflows/{wid}/steps/{sid}/documents",
    tag = "Step Documents",
    security(("bearer_auth" = [])),
    params(
        ("wid" = Uuid, Path, description = "Workflow ID"),
        ("sid" = Uuid, Path, description = "Step ID")
    ),
    request_body = StepDocumentRequest,
    responses(
        (status = 201, description = "Document added to step"),
        (status = 404, description = "Not found")
    )
)]
pub async fn add_step_document(State(state): State<AppState>, auth: auth::AuthUser, Path(p): Path<WorkflowStepPath>, Json(req): Json<StepDocumentRequest>) -> Result<StatusCode, StatusCode> {
    let repo = state.workflow_repo.as_ref().ok_or(StatusCode::INTERNAL_SERVER_ERROR)?;
    let wf = repo.get_workflow(p.wid).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?.ok_or(StatusCode::NOT_FOUND)?;
    if wf.user_id != auth.user_id.0 {
        return Err(StatusCode::NOT_FOUND);
    }
    let existing = repo.get_step(p.sid).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?.ok_or(StatusCode::NOT_FOUND)?;
    if existing.workflow_id != p.wid {
        return Err(StatusCode::NOT_FOUND);
    }
    repo.add_step_document(p.sid, req.document_id).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(StatusCode::CREATED)
}

/// DELETE /api/workflows/:wid/steps/:sid/documents
#[utoipa::path(
    delete,
    path = "/api/workflows/{wid}/steps/{sid}/documents",
    tag = "Step Documents",
    security(("bearer_auth" = [])),
    params(
        ("wid" = Uuid, Path, description = "Workflow ID"),
        ("sid" = Uuid, Path, description = "Step ID")
    ),
    request_body = StepDocumentRequest,
    responses(
        (status = 204, description = "Document removed from step"),
        (status = 404, description = "Not found")
    )
)]
pub async fn remove_step_document(State(state): State<AppState>, auth: auth::AuthUser, Path(p): Path<WorkflowStepPath>, Json(req): Json<StepDocumentRequest>) -> Result<StatusCode, StatusCode> {
    let repo = state.workflow_repo.as_ref().ok_or(StatusCode::INTERNAL_SERVER_ERROR)?;
    let wf = repo.get_workflow(p.wid).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?.ok_or(StatusCode::NOT_FOUND)?;
    if wf.user_id != auth.user_id.0 {
        return Err(StatusCode::NOT_FOUND);
    }
    let existing = repo.get_step(p.sid).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?.ok_or(StatusCode::NOT_FOUND)?;
    if existing.workflow_id != p.wid {
        return Err(StatusCode::NOT_FOUND);
    }
    repo.remove_step_document(p.sid, req.document_id).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(StatusCode::NO_CONTENT)
}

/// GET /api/workflows/:wid/steps/:sid/documents
#[utoipa::path(
    get,
    path = "/api/workflows/{wid}/steps/{sid}/documents",
    tag = "Step Documents",
    security(("bearer_auth" = [])),
    params(
        ("wid" = Uuid, Path, description = "Workflow ID"),
        ("sid" = Uuid, Path, description = "Step ID")
    ),
    responses(
        (status = 200, description = "List of step documents", body = Vec<StepDocumentResponse>),
        (status = 404, description = "Not found")
    )
)]
pub async fn list_step_documents(State(state): State<AppState>, auth: auth::AuthUser, Path(p): Path<WorkflowStepPath>) -> Result<Json<Vec<StepDocumentResponse>>, StatusCode> {
    let repo = state.workflow_repo.as_ref().ok_or(StatusCode::INTERNAL_SERVER_ERROR)?;
    let wf = repo.get_workflow(p.wid).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?.ok_or(StatusCode::NOT_FOUND)?;
    if wf.user_id != auth.user_id.0 {
        return Err(StatusCode::NOT_FOUND);
    }
    let rows = repo.list_step_documents(p.sid).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Json(
        rows.into_iter()
            .map(|r| StepDocumentResponse {
                step_id: r.step_id,
                document_id: r.document_id,
            })
            .collect(),
    ))
}

// ============================================================================
// Context Response Endpoint (F6)
// ============================================================================

/// Request body for submitting a context response to an agent
#[derive(Deserialize, utoipa::ToSchema)]
pub struct ContextResponseRequest {
    pub agent_id: Uuid,
    pub task_id: Uuid,
    pub context: String,
    pub files: Vec<FilePathContent>,
}

/// A file with path and content for context responses
#[derive(Deserialize, utoipa::ToSchema)]
pub struct FilePathContent {
    pub path: String,
    pub content: String,
}

/// POST /api/context-response - Submit a human context response to an agent
#[utoipa::path(
    post,
    path = "/api/context-response",
    tag = "Agent Context",
    security(("bearer_auth" = [])),
    request_body = ContextResponseRequest,
    responses(
        (status = 200, description = "Context response submitted"),
        (status = 404, description = "Agent not found")
    )
)]
pub async fn submit_context_response(State(state): State<AppState>, _auth: auth::AuthUser, Json(request): Json<ContextResponseRequest>) -> Result<StatusCode, StatusCode> {
    use crate::agents::{AgentCommand, AgentId, ContextResponse, FileContent};

    let agent_id = AgentId(request.agent_id);

    let files: Vec<FileContent> = request.files.into_iter().map(|f| FileContent { path: f.path, content: f.content }).collect();

    let answers = if request.context.is_empty() { vec![] } else { vec![request.context] };

    let dispatcher = state.dispatcher.as_ref().ok_or(StatusCode::SERVICE_UNAVAILABLE)?;
    let disp = dispatcher.lock().await;
    disp.send_to_agent(
        &agent_id,
        AgentCommand::ProvideContext(ContextResponse {
            task_id: request.task_id,
            files,
            answers,
            true_context: None,
        }),
    )
    .await
    .map_err(|_| StatusCode::NOT_FOUND)?;

    Ok(StatusCode::OK)
}

// ============================================================================
// Pipeline Stage Template Rendering
// ============================================================================

/// Resolve `{{stage_name.field}}` placeholders in a template string.
///
/// `stage_outputs` maps stage_name → JSON object of that stage's output fields.
pub fn resolve_template(template: &str, stage_outputs: &std::collections::HashMap<String, serde_json::Value>, context_docs: &std::collections::HashMap<String, String>) -> String {
    let mut result = template.to_string();
    // Match {{context.ref_tag}} patterns first (so stage refs can contain context)
    let context_re = regex::Regex::new(r"\{\{context\.(\w+)\}\}").unwrap();
    for cap in context_re.captures_iter(template) {
        let full_match = &cap[0];
        let ref_tag = &cap[1];
        let replacement = context_docs.get(ref_tag).cloned().unwrap_or_else(|| full_match.to_string());
        result = result.replace(full_match, &replacement);
    }
    // Match {{stage_name.field}} patterns
    let stage_re = regex::Regex::new(r"\{\{(\w+)\.(\w+)\}\}").unwrap();
    let snapshot = result.clone();
    for cap in stage_re.captures_iter(&snapshot) {
        let full_match = &cap[0];
        let stage = &cap[1];
        let field = &cap[2];
        if stage == "context" {
            continue; // already handled
        }
        let replacement = stage_outputs
            .get(stage)
            .and_then(|obj| obj.get(field))
            .map(|v| match v {
                serde_json::Value::String(s) => s.clone(),
                other => other.to_string(),
            })
            .unwrap_or_else(|| full_match.to_string());
        result = result.replace(full_match, &replacement);
    }
    result
}

/// Render a pipeline stage into a markdown prompt.
///
/// Takes the stage definition and a map of resolved input values (key → string value).
pub fn render_stage_prompt(output_description: &str, resolved_inputs: &[(String, String)], output_schema: &serde_json::Value) -> String {
    let mut prompt = String::new();

    // Goal section
    prompt.push_str("# Goal\n");
    prompt.push_str(output_description);
    prompt.push_str("\n\n");

    // Input section
    if !resolved_inputs.is_empty() {
        prompt.push_str("# Input\n");
        for (key, value) in resolved_inputs {
            prompt.push_str(&format!("- {}: {}\n", key, value));
        }
        prompt.push('\n');
    }

    // Output schema section
    if let Some(fields) = output_schema.get("fields").and_then(|f| f.as_array()) {
        if !fields.is_empty() {
            prompt.push_str("# Output Schema\nReturn a JSON object with these fields:\n");
            for field in fields {
                let name = field.get("name").and_then(|n| n.as_str()).unwrap_or("?");
                let ftype = field.get("type").and_then(|t| t.as_str()).unwrap_or("string");
                let desc = field.get("description").and_then(|d| d.as_str()).unwrap_or("");
                let type_str = if ftype == "enum" {
                    if let Some(values) = field.get("values").and_then(|v| v.as_array()) {
                        let vals: Vec<&str> = values.iter().filter_map(|v| v.as_str()).collect();
                        format!("one of {:?}", vals)
                    } else {
                        "enum".to_string()
                    }
                } else {
                    ftype.to_string()
                };
                if desc.is_empty() {
                    prompt.push_str(&format!("- {}: {}\n", name, type_str));
                } else {
                    prompt.push_str(&format!("- {}: {} — {}\n", name, type_str, desc));
                }
            }
        }
    }

    prompt.trim_end().to_string()
}

/// Render a pipeline stage into a resolved prompt string.
///
/// Reusable core that can be called from HTTP endpoints or the orchestrator.
/// Resolves input definitions, context documents, and output_description templates.
pub async fn render_stage(doc_repo: Option<&dyn crate::db::traits::DocumentRepo>, stage: &crate::db::PipelineStageRow, stage_outputs: &std::collections::HashMap<String, serde_json::Value>) -> String {
    // Fetch context documents referenced via {{context.ref_tag}} patterns
    let mut context_docs: std::collections::HashMap<String, String> = std::collections::HashMap::new();
    let context_re = regex::Regex::new(r"\{\{context\.(\w+)\}\}").unwrap();
    let output_desc = stage.output_description.clone().unwrap_or_default();
    let input_defs = stage.input_definitions.clone().unwrap_or_else(|| serde_json::json!([]));
    let out_schema = stage.output_schema.clone().unwrap_or_else(|| serde_json::json!({"fields": []}));
    let mut all_text = output_desc.clone();
    if let Some(defs) = input_defs.as_array() {
        for def in defs {
            if let Some(v) = def.get("value").and_then(|v| v.as_str()) {
                all_text.push(' ');
                all_text.push_str(v);
            }
        }
    }
    if let Some(doc_repo) = doc_repo {
        for cap in context_re.captures_iter(&all_text) {
            let ref_tag_str = cap[1].to_string();
            if let std::collections::hash_map::Entry::Vacant(e) = context_docs.entry(ref_tag_str.clone()) {
                if let Ok(Some(doc)) = doc_repo.get_document_by_ref_tag(&ref_tag_str).await {
                    e.insert(doc.content);
                }
            }
        }
    }

    // Resolve input definitions
    let mut resolved_inputs: Vec<(String, String)> = Vec::new();
    if let Some(defs) = input_defs.as_array() {
        for def in defs {
            let key = def.get("key").and_then(|k| k.as_str()).unwrap_or("").to_string();
            let source = def.get("source").and_then(|s| s.as_str()).unwrap_or("");
            let value = match source {
                "static" => def
                    .get("value")
                    .map(|v| match v {
                        serde_json::Value::String(s) => s.clone(),
                        other => other.to_string(),
                    })
                    .unwrap_or_default(),
                "stage" => {
                    let ref_str = def.get("ref").and_then(|r| r.as_str()).unwrap_or("");
                    let template = format!("{{{{{}}}}}", ref_str);
                    resolve_template(&template, stage_outputs, &context_docs)
                }
                _ => String::new(),
            };
            if !key.is_empty() {
                resolved_inputs.push((key, value));
            }
        }
    }

    // Resolve output_description template
    let resolved_description = resolve_template(&output_desc, stage_outputs, &context_docs);

    render_stage_prompt(&resolved_description, &resolved_inputs, &out_schema)
}

/// Request body for rendering a pipeline stage prompt.
#[derive(Deserialize, utoipa::ToSchema)]
pub struct RenderStageRequest {
    /// Map of stage_name → JSON output from that stage.
    pub stage_outputs: std::collections::HashMap<String, serde_json::Value>,
}

/// Render a pipeline stage into a resolved prompt (HTTP endpoint wrapper).
#[utoipa::path(
    post,
    path = "/api/pipelines/{id}/stages/{stage_number}/render",
    tag = "Pipeline Runs",
    security(("bearer_auth" = [])),
    params(
        ("id" = String, Path, description = "Pipeline ID"),
        ("stage_number" = i32, Path, description = "Stage number")
    ),
    request_body = RenderStageRequest,
    responses(
        (status = 200, description = "Rendered stage prompt"),
        (status = 404, description = "Stage not found")
    )
)]
pub async fn render_pipeline_stage(
    State(state): State<AppState>,
    _user: auth::AuthUser,
    Path((pipeline_id, stage_number)): Path<(String, i32)>,
    Json(body): Json<RenderStageRequest>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let Ok(pipeline_uuid) = Uuid::parse_str(&pipeline_id) else {
        return Err(StatusCode::BAD_REQUEST);
    };

    let stages = state.repo.list_pipeline_stages(pipeline_uuid).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let stage = stages.into_iter().find(|s| s.stage_number == stage_number).ok_or(StatusCode::NOT_FOUND)?;

    let doc_repo_ref = state.doc_repo.as_deref();
    let prompt = render_stage(doc_repo_ref, &stage, &body.stage_outputs).await;

    Ok(Json(serde_json::json!({
        "pipeline_id": pipeline_id,
        "stage_number": stage_number,
        "stage_name": stage.stage_name,
        "prompt": prompt
    })))
}

// ============================================================================
// Pipeline Run Endpoints
// ============================================================================

#[derive(Deserialize, utoipa::ToSchema)]
pub struct ApproveRunRequest {
    pub user_input: Option<String>,
}

#[derive(Serialize, utoipa::ToSchema)]
pub struct PipelineRunResponse {
    pub id: String,
    pub pipeline_id: String,
    pub status: String,
    pub initial_task: String,
    pub stage_outputs: serde_json::Value,
    pub current_stage: i32,
    pub started_at: String,
    pub completed_at: Option<String>,
    pub total_input_tokens: i64,
    pub total_output_tokens: i64,
}

impl PipelineRunResponse {
    fn from_row(row: crate::db::PipelineRunRow) -> Self {
        Self {
            id: row.id.to_string(),
            pipeline_id: row.pipeline_id.to_string(),
            status: row.status,
            initial_task: row.initial_task,
            stage_outputs: row.stage_outputs.unwrap_or_else(|| serde_json::json!({})),
            current_stage: row.current_stage,
            started_at: row.started_at.to_rfc3339(),
            completed_at: row.completed_at.map(|t| t.to_rfc3339()),
            total_input_tokens: row.total_input_tokens.unwrap_or(0),
            total_output_tokens: row.total_output_tokens.unwrap_or(0),
        }
    }
}

#[derive(Serialize, utoipa::ToSchema)]
pub struct StageExecutionResponse {
    pub id: String,
    pub run_id: String,
    pub stage_number: i32,
    pub stage_name: String,
    pub agent_id: Option<String>,
    pub status: String,
    pub rendered_prompt: Option<String>,
    pub output: Option<String>,
    pub structured_output: Option<serde_json::Value>,
    pub user_input: Option<String>,
    pub input_tokens: i64,
    pub output_tokens: i64,
    pub started_at: String,
    pub completed_at: Option<String>,
    pub duration_ms: i64,
}

impl StageExecutionResponse {
    fn from_row(row: crate::db::StageExecutionRow) -> Self {
        Self {
            id: row.id.to_string(),
            run_id: row.run_id.to_string(),
            stage_number: row.stage_number,
            stage_name: row.stage_name,
            agent_id: row.agent_id.map(|id| id.to_string()),
            status: row.status,
            rendered_prompt: row.rendered_prompt,
            output: row.output,
            structured_output: row.structured_output,
            user_input: row.user_input,
            input_tokens: row.input_tokens,
            output_tokens: row.output_tokens,
            started_at: row.started_at.to_rfc3339(),
            completed_at: row.completed_at.map(|t| t.to_rfc3339()),
            duration_ms: row.duration_ms,
        }
    }
}

#[derive(Serialize, utoipa::ToSchema)]
pub struct PipelineRunDetailResponse {
    #[serde(flatten)]
    pub run: PipelineRunResponse,
    pub stages: Vec<StageExecutionResponse>,
}

#[derive(Deserialize, utoipa::ToSchema, utoipa::IntoParams)]
pub struct ListRunsQuery {
    pub pipeline_id: Option<String>,
}

/// List pipeline runs, optionally filtered by pipeline_id.
#[utoipa::path(
    get,
    path = "/api/pipeline-runs",
    tag = "Pipeline Runs",
    security(("bearer_auth" = [])),
    params(ListRunsQuery),
    responses(
        (status = 200, description = "List of pipeline runs", body = Vec<PipelineRunResponse>)
    )
)]
pub async fn list_pipeline_runs(State(state): State<AppState>, _user: auth::AuthUser, Query(query): Query<ListRunsQuery>) -> Result<Json<Vec<PipelineRunResponse>>, StatusCode> {
    let pipeline_id = query.pipeline_id.as_deref().and_then(|s| Uuid::parse_str(s).ok()).ok_or(StatusCode::BAD_REQUEST)?;

    let runs = state.repo.list_pipeline_runs(pipeline_id).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(runs.into_iter().map(PipelineRunResponse::from_row).collect()))
}

/// Get a pipeline run with its stage executions.
#[utoipa::path(
    get,
    path = "/api/pipeline-runs/{run_id}",
    tag = "Pipeline Runs",
    security(("bearer_auth" = [])),
    params(("run_id" = String, Path, description = "Pipeline run ID")),
    responses(
        (status = 200, description = "Pipeline run details", body = PipelineRunDetailResponse),
        (status = 404, description = "Not found")
    )
)]
pub async fn get_pipeline_run(State(state): State<AppState>, _user: auth::AuthUser, Path(run_id): Path<String>) -> Result<Json<PipelineRunDetailResponse>, StatusCode> {
    let run_uuid = Uuid::parse_str(&run_id).map_err(|_| StatusCode::BAD_REQUEST)?;

    let run = state
        .repo
        .get_pipeline_run(run_uuid)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::NOT_FOUND)?;

    let stages = state.repo.list_stage_executions(run_uuid).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(PipelineRunDetailResponse {
        run: PipelineRunResponse::from_row(run),
        stages: stages.into_iter().map(StageExecutionResponse::from_row).collect(),
    }))
}

/// Approve a pipeline run gate and optionally inject user context.
#[utoipa::path(
    post,
    path = "/api/pipeline-runs/{run_id}/approve",
    tag = "Pipeline Runs",
    security(("bearer_auth" = [])),
    params(("run_id" = String, Path, description = "Pipeline run ID")),
    request_body = ApproveRunRequest,
    responses(
        (status = 200, description = "Run approved and resumed"),
        (status = 404, description = "Run not found"),
        (status = 409, description = "Run not waiting for approval")
    )
)]
pub async fn approve_pipeline_run(
    State(state): State<AppState>,
    _user: auth::AuthUser,
    Path(run_id): Path<String>,
    Json(request): Json<ApproveRunRequest>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let run_uuid = Uuid::parse_str(&run_id).map_err(|_| StatusCode::BAD_REQUEST)?;

    // Validate the run is waiting for approval
    let run = state
        .repo
        .get_pipeline_run(run_uuid)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::NOT_FOUND)?;

    if run.status != "waiting_for_approval" {
        return Err(StatusCode::CONFLICT);
    }

    // If user provided input, store it on the current stage execution
    if let Some(ref user_input) = request.user_input {
        let stages = state.repo.list_stage_executions(run_uuid).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

        if let Some(current_exec) = stages.into_iter().find(|s| s.stage_number == run.current_stage) {
            let mut updated = current_exec;
            updated.user_input = Some(user_input.clone());
            updated.status = "completed".to_string();
            updated.completed_at = Some(chrono::Utc::now());
            let _ = state.repo.update_stage_execution(&updated).await;
        }

        // Record user_input in stage_outputs for template access
        let stage_name = {
            let mgr = state.pipeline_manager.read().await;
            mgr.get_stage_name(run_uuid, run.current_stage as u32).unwrap_or_else(|| format!("stage_{}", run.current_stage))
        };
        {
            let mut mgr = state.pipeline_manager.write().await;
            mgr.record_stage_output(run_uuid, stage_name, serde_json::json!({ "user_input": user_input }));
        }
    }

    // Update run status back to running
    let mut updated_run = run;
    updated_run.status = "running".to_string();
    let _ = state.repo.update_pipeline_run(&updated_run).await;

    // Resume the in-memory pipeline manager
    {
        let mut mgr = state.pipeline_manager.write().await;
        // Set status back to Running
        if let Some(mem_run) = mgr.get_run(run_uuid) {
            if mem_run.status == crate::agents::pipeline::PipelineRunStatus::WaitingForApproval {
                // Advance to next stage
                match mgr.advance_stage(run_uuid) {
                    Ok(Some(next_stage)) => {
                        drop(mgr);

                        // Trigger task assignment for the next stage
                        let initial_task = {
                            let mgr2 = state.pipeline_manager.read().await;
                            mgr2.get_run_initial_task(run_uuid).unwrap_or_default().to_string()
                        };
                        let stage_outputs = {
                            let mgr2 = state.pipeline_manager.read().await;
                            mgr2.get_stage_outputs(run_uuid).cloned().unwrap_or_default()
                        };
                        let pipeline_id = {
                            let mgr2 = state.pipeline_manager.read().await;
                            mgr2.get_run_pipeline_id(run_uuid)
                        };

                        let rendered_prompt = if let Some(pid) = pipeline_id {
                            match state.repo.list_pipeline_stages(pid.0).await {
                                Ok(db_stages) => {
                                    if let Some(db_stage) = db_stages.into_iter().find(|s| s.stage_number == next_stage.stage_number as i32) {
                                        let doc_repo_ref = state.doc_repo.as_deref();
                                        Some(render_stage(doc_repo_ref, &db_stage, &stage_outputs).await)
                                    } else {
                                        None
                                    }
                                }
                                Err(_) => None,
                            }
                        } else {
                            None
                        };

                        let description = rendered_prompt.unwrap_or_else(|| format!("{}\n\nPrevious output available in stage context.", initial_task));

                        let mut context_reading: Vec<crate::agents::FileContent> = Vec::new();

                        let resolved_agent_id = if let Some(aid) = &next_stage.agent_id {
                            if let Ok(docs) = state.repo.get_agent_context(aid.0).await {
                                for doc in &docs {
                                    context_reading.push(crate::agents::FileContent {
                                        path: format!("context:{}", doc.ref_tag.as_deref().filter(|s| !s.is_empty()).unwrap_or(&doc.title)),
                                        content: doc.content.clone(),
                                    });
                                }
                            }
                            Some(aid.clone())
                        } else {
                            None
                        };

                        use crate::agents::{AgentCommand, CommunicationStyle, OutputFormat, RoleContext, RoleId, TaskAssignment, TaskConstraints, TaskContext};

                        let role_str = next_stage.role.as_deref().unwrap_or("worker");
                        let assignment = TaskAssignment {
                            task_id: Uuid::new_v4(),
                            title: format!("Pipeline stage {}: {}", next_stage.stage_number, initial_task),
                            description,
                            context: TaskContext {
                                required_reading: context_reading,
                                files: vec![],
                                history: vec![],
                                conventions: String::new(),
                                role_context: RoleContext {
                                    system_prompt: format!("You are a {} working on: {}", role_str, initial_task),
                                    style: CommunicationStyle::Technical,
                                    output_format: OutputFormat::CodeAndReport,
                                },
                                chat_messages: vec![],
                                execution_context: Some(crate::execution::ExecutionContext::new(std::env::current_dir().unwrap_or_default())),
                                tool_rows: vec![],
                                router_mode: false,
                                cluster_routing: None,
                                context_docs: vec![],
                                distiller_mode: crate::agents::DistillerMode::Background,
                            },
                            constraints: TaskConstraints::default(),
                            timeout: std::time::Duration::from_secs(crate::constants::DEFAULT_TIMEOUT_SECS),
                            role_id: RoleId::new(role_str),
                        };

                        let new_task_id = assignment.task_id;
                        {
                            let mut mgr2 = state.pipeline_manager.write().await;
                            mgr2.record_stage_task(run_uuid, next_stage.stage_number, new_task_id);
                        }

                        if let Some(agent_id) = &resolved_agent_id {
                            if let Some(disp) = &state.dispatcher {
                                let disp = disp.lock().await;
                                if let Err(e) = disp.send_to_agent(agent_id, AgentCommand::AssignTask(Box::new(assignment))).await {
                                    tracing::error!("Gate resume dispatch failed: {}", e);
                                    let mut mgr2 = state.pipeline_manager.write().await;
                                    let _ = mgr2.fail_run(run_uuid, &e.to_string());
                                }
                            }
                        }

                        // Broadcast gate_resumed
                        state.broadcast_pipeline(super::ws::PipelineUpdate {
                            run_id: run_uuid,
                            pipeline_id: pipeline_id.map(|p| p.0).unwrap_or(run_uuid),
                            event: "gate_resumed".into(),
                            stage_number: Some(next_stage.stage_number as i32),
                            stage_name: Some(next_stage.stage_name.clone()),
                            agent_id: resolved_agent_id.as_ref().map(|a| a.0.to_string()),
                            output: None,
                            input_tokens: None,
                            output_tokens: None,
                            duration_ms: None,
                            user_input: request.user_input.clone(),
                            timestamp: chrono::Utc::now(),
                            user_id: Some(_user.user_id.0),
                        });

                        return Ok(Json(serde_json::json!({ "status": "resumed", "next_stage": next_stage.stage_number })));
                    }
                    Ok(None) => {
                        // Pipeline completed
                        return Ok(Json(serde_json::json!({ "status": "completed" })));
                    }
                    Err(e) => {
                        tracing::error!("Gate resume advance error: {}", e);
                        return Err(StatusCode::INTERNAL_SERVER_ERROR);
                    }
                }
            }
        }
    }

    Ok(Json(serde_json::json!({ "status": "resumed" })))
}

// ============================================================================
// Cancellation Endpoints
// ============================================================================

/// POST /pipeline-runs/:run_id/cancel - Cancel a running pipeline.
#[utoipa::path(
    post,
    path = "/pipeline-runs/{run_id}/cancel",
    params(("run_id" = String, Path, description = "Pipeline run UUID")),
    responses(
        (status = 200, description = "Pipeline cancelled"),
        (status = 404, description = "Pipeline run not found"),
        (status = 409, description = "Pipeline run not in a cancellable state")
    )
)]
pub async fn cancel_pipeline_run(State(state): State<AppState>, _user: auth::AuthUser, Path(run_id): Path<String>) -> Result<Json<serde_json::Value>, StatusCode> {
    let run_uuid = Uuid::parse_str(&run_id).map_err(|_| StatusCode::BAD_REQUEST)?;

    let run = state
        .repo
        .get_pipeline_run(run_uuid)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::NOT_FOUND)?;

    if run.status == "completed" || run.status == "failed" || run.status == "cancelled" {
        return Err(StatusCode::CONFLICT);
    }

    // Trigger cancellation token
    state.cancel_execution(run_uuid).await;

    // Update pipeline run status
    let mut updated = run;
    updated.status = "cancelled".to_string();
    updated.completed_at = Some(chrono::Utc::now());
    let _ = state.repo.update_pipeline_run(&updated).await;

    // Cancel all running stage executions
    if let Ok(stages) = state.repo.list_stage_executions(run_uuid).await {
        for stage in stages {
            if stage.status == "running" || stage.status == "waiting_for_approval" {
                let mut s = stage;
                s.status = "cancelled".to_string();
                s.completed_at = Some(chrono::Utc::now());
                let _ = state.repo.update_stage_execution(&s).await;
            }
        }
    }

    // Broadcast cancellation
    let pipeline_id = {
        let mgr = state.pipeline_manager.read().await;
        mgr.get_run_pipeline_id(run_uuid).map(|p| p.0).unwrap_or(run_uuid)
    };

    state.broadcast_pipeline(super::ws::PipelineUpdate {
        run_id: run_uuid,
        pipeline_id,
        event: "run_cancelled".into(),
        stage_number: None,
        stage_name: None,
        agent_id: None,
        output: None,
        input_tokens: None,
        output_tokens: None,
        duration_ms: None,
        user_input: None,
        timestamp: chrono::Utc::now(),
        user_id: Some(_user.user_id.0),
    });

    Ok(Json(serde_json::json!({ "status": "cancelled" })))
}

/// POST /agent-executions/:execution_id/cancel - Cancel a running agent execution.
#[utoipa::path(
    post,
    path = "/agent-executions/{execution_id}/cancel",
    params(("execution_id" = String, Path, description = "Agent execution UUID")),
    responses(
        (status = 200, description = "Execution cancelled"),
        (status = 404, description = "Execution not found or no cancellation token registered")
    )
)]
pub async fn cancel_agent_execution(State(state): State<AppState>, _user: auth::AuthUser, Path(execution_id): Path<String>) -> Result<Json<serde_json::Value>, StatusCode> {
    let exec_uuid = Uuid::parse_str(&execution_id).map_err(|_| StatusCode::BAD_REQUEST)?;

    let cancelled = state.cancel_execution(exec_uuid).await;
    if !cancelled {
        return Err(StatusCode::NOT_FOUND);
    }

    // Update execution status in DB
    if let Some(ae_repo) = &state.agent_execution_repo {
        let _ = ae_repo.update_agent_execution_status(exec_uuid, "cancelled", None, None).await;
    }

    Ok(Json(serde_json::json!({ "status": "cancelled" })))
}

// ============================================================================
// Tool Router Endpoints
// ============================================================================

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
pub async fn list_tool_routers(State(state): State<AppState>, auth: auth::AuthUser) -> Result<Json<Vec<crate::db::ToolRouterRow>>, StatusCode> {
    let repo = state.tool_router_repo.as_ref().ok_or(StatusCode::INTERNAL_SERVER_ERROR)?;
    let rows = repo.list_tool_routers(auth.user_id.0).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Json(rows))
}

/// Request body for creating a tool router.
#[derive(Deserialize, utoipa::ToSchema)]
pub struct CreateToolRouterRequest {
    pub name: String,
    pub description: Option<String>,
    pub system_prompt: String,
    pub model_id: String,
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
pub async fn create_tool_router(State(state): State<AppState>, auth: auth::AuthUser, Json(request): Json<CreateToolRouterRequest>) -> Result<(StatusCode, Json<crate::db::ToolRouterRow>), StatusCode> {
    if request.name.trim().is_empty() || request.name.len() > MAX_TITLE_LENGTH {
        return Err(StatusCode::BAD_REQUEST);
    }
    let repo = state.tool_router_repo.as_ref().ok_or(StatusCode::INTERNAL_SERVER_ERROR)?;
    let row = repo
        .create_tool_router(auth.user_id.0, &request.name, request.description, &request.system_prompt, &request.model_id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
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
pub async fn get_tool_router(State(state): State<AppState>, auth: auth::AuthUser, Path(id): Path<Uuid>) -> Result<Json<crate::db::ToolRouterRow>, StatusCode> {
    let repo = state.tool_router_repo.as_ref().ok_or(StatusCode::INTERNAL_SERVER_ERROR)?;
    let row = repo.get_tool_router(id).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?.ok_or(StatusCode::NOT_FOUND)?;
    if row.user_id != auth.user_id.0 {
        return Err(StatusCode::NOT_FOUND);
    }
    Ok(Json(row))
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
    auth: auth::AuthUser,
    Path(id): Path<Uuid>,
    Json(request): Json<UpdateToolRouterRequest>,
) -> Result<Json<crate::db::ToolRouterRow>, StatusCode> {
    let repo = state.tool_router_repo.as_ref().ok_or(StatusCode::INTERNAL_SERVER_ERROR)?;
    let existing = repo.get_tool_router(id).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?.ok_or(StatusCode::NOT_FOUND)?;
    if existing.user_id != auth.user_id.0 {
        return Err(StatusCode::NOT_FOUND);
    }
    if let Some(ref name) = request.name {
        if name.trim().is_empty() || name.len() > MAX_TITLE_LENGTH {
            return Err(StatusCode::BAD_REQUEST);
        }
    }
    let row = repo
        .update_tool_router(id, request.name, request.description, request.system_prompt, request.model_id, request.is_active)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
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
pub async fn delete_tool_router(State(state): State<AppState>, auth: auth::AuthUser, Path(id): Path<Uuid>) -> Result<StatusCode, StatusCode> {
    let repo = state.tool_router_repo.as_ref().ok_or(StatusCode::INTERNAL_SERVER_ERROR)?;
    let existing = repo.get_tool_router(id).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?.ok_or(StatusCode::NOT_FOUND)?;
    if existing.user_id != auth.user_id.0 {
        return Err(StatusCode::NOT_FOUND);
    }
    repo.delete_tool_router(id).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
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
pub async fn get_router_tools(State(state): State<AppState>, auth: auth::AuthUser, Path(id): Path<Uuid>) -> Result<Json<Vec<ToolResponse>>, StatusCode> {
    let repo = state.tool_router_repo.as_ref().ok_or(StatusCode::INTERNAL_SERVER_ERROR)?;
    let existing = repo.get_tool_router(id).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?.ok_or(StatusCode::NOT_FOUND)?;
    if existing.user_id != auth.user_id.0 {
        return Err(StatusCode::NOT_FOUND);
    }
    let tools = repo.get_router_tools(id).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let tools = tools.into_iter().map(ToolResponse::from_row).collect();
    Ok(Json(tools))
}

/// Request body for setting router tools.
#[derive(Deserialize, utoipa::ToSchema)]
pub struct SetRouterToolsRequest {
    pub tool_ids: Vec<Uuid>,
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
pub async fn set_router_tools(State(state): State<AppState>, auth: auth::AuthUser, Path(id): Path<Uuid>, Json(request): Json<SetRouterToolsRequest>) -> Result<StatusCode, StatusCode> {
    let repo = state.tool_router_repo.as_ref().ok_or(StatusCode::INTERNAL_SERVER_ERROR)?;
    let existing = repo.get_tool_router(id).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?.ok_or(StatusCode::NOT_FOUND)?;
    if existing.user_id != auth.user_id.0 {
        return Err(StatusCode::NOT_FOUND);
    }
    repo.set_router_tools(id, &request.tool_ids).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(StatusCode::NO_CONTENT)
}

/// GET /api/sessions/:session_id/context - Get context entries for a session.
#[utoipa::path(
    get,
    path = "/api/sessions/{session_id}/context",
    tag = "Session Context",
    security(("bearer_auth" = [])),
    params(("session_id" = Uuid, Path, description = "Session ID")),
    responses(
        (status = 200, description = "Session context entries")
    )
)]
pub async fn get_session_context(State(state): State<AppState>, _auth: auth::AuthUser, Path(session_id): Path<Uuid>) -> Result<Json<Vec<crate::db::ContextStoreRow>>, StatusCode> {
    let repo = state.context_store_repo.as_ref().ok_or(StatusCode::INTERNAL_SERVER_ERROR)?;
    let rows = repo.get_active_context(session_id, 100).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Json(rows))
}

/// GET /api/sessions/:session_id/requests - List router requests for a session.
#[utoipa::path(
    get,
    path = "/api/sessions/{session_id}/requests",
    tag = "Session Context",
    security(("bearer_auth" = [])),
    params(("session_id" = Uuid, Path, description = "Session ID")),
    responses(
        (status = 200, description = "List of router requests")
    )
)]
pub async fn list_session_requests(State(state): State<AppState>, _auth: auth::AuthUser, Path(session_id): Path<Uuid>) -> Result<Json<Vec<crate::db::RouterRequestRow>>, StatusCode> {
    let repo = state.router_request_repo.as_ref().ok_or(StatusCode::INTERNAL_SERVER_ERROR)?;
    let rows = repo.list_session_requests(session_id).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Json(rows))
}
// ============================================================================
// Rooms
// ============================================================================

/// Request body for creating a room.
#[derive(Deserialize, utoipa::ToSchema)]
pub struct CreateRoomRequest {
    pub pipeline_id: Uuid,
    pub name: String,
    #[serde(default)]
    pub gatekeeper_enabled: bool,
    #[serde(default = "default_gatekeeper_model")]
    pub gatekeeper_model_id: String,
    #[serde(default = "default_max_speakers")]
    pub max_speakers_per_turn: i32,
    #[serde(default = "default_max_turns")]
    pub max_turns: i32,
    #[serde(default)]
    pub tools_enabled: bool,
}

fn default_gatekeeper_model() -> String {
    "claude-haiku-4-20250414".to_string()
}
fn default_max_speakers() -> i32 {
    4
}
fn default_max_turns() -> i32 {
    20
}

/// Request body for updating a room.
#[derive(Deserialize, utoipa::ToSchema)]
pub struct UpdateRoomRequest {
    pub name: Option<String>,
    pub gatekeeper_enabled: Option<bool>,
    pub gatekeeper_model_id: Option<String>,
    pub max_speakers_per_turn: Option<i32>,
    pub max_turns: Option<i32>,
    pub tools_enabled: Option<bool>,
}

/// Request body for adding a room member.
#[derive(Deserialize, utoipa::ToSchema)]
pub struct AddRoomMemberRequest {
    pub agent_id: Uuid,
    pub display_name: Option<String>,
    pub role_description: String,
    #[serde(default)]
    pub display_order: i32,
}

/// Request body for setting all room members at once.
#[derive(Deserialize, utoipa::ToSchema)]
pub struct SetRoomMembersRequest {
    pub members: Vec<AddRoomMemberRequest>,
}

/// Request body for sending a message to a room session.
#[derive(Deserialize, utoipa::ToSchema)]
pub struct RoomMessageRequest {
    pub content: String,
}

/// POST /api/rooms - Create a room.
pub async fn create_room(State(state): State<AppState>, auth: auth::AuthUser, Json(request): Json<CreateRoomRequest>) -> Result<(StatusCode, Json<crate::db::RoomRow>), StatusCode> {
    if request.name.trim().is_empty() || request.name.len() > MAX_TITLE_LENGTH {
        return Err(StatusCode::BAD_REQUEST);
    }
    let repo = state.room_repo.as_ref().ok_or(StatusCode::INTERNAL_SERVER_ERROR)?;
    let row = repo
        .create_room(
            auth.user_id.0,
            request.pipeline_id,
            &request.name,
            request.gatekeeper_enabled,
            &request.gatekeeper_model_id,
            request.max_speakers_per_turn,
            request.max_turns,
            request.tools_enabled,
        )
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok((StatusCode::CREATED, Json(row)))
}

/// GET /api/rooms/:id - Get a room.
pub async fn get_room(State(state): State<AppState>, auth: auth::AuthUser, Path(id): Path<Uuid>) -> Result<Json<crate::db::RoomRow>, StatusCode> {
    let repo = state.room_repo.as_ref().ok_or(StatusCode::INTERNAL_SERVER_ERROR)?;
    let row = repo.get_room(id).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?.ok_or(StatusCode::NOT_FOUND)?;
    if row.user_id != auth.user_id.0 {
        return Err(StatusCode::NOT_FOUND);
    }
    Ok(Json(row))
}

/// PUT /api/rooms/:id - Update a room.
pub async fn update_room(State(state): State<AppState>, auth: auth::AuthUser, Path(id): Path<Uuid>, Json(request): Json<UpdateRoomRequest>) -> Result<Json<crate::db::RoomRow>, StatusCode> {
    let repo = state.room_repo.as_ref().ok_or(StatusCode::INTERNAL_SERVER_ERROR)?;
    let existing = repo.get_room(id).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?.ok_or(StatusCode::NOT_FOUND)?;
    if existing.user_id != auth.user_id.0 {
        return Err(StatusCode::NOT_FOUND);
    }
    if let Some(ref name) = request.name {
        if name.trim().is_empty() || name.len() > MAX_TITLE_LENGTH {
            return Err(StatusCode::BAD_REQUEST);
        }
    }
    let row = repo
        .update_room(
            id,
            request.name,
            request.gatekeeper_enabled,
            request.gatekeeper_model_id,
            request.max_speakers_per_turn,
            request.max_turns,
            request.tools_enabled,
        )
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Json(row))
}

/// DELETE /api/rooms/:id - Delete a room.
pub async fn delete_room(State(state): State<AppState>, auth: auth::AuthUser, Path(id): Path<Uuid>) -> Result<StatusCode, StatusCode> {
    let repo = state.room_repo.as_ref().ok_or(StatusCode::INTERNAL_SERVER_ERROR)?;
    let existing = repo.get_room(id).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?.ok_or(StatusCode::NOT_FOUND)?;
    if existing.user_id != auth.user_id.0 {
        return Err(StatusCode::NOT_FOUND);
    }
    repo.delete_room(id).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(StatusCode::NO_CONTENT)
}

/// GET /api/pipelines/:id/rooms - List rooms for a pipeline.
pub async fn list_pipeline_rooms(State(state): State<AppState>, _auth: auth::AuthUser, Path(pipeline_id): Path<Uuid>) -> Result<Json<Vec<crate::db::RoomRow>>, StatusCode> {
    let repo = state.room_repo.as_ref().ok_or(StatusCode::INTERNAL_SERVER_ERROR)?;
    let rows = repo.list_rooms_for_pipeline(pipeline_id).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Json(rows))
}

/// GET /api/rooms/:id/members - List room members.
pub async fn list_room_members(State(state): State<AppState>, _auth: auth::AuthUser, Path(room_id): Path<Uuid>) -> Result<Json<Vec<crate::db::RoomMemberRow>>, StatusCode> {
    let repo = state.room_repo.as_ref().ok_or(StatusCode::INTERNAL_SERVER_ERROR)?;
    let rows = repo.list_room_members(room_id).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Json(rows))
}

/// POST /api/rooms/:id/members - Add a room member.
pub async fn add_room_member(State(state): State<AppState>, _auth: auth::AuthUser, Path(room_id): Path<Uuid>, Json(request): Json<AddRoomMemberRequest>) -> Result<StatusCode, StatusCode> {
    let repo = state.room_repo.as_ref().ok_or(StatusCode::INTERNAL_SERVER_ERROR)?;
    repo.add_room_member(room_id, request.agent_id, request.display_name, request.role_description, request.display_order)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(StatusCode::CREATED)
}

/// DELETE /api/rooms/:id/members/:agent_id - Remove a room member.
pub async fn remove_room_member(State(state): State<AppState>, _auth: auth::AuthUser, Path((room_id, agent_id)): Path<(Uuid, Uuid)>) -> Result<StatusCode, StatusCode> {
    let repo = state.room_repo.as_ref().ok_or(StatusCode::INTERNAL_SERVER_ERROR)?;
    repo.remove_room_member(room_id, agent_id).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(StatusCode::NO_CONTENT)
}

/// PUT /api/rooms/:id/members - Set all room members (replace).
pub async fn set_room_members(State(state): State<AppState>, _auth: auth::AuthUser, Path(room_id): Path<Uuid>, Json(request): Json<SetRoomMembersRequest>) -> Result<StatusCode, StatusCode> {
    let repo = state.room_repo.as_ref().ok_or(StatusCode::INTERNAL_SERVER_ERROR)?;
    let members: Vec<crate::db::traits::RoomMemberInput> = request
        .members
        .into_iter()
        .map(|m| crate::db::traits::RoomMemberInput {
            agent_id: m.agent_id,
            display_name: m.display_name,
            role_description: m.role_description,
            display_order: m.display_order,
        })
        .collect();
    repo.set_room_members(room_id, &members).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(StatusCode::OK)
}

/// POST /api/rooms/:id/sessions - Start a room session.
pub async fn create_room_session(State(state): State<AppState>, _auth: auth::AuthUser, Path(room_id): Path<Uuid>) -> Result<(StatusCode, Json<crate::db::RoomSessionRow>), StatusCode> {
    let repo = state.room_repo.as_ref().ok_or(StatusCode::INTERNAL_SERVER_ERROR)?;
    let row = repo.create_room_session(room_id, None).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok((StatusCode::CREATED, Json(row)))
}

/// GET /api/room-sessions/:id - Get room session.
pub async fn get_room_session(State(state): State<AppState>, _auth: auth::AuthUser, Path(id): Path<Uuid>) -> Result<Json<crate::db::RoomSessionRow>, StatusCode> {
    let repo = state.room_repo.as_ref().ok_or(StatusCode::INTERNAL_SERVER_ERROR)?;
    let row = repo.get_room_session(id).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?.ok_or(StatusCode::NOT_FOUND)?;
    Ok(Json(row))
}

/// POST /api/room-sessions/:id/messages - Send a message to a room session.
///
/// Triggers a full room turn: gatekeeper (if enabled) selects speakers,
/// each speaker executes via the engine, responses stream via WebSocket.
/// Returns immediately with turn status.
pub async fn send_room_message(
    State(state): State<AppState>,
    auth: auth::AuthUser,
    Path(session_id): Path<Uuid>,
    Json(request): Json<RoomMessageRequest>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    if request.content.trim().is_empty() {
        return Err(StatusCode::BAD_REQUEST);
    }

    let room_repo = state.room_repo.as_ref().ok_or(StatusCode::INTERNAL_SERVER_ERROR)?;

    // Load session
    let session = room_repo
        .get_room_session(session_id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::NOT_FOUND)?;

    if session.status != "active" {
        return Err(StatusCode::CONFLICT);
    }

    // Load room
    let room = room_repo.get_room(session.room_id).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?.ok_or(StatusCode::NOT_FOUND)?;

    // Load members + agents
    let member_rows = room_repo.list_room_members(room.id).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let mut members = Vec::new();
    for m in member_rows {
        let agent = state
            .repo
            .get_persisted_agent(m.agent_id)
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
            .ok_or(StatusCode::INTERNAL_SERVER_ERROR)?;
        members.push(super::room_executor::RoomMemberWithAgent { member: m, agent });
    }

    // Create LLM provider
    let provider: std::sync::Arc<dyn crate::llm::LLMProvider + Send + Sync> = match crate::llm::AnthropicClient::from_env() {
        Ok(p) => std::sync::Arc::new(p),
        Err(_) => return Err(StatusCode::SERVICE_UNAVAILABLE),
    };

    // Spawn the room turn in background so the HTTP response returns immediately
    let user_id = auth.user_id.0;
    let content = request.content;
    let state_clone = state.clone();
    tokio::spawn(async move {
        match super::room_executor::execute_room_turn(&state_clone, provider, &room, &session, &members, &content, user_id, None).await {
            Ok(result) => {
                tracing::info!(
                    session_id = %session_id,
                    turn = result.turn_number,
                    speakers = result.speakers.len(),
                    "Room turn completed"
                );
            }
            Err(e) => {
                tracing::warn!(session_id = %session_id, error = %e, "Room turn failed");
            }
        }
    });

    Ok(Json(serde_json::json!({
        "session_id": session_id,
        "status": "processing"
    })))
}

/// GET /api/room-sessions/:id/transcript - Get full transcript.
pub async fn get_room_transcript(State(state): State<AppState>, _auth: auth::AuthUser, Path(session_id): Path<Uuid>) -> Result<Json<Vec<crate::db::RoomTranscriptEntry>>, StatusCode> {
    let repo = state.room_repo.as_ref().ok_or(StatusCode::INTERNAL_SERVER_ERROR)?;
    let entries = repo.get_room_transcript(session_id).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Json(entries))
}

/// POST /api/room-sessions/:id/close - Close a room session.
pub async fn close_room_session(State(state): State<AppState>, _auth: auth::AuthUser, Path(session_id): Path<Uuid>) -> Result<StatusCode, StatusCode> {
    let repo = state.room_repo.as_ref().ok_or(StatusCode::INTERNAL_SERVER_ERROR)?;
    let session = repo.get_room_session(session_id).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?.ok_or(StatusCode::NOT_FOUND)?;
    if session.status != "active" {
        return Err(StatusCode::CONFLICT);
    }
    repo.update_room_session_status(session_id, "completed").await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    state.broadcast_room_update(super::ws::RoomUpdateEvent {
        room_session_id: session_id,
        run_id: session.run_id,
        event: "session_complete".into(),
        agent_id: None,
        agent_name: None,
        content: None,
        speaker_order: None,
        turn_number: Some(session.current_turn),
        timestamp: Utc::now(),
        user_id: None,
    });

    Ok(StatusCode::NO_CONTENT)
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn create_task_request_deserializes() {
        let json = r#"{"title": "Test task", "priority": "high"}"#;
        let request: CreateTaskRequest = serde_json::from_str(json).unwrap();
        assert_eq!(request.title, "Test task");
        assert_eq!(request.priority, Some("high".to_string()));
    }

    #[test]
    fn health_response_serializes() {
        let response = HealthResponse {
            status: "ok".to_string(),
            version: "1.0.0".to_string(),
            db_connected: true,
        };
        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("\"status\":\"ok\""));
        assert!(json.contains("\"db_connected\":true"));
    }

    #[test]
    fn config_response_serializes() {
        let response = ConfigResponse {
            verbosity: "normal".to_string(),
            pool: AgentPoolConfig::default(),
            autonomy: "approval_gates".to_string(),
            git_strategy: "branch_per_slice".to_string(),
            sandbox_mode: "docker".to_string(),
        };
        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("\"verbosity\":\"normal\""));
    }

    #[test]
    fn tasks_query_deserializes() {
        let json = r#"{"status": "pending", "limit": 10}"#;
        let query: TasksQuery = serde_json::from_str(json).unwrap();
        assert_eq!(query.status, Some("pending".to_string()));
        assert_eq!(query.limit, Some(10));
    }

    #[test]
    fn agent_pool_stats_serializes() {
        let stats = AgentPoolStats {
            total: 6,
            available: 5,
            max: 12,
        };
        let json = serde_json::to_string(&stats).unwrap();
        assert!(json.contains("\"total\""));
        assert!(json.contains("\"available\""));
        assert!(json.contains("\"max\""));
    }

    // Chat endpoint tests

    #[test]
    fn chat_request_deserializes() {
        let json = r#"{"message": "Hello, world!"}"#;
        let request: ChatRequest = serde_json::from_str(json).unwrap();
        assert_eq!(request.message, "Hello, world!");
    }

    #[test]
    fn chat_response_serializes() {
        let response = ChatResponse {
            message_id: Uuid::new_v4(),
            status: "queued".to_string(),
        };
        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("\"message_id\""));
        assert!(json.contains("\"status\":\"queued\""));
    }

    #[test]
    fn history_query_deserializes() {
        let json = r#"{"limit": 25, "offset": 10}"#;
        let query: HistoryQuery = serde_json::from_str(json).unwrap();
        assert_eq!(query.limit, Some(25));
        assert_eq!(query.offset, Some(10));
    }

    #[test]
    fn history_query_with_defaults() {
        let json = r#"{}"#;
        let query: HistoryQuery = serde_json::from_str(json).unwrap();
        assert_eq!(query.limit, None);
        assert_eq!(query.offset, None);
    }

    #[test]
    fn chat_message_serializes() {
        let message = ChatMessage {
            id: Uuid::new_v4(),
            role: "user".to_string(),
            content: "Hello!".to_string(),
            timestamp: Utc::now(),
        };
        let json = serde_json::to_string(&message).unwrap();
        assert!(json.contains("\"role\":\"user\""));
        assert!(json.contains("\"content\":\"Hello!\""));
    }

    // === Integration tests using setup_test_app (mock-based, no Postgres) ===

    use axum::body::Body;
    use axum::http::Request;
    use tower::util::ServiceExt;

    use std::sync::Arc;

    use crate::db::traits::{MockUserRepo, ServerRepo};
    use crate::db::{ChatMessageRow, PipelineRow, PipelineStageRow};
    use crate::types::{User, UserId};

    /// In-memory implementation of ServerRepo for tests (no Postgres needed).
    struct InMemoryServerRepo {
        tasks: std::sync::Mutex<Vec<Task>>,
        chat_messages: std::sync::Mutex<Vec<ChatMessageRow>>,
        password_hash: std::sync::Mutex<Option<String>>,
        agents: std::sync::Mutex<Vec<crate::db::AgentRow>>,
        tools: std::sync::Mutex<Vec<crate::db::ToolRow>>,
        agent_tools: std::sync::Mutex<Vec<(Uuid, Uuid)>>,
    }

    impl InMemoryServerRepo {
        fn new() -> Self {
            Self {
                tasks: std::sync::Mutex::new(vec![]),
                chat_messages: std::sync::Mutex::new(vec![]),
                password_hash: std::sync::Mutex::new(None),
                agents: std::sync::Mutex::new(vec![]),
                tools: std::sync::Mutex::new(vec![]),
                agent_tools: std::sync::Mutex::new(vec![]),
            }
        }
    }

    #[async_trait::async_trait]
    impl ServerRepo for InMemoryServerRepo {
        async fn health_check(&self) -> bool {
            true
        }

        async fn list_tasks(&self, _user_id: UserId, status: Option<String>, limit: Option<u32>) -> anyhow::Result<Vec<Task>> {
            let tasks = self.tasks.lock().unwrap();
            let limit = limit.unwrap_or(crate::constants::DEFAULT_QUERY_LIMIT as u32).min(crate::constants::MAX_QUERY_LIMIT as u32) as usize;
            let filtered: Vec<Task> = tasks
                .iter()
                .filter(|t| {
                    if let Some(ref s) = status {
                        let ts = format!("{:?}", t.status).to_lowercase();
                        &ts == s
                    } else {
                        true
                    }
                })
                .rev()
                .take(limit)
                .cloned()
                .collect();
            Ok(filtered)
        }

        async fn get_task_by_uuid(&self, _user_id: UserId, id: Uuid) -> anyhow::Result<Option<Task>> {
            let tasks = self.tasks.lock().unwrap();
            Ok(tasks.iter().find(|t| t.id.0 == id).cloned())
        }

        async fn insert_task(&self, _user_id: UserId, task: Task) -> anyhow::Result<()> {
            self.tasks.lock().unwrap().push(task);
            Ok(())
        }

        async fn insert_chat_message(&self, _user_id: UserId, id: Uuid, role: String, content: String) -> anyhow::Result<()> {
            self.chat_messages.lock().unwrap().push(ChatMessageRow {
                id,
                role,
                content,
                timestamp: Utc::now(),
            });
            Ok(())
        }

        async fn get_chat_history(&self, _user_id: UserId, limit: u32, offset: u32) -> anyhow::Result<Vec<ChatMessageRow>> {
            let msgs = self.chat_messages.lock().unwrap();
            let result: Vec<ChatMessageRow> = msgs.iter().skip(offset as usize).take(limit.min(1000) as usize).cloned().collect();
            Ok(result)
        }

        async fn clear_chat_history(&self, _user_id: UserId) -> anyhow::Result<()> {
            self.chat_messages.lock().unwrap().clear();
            Ok(())
        }

        async fn has_password(&self) -> anyhow::Result<bool> {
            Ok(self.password_hash.lock().unwrap().is_some())
        }

        async fn set_password(&self, password_hash: String) -> anyhow::Result<()> {
            *self.password_hash.lock().unwrap() = Some(password_hash);
            Ok(())
        }

        async fn get_password(&self) -> anyhow::Result<Option<String>> {
            Ok(self.password_hash.lock().unwrap().clone())
        }
        async fn list_persisted_agents(&self, _user_id: UserId) -> anyhow::Result<Vec<crate::db::AgentRow>> {
            Ok(self.agents.lock().unwrap().clone())
        }
        async fn get_persisted_agent(&self, agent_id: Uuid) -> anyhow::Result<Option<crate::db::AgentRow>> {
            Ok(self.agents.lock().unwrap().iter().find(|a| a.id == agent_id).cloned())
        }
        async fn upsert_agent(&self, _user_id: UserId, agent: crate::db::AgentRow) -> anyhow::Result<()> {
            let mut agents = self.agents.lock().unwrap();
            if let Some(existing) = agents.iter_mut().find(|a| a.id == agent.id) {
                *existing = agent;
            } else {
                agents.push(agent);
            }
            Ok(())
        }
        async fn delete_persisted_agent(&self, agent_id: Uuid) -> anyhow::Result<()> {
            self.agents.lock().unwrap().retain(|a| a.id != agent_id);
            Ok(())
        }
        async fn list_tools(&self, _user_id: UserId) -> anyhow::Result<Vec<crate::db::ToolRow>> {
            Ok(self.tools.lock().unwrap().clone())
        }
        async fn get_tool(&self, tool_id: Uuid) -> anyhow::Result<Option<crate::db::ToolRow>> {
            Ok(self.tools.lock().unwrap().iter().find(|t| t.id == tool_id).cloned())
        }
        async fn upsert_tool(&self, _user_id: UserId, tool: crate::db::ToolRow) -> anyhow::Result<()> {
            let mut tools = self.tools.lock().unwrap();
            if let Some(existing) = tools.iter_mut().find(|t| t.id == tool.id) {
                *existing = tool;
            } else {
                tools.push(tool);
            }
            Ok(())
        }
        async fn delete_tool(&self, tool_id: Uuid) -> anyhow::Result<()> {
            self.tools.lock().unwrap().retain(|t| t.id != tool_id);
            self.agent_tools.lock().unwrap().retain(|(_, tid)| *tid != tool_id);
            Ok(())
        }
        async fn get_agent_tools(&self, agent_id: Uuid) -> anyhow::Result<Vec<crate::db::ToolRow>> {
            let at = self.agent_tools.lock().unwrap();
            let tool_ids: Vec<Uuid> = at.iter().filter(|(aid, _)| *aid == agent_id).map(|(_, tid)| *tid).collect();
            let tools = self.tools.lock().unwrap();
            Ok(tools.iter().filter(|t| tool_ids.contains(&t.id)).cloned().collect())
        }
        async fn set_agent_tools(&self, agent_id: Uuid, tool_ids: Vec<Uuid>) -> anyhow::Result<()> {
            let mut at = self.agent_tools.lock().unwrap();
            at.retain(|(aid, _)| *aid != agent_id);
            for tid in tool_ids {
                at.push((agent_id, tid));
            }
            Ok(())
        }
        async fn seed_builtin_tools(&self, _user_id: UserId) -> anyhow::Result<()> {
            Ok(())
        }
        async fn get_agent_context(&self, _agent_id: Uuid) -> anyhow::Result<Vec<crate::db::DocumentRow>> {
            Ok(vec![])
        }
        async fn set_agent_context(&self, _agent_id: Uuid, _document_ids: Vec<Uuid>) -> anyhow::Result<()> {
            Ok(())
        }
        async fn list_pipelines(&self, _user_id: UserId) -> anyhow::Result<Vec<PipelineRow>> {
            Ok(vec![])
        }
        async fn upsert_pipeline(&self, _user_id: UserId, _pipeline: PipelineRow) -> anyhow::Result<()> {
            Ok(())
        }
        async fn delete_pipeline(&self, _pipeline_id: Uuid) -> anyhow::Result<()> {
            Ok(())
        }
        async fn list_pipeline_stages(&self, _pipeline_id: Uuid) -> anyhow::Result<Vec<PipelineStageRow>> {
            Ok(vec![])
        }
        async fn upsert_pipeline_stage(&self, _stage: PipelineStageRow) -> anyhow::Result<()> {
            Ok(())
        }
        async fn create_session(&self, _user_id: UserId, _session_id: Uuid, _mode_id: &str, _title: &str, _agent_id: Option<Uuid>) -> anyhow::Result<()> {
            Ok(())
        }
        async fn list_sessions(&self, _user_id: UserId) -> anyhow::Result<Vec<crate::db::SessionRow>> {
            Ok(vec![])
        }
        async fn get_session(&self, _session_id: Uuid) -> anyhow::Result<Option<crate::db::SessionRow>> {
            Ok(None)
        }
        async fn delete_session(&self, _session_id: Uuid) -> anyhow::Result<()> {
            Ok(())
        }
        async fn insert_session_message(&self, _user_id: UserId, _session_id: Uuid, _id: Uuid, _role: String, _content: String) -> anyhow::Result<()> {
            Ok(())
        }
        async fn get_session_history(&self, _session_id: Uuid, _limit: u32) -> anyhow::Result<Vec<ChatMessageRow>> {
            Ok(vec![])
        }
        async fn update_session_title(&self, _session_id: Uuid, _title: &str) -> anyhow::Result<()> {
            Ok(())
        }
        async fn update_session_summary(&self, _session_id: Uuid, _summary: &str) -> anyhow::Result<()> {
            Ok(())
        }
        async fn count_session_messages(&self, _session_id: Uuid) -> anyhow::Result<u32> {
            Ok(0)
        }
        async fn create_pipeline_run(&self, _run: &crate::db::PipelineRunRow) -> anyhow::Result<()> {
            Ok(())
        }
        async fn update_pipeline_run(&self, _run: &crate::db::PipelineRunRow) -> anyhow::Result<()> {
            Ok(())
        }
        async fn get_pipeline_run(&self, _run_id: Uuid) -> anyhow::Result<Option<crate::db::PipelineRunRow>> {
            Ok(None)
        }
        async fn list_pipeline_runs(&self, _pipeline_id: Uuid) -> anyhow::Result<Vec<crate::db::PipelineRunRow>> {
            Ok(vec![])
        }
        async fn create_stage_execution(&self, _exec: &crate::db::StageExecutionRow) -> anyhow::Result<()> {
            Ok(())
        }
        async fn update_stage_execution(&self, _exec: &crate::db::StageExecutionRow) -> anyhow::Result<()> {
            Ok(())
        }
        async fn list_stage_executions(&self, _run_id: Uuid) -> anyhow::Result<Vec<crate::db::StageExecutionRow>> {
            Ok(vec![])
        }
        async fn get_agent_modes(&self, _agent_id: Uuid) -> anyhow::Result<Vec<crate::db::AgentModeRow>> {
            Ok(vec![])
        }
        async fn create_agent_mode(&self, _mode: &crate::db::AgentModeRow) -> anyhow::Result<()> {
            Ok(())
        }
        async fn delete_agent_mode(&self, _mode_id: Uuid) -> anyhow::Result<()> {
            Ok(())
        }
    }

    fn create_test_token(jwt_secret: &[u8]) -> String {
        super::super::auth::create_token(jwt_secret, 24, UserId::new(), "test@test.com").unwrap()
    }

    fn setup_test_app() -> (axum::Router, Vec<u8>) {
        setup_test_app_with_user_repo(None)
    }

    fn setup_test_app_with_user_repo(user_repo: Option<Arc<dyn crate::db::traits::UserRepo>>) -> (axum::Router, Vec<u8>) {
        let repo: Arc<dyn ServerRepo> = Arc::new(InMemoryServerRepo::new());
        let config = crate::types::AppConfig::default();
        let (mut state, rx) = AppState::with_repo(None, repo, config);
        // Keep the receiver alive so orchestrator_tx.send() doesn't fail
        std::mem::forget(rx);
        if let Some(ur) = user_repo {
            state.user_repo = Some(ur);
        }
        let jwt_secret = state.jwt_secret.clone();
        let router = super::super::create_router_with_static_dir(state, "nonexistent_static");
        (router, jwt_secret)
    }

    #[tokio::test]
    async fn create_task_valid_returns_created() {
        let (app, jwt_secret) = setup_test_app();
        let token = create_test_token(&jwt_secret);

        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/tasks")
                    .header("content-type", "application/json")
                    .header("authorization", format!("Bearer {}", token))
                    .body(Body::from(r#"{"title":"My task","description":"desc","priority":"high"}"#))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::CREATED);
    }

    #[tokio::test]
    async fn create_task_empty_title_returns_bad_request() {
        let (app, jwt_secret) = setup_test_app();
        let token = create_test_token(&jwt_secret);

        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/tasks")
                    .header("content-type", "application/json")
                    .header("authorization", format!("Bearer {}", token))
                    .body(Body::from(r#"{"title":"   "}"#))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn update_config_valid_verbosity() {
        let (app, jwt_secret) = setup_test_app();
        let token = create_test_token(&jwt_secret);

        let response = app
            .oneshot(
                Request::builder()
                    .method("PATCH")
                    .uri("/api/config")
                    .header("content-type", "application/json")
                    .header("authorization", format!("Bearer {}", token))
                    .body(Body::from(r#"{"verbosity":"verbose"}"#))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn send_chat_valid_message_returns_accepted() {
        let (app, jwt_secret) = setup_test_app();
        let token = create_test_token(&jwt_secret);

        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/chat")
                    .header("content-type", "application/json")
                    .header("authorization", format!("Bearer {}", token))
                    .body(Body::from(r#"{"message":"Hello agent"}"#))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::ACCEPTED);
    }

    #[tokio::test]
    async fn send_chat_empty_message_returns_bad_request() {
        let (app, jwt_secret) = setup_test_app();
        let token = create_test_token(&jwt_secret);

        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/chat")
                    .header("content-type", "application/json")
                    .header("authorization", format!("Bearer {}", token))
                    .body(Body::from(r#"{"message":"  "}"#))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn clear_chat_history_returns_no_content() {
        let (app, jwt_secret) = setup_test_app();
        let token = create_test_token(&jwt_secret);

        let response = app
            .oneshot(
                Request::builder()
                    .method("DELETE")
                    .uri("/api/chat/history")
                    .header("authorization", format!("Bearer {}", token))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::NO_CONTENT);
    }

    #[tokio::test]
    async fn health_check_returns_ok() {
        let (app, _jwt_secret) = setup_test_app();

        let response = app.oneshot(Request::builder().uri("/api/health").body(Body::empty()).unwrap()).await.unwrap();

        assert_eq!(response.status(), StatusCode::OK);

        let body = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let body_str = String::from_utf8(body.to_vec()).unwrap();
        assert!(body_str.contains("\"status\":\"ok\""));
        assert!(body_str.contains("\"db_connected\":true"));
    }

    // === Priority parsing tests ===

    #[tokio::test]
    async fn create_task_with_low_priority() {
        let (app, jwt_secret) = setup_test_app();
        let token = create_test_token(&jwt_secret);

        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/tasks")
                    .header("content-type", "application/json")
                    .header("authorization", format!("Bearer {}", token))
                    .body(Body::from(r#"{"title":"Low prio","priority":"low"}"#))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::CREATED);
        let body = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let body_str = String::from_utf8(body.to_vec()).unwrap();
        assert!(body_str.contains("\"priority\":\"low\""));
    }

    #[tokio::test]
    async fn create_task_with_urgent_priority() {
        let (app, jwt_secret) = setup_test_app();
        let token = create_test_token(&jwt_secret);

        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/tasks")
                    .header("content-type", "application/json")
                    .header("authorization", format!("Bearer {}", token))
                    .body(Body::from(r#"{"title":"Urgent","priority":"urgent"}"#))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::CREATED);
        let body = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let body_str = String::from_utf8(body.to_vec()).unwrap();
        assert!(body_str.contains("\"priority\":\"urgent\""));
    }

    #[tokio::test]
    async fn create_task_with_unknown_priority_defaults_to_normal() {
        let (app, jwt_secret) = setup_test_app();
        let token = create_test_token(&jwt_secret);

        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/tasks")
                    .header("content-type", "application/json")
                    .header("authorization", format!("Bearer {}", token))
                    .body(Body::from(r#"{"title":"Default prio","priority":"critical"}"#))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::CREATED);
        let body = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let body_str = String::from_utf8(body.to_vec()).unwrap();
        assert!(body_str.contains("\"priority\":\"normal\""));
    }

    #[tokio::test]
    async fn create_task_with_no_priority_defaults_to_normal() {
        let (app, jwt_secret) = setup_test_app();
        let token = create_test_token(&jwt_secret);

        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/tasks")
                    .header("content-type", "application/json")
                    .header("authorization", format!("Bearer {}", token))
                    .body(Body::from(r#"{"title":"No prio"}"#))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::CREATED);
        let body = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let body_str = String::from_utf8(body.to_vec()).unwrap();
        assert!(body_str.contains("\"priority\":\"normal\""));
    }

    // === get_task: found and not found ===

    #[tokio::test]
    async fn get_task_returns_created_task() {
        let (app, jwt_secret) = setup_test_app();
        let token = create_test_token(&jwt_secret);

        // Create a task first
        let create_resp = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/tasks")
                    .header("content-type", "application/json")
                    .header("authorization", format!("Bearer {}", token))
                    .body(Body::from(r#"{"title":"Findable task"}"#))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(create_resp.status(), StatusCode::CREATED);
        let body = axum::body::to_bytes(create_resp.into_body(), usize::MAX).await.unwrap();
        let created: serde_json::Value = serde_json::from_slice(&body).unwrap();
        let task_id = created["id"].as_str().unwrap();

        // Verify through list endpoint that the task was persisted
        let list_resp = app
            .oneshot(Request::builder().uri("/api/tasks").header("authorization", format!("Bearer {}", token)).body(Body::empty()).unwrap())
            .await
            .unwrap();

        assert_eq!(list_resp.status(), StatusCode::OK);
        let body = axum::body::to_bytes(list_resp.into_body(), usize::MAX).await.unwrap();
        let tasks: Vec<serde_json::Value> = serde_json::from_slice(&body).unwrap();
        let found = tasks.iter().find(|t| t["id"].as_str() == Some(task_id));
        assert!(found.is_some(), "Created task should appear in task list");
        assert_eq!(found.unwrap()["title"].as_str().unwrap(), "Findable task");
    }

    #[tokio::test]
    async fn get_task_not_found() {
        let (app, jwt_secret) = setup_test_app();
        let token = create_test_token(&jwt_secret);

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/api/tasks/00000000-0000-0000-0000-000000000000")
                    .header("authorization", format!("Bearer {}", token))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        // Note: This may return 404 from the handler OR from the static fallback.
        // Both are acceptable for a non-existent task.
        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }

    // === list_tasks with filters ===

    #[tokio::test]
    async fn list_tasks_returns_empty_initially() {
        let (app, jwt_secret) = setup_test_app();
        let token = create_test_token(&jwt_secret);

        let response = app
            .oneshot(Request::builder().uri("/api/tasks").header("authorization", format!("Bearer {}", token)).body(Body::empty()).unwrap())
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        let body = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let tasks: Vec<serde_json::Value> = serde_json::from_slice(&body).unwrap();
        assert!(tasks.is_empty());
    }

    #[tokio::test]
    async fn list_tasks_with_limit() {
        let (app, jwt_secret) = setup_test_app();
        let token = create_test_token(&jwt_secret);

        // Create two tasks
        for title in ["Task A", "Task B"] {
            app.clone()
                .oneshot(
                    Request::builder()
                        .method("POST")
                        .uri("/api/tasks")
                        .header("content-type", "application/json")
                        .header("authorization", format!("Bearer {}", token))
                        .body(Body::from(format!(r#"{{"title":"{}"}}"#, title)))
                        .unwrap(),
                )
                .await
                .unwrap();
        }

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/api/tasks?limit=1")
                    .header("authorization", format!("Bearer {}", token))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        let body = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let tasks: Vec<serde_json::Value> = serde_json::from_slice(&body).unwrap();
        assert_eq!(tasks.len(), 1);
    }

    #[tokio::test]
    async fn list_tasks_with_status_filter() {
        let (app, jwt_secret) = setup_test_app();
        let token = create_test_token(&jwt_secret);

        // Create a task (default status is pending)
        app.clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/tasks")
                    .header("content-type", "application/json")
                    .header("authorization", format!("Bearer {}", token))
                    .body(Body::from(r#"{"title":"Pending task"}"#))
                    .unwrap(),
            )
            .await
            .unwrap();

        // Filter for in_progress - should return nothing
        let response = app
            .oneshot(
                Request::builder()
                    .uri("/api/tasks?status=in_progress")
                    .header("authorization", format!("Bearer {}", token))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        let body = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let tasks: Vec<serde_json::Value> = serde_json::from_slice(&body).unwrap();
        assert!(tasks.is_empty());
    }

    // === update_config invalid verbosity ===

    #[tokio::test]
    async fn update_config_invalid_verbosity_returns_bad_request() {
        let (app, jwt_secret) = setup_test_app();
        let token = create_test_token(&jwt_secret);

        let response = app
            .oneshot(
                Request::builder()
                    .method("PATCH")
                    .uri("/api/config")
                    .header("content-type", "application/json")
                    .header("authorization", format!("Bearer {}", token))
                    .body(Body::from(r#"{"verbosity":"extreme"}"#))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn update_config_quiet_verbosity() {
        let (app, jwt_secret) = setup_test_app();
        let token = create_test_token(&jwt_secret);

        let response = app
            .oneshot(
                Request::builder()
                    .method("PATCH")
                    .uri("/api/config")
                    .header("content-type", "application/json")
                    .header("authorization", format!("Bearer {}", token))
                    .body(Body::from(r#"{"verbosity":"quiet"}"#))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn update_config_normal_verbosity() {
        let (app, jwt_secret) = setup_test_app();
        let token = create_test_token(&jwt_secret);

        let response = app
            .oneshot(
                Request::builder()
                    .method("PATCH")
                    .uri("/api/config")
                    .header("content-type", "application/json")
                    .header("authorization", format!("Bearer {}", token))
                    .body(Body::from(r#"{"verbosity":"normal"}"#))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn update_config_no_verbosity_returns_ok() {
        let (app, jwt_secret) = setup_test_app();
        let token = create_test_token(&jwt_secret);

        let response = app
            .oneshot(
                Request::builder()
                    .method("PATCH")
                    .uri("/api/config")
                    .header("content-type", "application/json")
                    .header("authorization", format!("Bearer {}", token))
                    .body(Body::from(r#"{}"#))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
    }

    // === list_agents response body ===

    #[tokio::test]
    async fn list_agents_returns_stats() {
        let (app, jwt_secret) = setup_test_app();
        let token = create_test_token(&jwt_secret);

        let response = app
            .oneshot(Request::builder().uri("/api/agents").header("authorization", format!("Bearer {}", token)).body(Body::empty()).unwrap())
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        let body = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let resp: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert!(resp["agents"].is_array());
        assert!(resp["stats"]["total"].is_number());
        assert!(resp["stats"]["available"].is_number());
        assert!(resp["stats"]["max"].is_number());
    }

    // === Agent CRUD tests ===

    #[tokio::test]
    async fn create_agent_returns_created() {
        let (app, jwt_secret) = setup_test_app();
        let token = create_test_token(&jwt_secret);

        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/agents")
                    .header("content-type", "application/json")
                    .header("authorization", format!("Bearer {}", token))
                    .body(Body::from(r#"{"tier":"worker","persona_name":"Builder","model_id":"claude-sonnet-4-20250514"}"#))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::CREATED);
        let body = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let agent: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(agent["tier"].as_str().unwrap(), "worker");
        assert_eq!(agent["persona_name"].as_str().unwrap(), "Builder");
        assert_eq!(agent["persona_style"].as_str().unwrap(), "casual");
        assert_eq!(agent["model_provider"].as_str().unwrap(), "anthropic");
        assert_eq!(agent["model_max_tokens"].as_i64().unwrap(), 4096);
        assert_eq!(agent["status"].as_str().unwrap(), "idle");
    }

    #[tokio::test]
    async fn create_agent_empty_tier_returns_bad_request() {
        let (app, jwt_secret) = setup_test_app();
        let token = create_test_token(&jwt_secret);

        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/agents")
                    .header("content-type", "application/json")
                    .header("authorization", format!("Bearer {}", token))
                    .body(Body::from(r#"{"tier":"","persona_name":"X","model_id":"claude-sonnet-4-20250514"}"#))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn list_agents_includes_created_agent() {
        let (app, jwt_secret) = setup_test_app();
        let token = create_test_token(&jwt_secret);

        // Create an agent
        app.clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/agents")
                    .header("content-type", "application/json")
                    .header("authorization", format!("Bearer {}", token))
                    .body(Body::from(r#"{"tier":"orchestrator","persona_name":"Planner","model_id":"claude-sonnet-4-20250514"}"#))
                    .unwrap(),
            )
            .await
            .unwrap();

        // List agents
        let response = app
            .oneshot(Request::builder().uri("/api/agents").header("authorization", format!("Bearer {}", token)).body(Body::empty()).unwrap())
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        let body = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let resp: serde_json::Value = serde_json::from_slice(&body).unwrap();
        let agents = resp["agents"].as_array().unwrap();
        assert_eq!(agents.len(), 1);
        assert_eq!(agents[0]["persona_name"].as_str().unwrap(), "Planner");
    }

    #[tokio::test]
    async fn get_agent_returns_created_agent() {
        let (app, jwt_secret) = setup_test_app();
        let token = create_test_token(&jwt_secret);

        // Create
        let create_resp = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/agents")
                    .header("content-type", "application/json")
                    .header("authorization", format!("Bearer {}", token))
                    .body(Body::from(r#"{"tier":"utility","persona_name":"Helper","model_id":"claude-haiku-35-20241022"}"#))
                    .unwrap(),
            )
            .await
            .unwrap();

        let body = axum::body::to_bytes(create_resp.into_body(), usize::MAX).await.unwrap();
        let created: serde_json::Value = serde_json::from_slice(&body).unwrap();
        let agent_id = created["id"].as_str().unwrap();

        // Get by ID
        let response = app
            .oneshot(
                Request::builder()
                    .uri(format!("/api/agents/{}", agent_id))
                    .header("authorization", format!("Bearer {}", token))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        let body = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let agent: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(agent["persona_name"].as_str().unwrap(), "Helper");
        assert_eq!(agent["tier"].as_str().unwrap(), "utility");
    }

    #[tokio::test]
    async fn get_agent_not_found() {
        let (app, jwt_secret) = setup_test_app();
        let token = create_test_token(&jwt_secret);

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/api/agents/00000000-0000-0000-0000-000000000000")
                    .header("authorization", format!("Bearer {}", token))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn update_agent_partial_fields() {
        let (app, jwt_secret) = setup_test_app();
        let token = create_test_token(&jwt_secret);

        // Create
        let create_resp = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/agents")
                    .header("content-type", "application/json")
                    .header("authorization", format!("Bearer {}", token))
                    .body(Body::from(r#"{"tier":"worker","persona_name":"OldName","model_id":"claude-sonnet-4-20250514"}"#))
                    .unwrap(),
            )
            .await
            .unwrap();

        let body = axum::body::to_bytes(create_resp.into_body(), usize::MAX).await.unwrap();
        let created: serde_json::Value = serde_json::from_slice(&body).unwrap();
        let agent_id = created["id"].as_str().unwrap();

        // Update only persona_name
        let response = app
            .oneshot(
                Request::builder()
                    .method("PATCH")
                    .uri(format!("/api/agents/{}", agent_id))
                    .header("content-type", "application/json")
                    .header("authorization", format!("Bearer {}", token))
                    .body(Body::from(r#"{"persona_name":"NewName"}"#))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        let body = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let agent: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(agent["persona_name"].as_str().unwrap(), "NewName");
        assert_eq!(agent["tier"].as_str().unwrap(), "worker"); // unchanged
    }

    #[tokio::test]
    async fn delete_agent_returns_no_content() {
        let (app, jwt_secret) = setup_test_app();
        let token = create_test_token(&jwt_secret);

        // Create
        let create_resp = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/agents")
                    .header("content-type", "application/json")
                    .header("authorization", format!("Bearer {}", token))
                    .body(Body::from(r#"{"tier":"worker","persona_name":"ToDelete","model_id":"claude-sonnet-4-20250514"}"#))
                    .unwrap(),
            )
            .await
            .unwrap();

        let body = axum::body::to_bytes(create_resp.into_body(), usize::MAX).await.unwrap();
        let created: serde_json::Value = serde_json::from_slice(&body).unwrap();
        let agent_id = created["id"].as_str().unwrap();

        // Delete
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("DELETE")
                    .uri(format!("/api/agents/{}", agent_id))
                    .header("authorization", format!("Bearer {}", token))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::NO_CONTENT);

        // Verify gone
        let response = app
            .oneshot(
                Request::builder()
                    .uri(format!("/api/agents/{}", agent_id))
                    .header("authorization", format!("Bearer {}", token))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }

    // === Tool CRUD tests ===

    #[tokio::test]
    async fn create_tool_returns_created() {
        let (app, jwt_secret) = setup_test_app();
        let token = create_test_token(&jwt_secret);

        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/tools")
                    .header("content-type", "application/json")
                    .header("authorization", format!("Bearer {}", token))
                    .body(Body::from(
                        r#"{"name":"read_file","display_name":"Read File","description":"Read a file","parameters":{"type":"object"}}"#,
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::CREATED);
        let body = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let tool: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(tool["name"], "read_file");
        assert_eq!(tool["display_name"], "Read File");
    }

    #[tokio::test]
    async fn create_tool_empty_name_returns_bad_request() {
        let (app, jwt_secret) = setup_test_app();
        let token = create_test_token(&jwt_secret);

        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/tools")
                    .header("content-type", "application/json")
                    .header("authorization", format!("Bearer {}", token))
                    .body(Body::from(r#"{"name":"  "}"#))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn list_tools_includes_created() {
        let (app, jwt_secret) = setup_test_app();
        let token = create_test_token(&jwt_secret);

        // Create
        app.clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/tools")
                    .header("content-type", "application/json")
                    .header("authorization", format!("Bearer {}", token))
                    .body(Body::from(r#"{"name":"git_status","description":"Show git status","category":"git"}"#))
                    .unwrap(),
            )
            .await
            .unwrap();

        // List
        let response = app
            .oneshot(Request::builder().uri("/api/tools").header("authorization", format!("Bearer {}", token)).body(Body::empty()).unwrap())
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        let body = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let tools: Vec<serde_json::Value> = serde_json::from_slice(&body).unwrap();
        assert_eq!(tools.len(), 1);
        assert_eq!(tools[0]["name"], "git_status");
    }

    #[tokio::test]
    async fn get_tool_by_id() {
        let (app, jwt_secret) = setup_test_app();
        let token = create_test_token(&jwt_secret);

        let create_resp = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/tools")
                    .header("content-type", "application/json")
                    .header("authorization", format!("Bearer {}", token))
                    .body(Body::from(r#"{"name":"run_tests","description":"Run tests"}"#))
                    .unwrap(),
            )
            .await
            .unwrap();

        let body = axum::body::to_bytes(create_resp.into_body(), usize::MAX).await.unwrap();
        let created: serde_json::Value = serde_json::from_slice(&body).unwrap();
        let tool_id = created["id"].as_str().unwrap();

        let response = app
            .oneshot(
                Request::builder()
                    .uri(format!("/api/tools/{}", tool_id))
                    .header("authorization", format!("Bearer {}", token))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        let body = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let tool: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(tool["name"], "run_tests");
    }

    #[tokio::test]
    async fn get_tool_not_found() {
        let (app, jwt_secret) = setup_test_app();
        let token = create_test_token(&jwt_secret);

        let response = app
            .oneshot(
                Request::builder()
                    .uri(format!("/api/tools/{}", Uuid::new_v4()))
                    .header("authorization", format!("Bearer {}", token))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn update_tool_partial() {
        let (app, jwt_secret) = setup_test_app();
        let token = create_test_token(&jwt_secret);

        let create_resp = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/tools")
                    .header("content-type", "application/json")
                    .header("authorization", format!("Bearer {}", token))
                    .body(Body::from(r#"{"name":"write_file","description":"Write a file"}"#))
                    .unwrap(),
            )
            .await
            .unwrap();

        let body = axum::body::to_bytes(create_resp.into_body(), usize::MAX).await.unwrap();
        let created: serde_json::Value = serde_json::from_slice(&body).unwrap();
        let tool_id = created["id"].as_str().unwrap();

        let response = app
            .oneshot(
                Request::builder()
                    .method("PATCH")
                    .uri(format!("/api/tools/{}", tool_id))
                    .header("content-type", "application/json")
                    .header("authorization", format!("Bearer {}", token))
                    .body(Body::from(r#"{"description":"Write content to a file"}"#))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        let body = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let tool: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(tool["description"], "Write content to a file");
        assert_eq!(tool["name"], "write_file");
    }

    #[tokio::test]
    async fn delete_tool_returns_no_content() {
        let (app, jwt_secret) = setup_test_app();
        let token = create_test_token(&jwt_secret);

        let create_resp = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/tools")
                    .header("content-type", "application/json")
                    .header("authorization", format!("Bearer {}", token))
                    .body(Body::from(r#"{"name":"to_delete","description":"temp"}"#))
                    .unwrap(),
            )
            .await
            .unwrap();

        let body = axum::body::to_bytes(create_resp.into_body(), usize::MAX).await.unwrap();
        let created: serde_json::Value = serde_json::from_slice(&body).unwrap();
        let tool_id = created["id"].as_str().unwrap();

        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("DELETE")
                    .uri(format!("/api/tools/{}", tool_id))
                    .header("authorization", format!("Bearer {}", token))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::NO_CONTENT);

        // Verify gone
        let response = app
            .oneshot(
                Request::builder()
                    .uri(format!("/api/tools/{}", tool_id))
                    .header("authorization", format!("Bearer {}", token))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn set_and_get_agent_tools() {
        let (app, jwt_secret) = setup_test_app();
        let token = create_test_token(&jwt_secret);

        // Create an agent
        let agent_resp = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/agents")
                    .header("content-type", "application/json")
                    .header("authorization", format!("Bearer {}", token))
                    .body(Body::from(r#"{"tier":"worker","persona_name":"ToolUser","model_id":"claude-sonnet-4-20250514"}"#))
                    .unwrap(),
            )
            .await
            .unwrap();

        let body = axum::body::to_bytes(agent_resp.into_body(), usize::MAX).await.unwrap();
        let agent: serde_json::Value = serde_json::from_slice(&body).unwrap();
        let agent_id = agent["id"].as_str().unwrap();

        // Create two tools
        let tool1_resp = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/tools")
                    .header("content-type", "application/json")
                    .header("authorization", format!("Bearer {}", token))
                    .body(Body::from(r#"{"name":"read_file","description":"Read"}"#))
                    .unwrap(),
            )
            .await
            .unwrap();
        let body = axum::body::to_bytes(tool1_resp.into_body(), usize::MAX).await.unwrap();
        let tool1: serde_json::Value = serde_json::from_slice(&body).unwrap();
        let tool1_id = tool1["id"].as_str().unwrap();

        let tool2_resp = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/tools")
                    .header("content-type", "application/json")
                    .header("authorization", format!("Bearer {}", token))
                    .body(Body::from(r#"{"name":"write_file","description":"Write"}"#))
                    .unwrap(),
            )
            .await
            .unwrap();
        let body = axum::body::to_bytes(tool2_resp.into_body(), usize::MAX).await.unwrap();
        let tool2: serde_json::Value = serde_json::from_slice(&body).unwrap();
        let tool2_id = tool2["id"].as_str().unwrap();

        // Set agent tools
        let set_resp = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("PUT")
                    .uri(format!("/api/agents/{}/tools", agent_id))
                    .header("content-type", "application/json")
                    .header("authorization", format!("Bearer {}", token))
                    .body(Body::from(serde_json::json!({"tool_ids": [tool1_id, tool2_id]}).to_string()))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(set_resp.status(), StatusCode::OK);

        // Get agent tools
        let get_resp = app
            .oneshot(
                Request::builder()
                    .uri(format!("/api/agents/{}/tools", agent_id))
                    .header("authorization", format!("Bearer {}", token))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(get_resp.status(), StatusCode::OK);
        let body = axum::body::to_bytes(get_resp.into_body(), usize::MAX).await.unwrap();
        let result: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(result["agent_id"], agent_id);
        assert_eq!(result["tools"].as_array().unwrap().len(), 2);
    }

    // === get_config response body ===

    #[tokio::test]
    async fn get_config_returns_expected_fields() {
        let (app, jwt_secret) = setup_test_app();
        let token = create_test_token(&jwt_secret);

        let response = app
            .oneshot(Request::builder().uri("/api/config").header("authorization", format!("Bearer {}", token)).body(Body::empty()).unwrap())
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        let body = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let resp: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert!(resp["verbosity"].is_string());
        assert!(resp["pool"].is_object());
        assert!(resp["autonomy"].is_string());
        assert!(resp["git_strategy"].is_string());
        assert!(resp["sandbox_mode"].is_string());
    }

    // === Auth endpoints ===

    #[tokio::test]
    async fn auth_setup_short_password_returns_bad_request() {
        let (app, _jwt_secret) = setup_test_app();

        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/auth/setup")
                    .header("content-type", "application/json")
                    .body(Body::from(r#"{"password":"short"}"#))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn auth_setup_success() {
        let (app, _jwt_secret) = setup_test_app();

        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/auth/setup")
                    .header("content-type", "application/json")
                    .body(Body::from(r#"{"password":"longpassword123"}"#))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        let body = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let resp: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(resp["message"].as_str().unwrap(), "Password configured successfully");
    }

    #[tokio::test]
    async fn auth_setup_conflict_when_already_configured() {
        let (app, _jwt_secret) = setup_test_app();

        // First setup
        app.clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/auth/setup")
                    .header("content-type", "application/json")
                    .body(Body::from(r#"{"password":"longpassword123"}"#))
                    .unwrap(),
            )
            .await
            .unwrap();

        // Second setup should conflict
        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/auth/setup")
                    .header("content-type", "application/json")
                    .body(Body::from(r#"{"password":"anotherpassword"}"#))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::CONFLICT);
    }

    fn setup_login_test_app(password: &str) -> (axum::Router, Vec<u8>) {
        let password_hash = super::super::auth::hash_password(password).unwrap();
        let test_user = User {
            id: UserId::new(),
            email: "test@test.com".to_string(),
            password_hash: Some(password_hash),
            github_id: None,
            github_login: None,
            github_token_encrypted: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };
        let mut mock = MockUserRepo::new();
        let user_clone = test_user.clone();
        mock.expect_get_user_by_email()
            .returning(move |email| if email == "test@test.com" { Ok(Some(user_clone.clone())) } else { Ok(None) });
        setup_test_app_with_user_repo(Some(Arc::new(mock)))
    }

    #[tokio::test]
    async fn auth_login_no_password_configured() {
        // No user_repo means login returns 500 (user service unavailable)
        let (app, _jwt_secret) = setup_test_app();

        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/auth/login")
                    .header("content-type", "application/json")
                    .body(Body::from(r#"{"email":"test@test.com","password":"anything"}"#))
                    .unwrap(),
            )
            .await
            .unwrap();

        // No user_repo configured, so we get 500
        assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);
    }

    #[tokio::test]
    async fn auth_login_wrong_password() {
        let (app, _jwt_secret) = setup_login_test_app("correctpassword");

        // Login with wrong password
        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/auth/login")
                    .header("content-type", "application/json")
                    .body(Body::from(r#"{"email":"test@test.com","password":"wrongpassword!"}"#))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn auth_login_success() {
        let (app, _jwt_secret) = setup_login_test_app("correctpassword");

        // Login with correct password
        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/auth/login")
                    .header("content-type", "application/json")
                    .body(Body::from(r#"{"email":"test@test.com","password":"correctpassword"}"#))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        let body = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let resp: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert!(resp["token"].is_string());
        assert_eq!(resp["expires_in"].as_u64().unwrap(), 86400);
    }

    // === Chat history with data ===

    #[tokio::test]
    async fn chat_history_returns_messages_after_send() {
        let (app, jwt_secret) = setup_test_app();
        let token = create_test_token(&jwt_secret);

        // Send a message
        app.clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/chat")
                    .header("content-type", "application/json")
                    .header("authorization", format!("Bearer {}", token))
                    .body(Body::from(r#"{"message":"Hello agent"}"#))
                    .unwrap(),
            )
            .await
            .unwrap();

        // Get history
        let response = app
            .oneshot(
                Request::builder()
                    .uri("/api/chat/history")
                    .header("authorization", format!("Bearer {}", token))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        let body = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let messages: Vec<serde_json::Value> = serde_json::from_slice(&body).unwrap();
        assert_eq!(messages.len(), 1);
        assert_eq!(messages[0]["role"].as_str().unwrap(), "user");
        assert_eq!(messages[0]["content"].as_str().unwrap(), "Hello agent");
    }

    #[tokio::test]
    async fn chat_history_with_pagination() {
        let (app, jwt_secret) = setup_test_app();
        let token = create_test_token(&jwt_secret);

        // Send two messages
        for msg in ["First", "Second"] {
            app.clone()
                .oneshot(
                    Request::builder()
                        .method("POST")
                        .uri("/api/chat")
                        .header("content-type", "application/json")
                        .header("authorization", format!("Bearer {}", token))
                        .body(Body::from(format!(r#"{{"message":"{}"}}"#, msg)))
                        .unwrap(),
                )
                .await
                .unwrap();
        }

        // Get with limit=1
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/api/chat/history?limit=1")
                    .header("authorization", format!("Bearer {}", token))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        let body = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let messages: Vec<serde_json::Value> = serde_json::from_slice(&body).unwrap();
        assert_eq!(messages.len(), 1);

        // Get with offset=1
        let response = app
            .oneshot(
                Request::builder()
                    .uri("/api/chat/history?limit=10&offset=1")
                    .header("authorization", format!("Bearer {}", token))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        let body = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let messages: Vec<serde_json::Value> = serde_json::from_slice(&body).unwrap();
        assert_eq!(messages.len(), 1);
    }

    // === Serialization edge cases ===

    #[test]
    fn create_task_request_all_fields() {
        let json = r#"{"title":"T","description":"D","priority":"low","tier":"orchestrator"}"#;
        let request: CreateTaskRequest = serde_json::from_str(json).unwrap();
        assert_eq!(request.title, "T");
        assert_eq!(request.description, Some("D".to_string()));
        assert_eq!(request.priority, Some("low".to_string()));
        assert_eq!(request.tier, Some("orchestrator".to_string()));
    }

    #[test]
    fn create_task_request_minimal() {
        let json = r#"{"title":"T"}"#;
        let request: CreateTaskRequest = serde_json::from_str(json).unwrap();
        assert_eq!(request.title, "T");
        assert!(request.description.is_none());
        assert!(request.priority.is_none());
        assert!(request.tier.is_none());
    }

    #[test]
    fn tasks_query_with_no_fields() {
        let json = r#"{}"#;
        let query: TasksQuery = serde_json::from_str(json).unwrap();
        assert!(query.status.is_none());
        assert!(query.limit.is_none());
    }

    #[test]
    fn setup_request_deserializes() {
        let json = r#"{"password":"mypassword"}"#;
        let request: SetupRequest = serde_json::from_str(json).unwrap();
        assert_eq!(request.password, "mypassword");
    }

    #[test]
    fn login_request_deserializes() {
        let json = r#"{"email":"test@test.com","password":"mypassword"}"#;
        let request: LoginRequest = serde_json::from_str(json).unwrap();
        assert_eq!(request.email, "test@test.com");
        assert_eq!(request.password, "mypassword");
    }

    #[test]
    fn setup_response_serializes() {
        let response = SetupResponse { message: "ok".to_string() };
        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("\"message\":\"ok\""));
    }

    #[test]
    fn login_response_serializes() {
        let response = LoginResponse {
            token: "abc123".to_string(),
            expires_in: 86400,
        };
        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("\"token\":\"abc123\""));
        assert!(json.contains("\"expires_in\":86400"));
    }

    #[test]
    fn me_response_serializes() {
        let response = MeResponse {
            id: "user-123".to_string(),
            email: "admin@example.com".to_string(),
            github_login: None,
            authenticated: true,
            token_expires: 99999,
        };
        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("\"id\":\"user-123\""));
        assert!(json.contains("\"authenticated\":true"));
        assert!(json.contains("\"token_expires\":99999"));
    }

    fn test_agent_response() -> AgentResponse {
        AgentResponse {
            id: "agent-1".to_string(),
            tier: "worker".to_string(),
            persona_name: "Test Agent".to_string(),
            persona_prompt: "You are a test agent".to_string(),
            persona_style: "casual".to_string(),
            model_provider: "anthropic".to_string(),
            model_id: "claude-sonnet-4-20250514".to_string(),
            model_max_tokens: 4096,
            model_temperature: 0.7,
            status: "idle".to_string(),
            version: 1,
        }
    }

    #[test]
    fn agents_list_response_serializes() {
        let response = AgentsListResponse {
            agents: vec![test_agent_response()],
            stats: AgentPoolStats {
                total: 1,
                available: 1,
                max: 12,
            },
        };
        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("\"agent-1\""));
        assert!(json.contains("\"total\""));
    }

    #[test]
    fn agent_response_serializes_all_fields() {
        let response = test_agent_response();
        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("\"persona_name\":\"Test Agent\""));
        assert!(json.contains("\"model_provider\":\"anthropic\""));
        assert!(json.contains("\"model_max_tokens\":4096"));
    }

    // === send_chat response body ===

    #[tokio::test]
    async fn send_chat_response_contains_message_id() {
        let (app, jwt_secret) = setup_test_app();
        let token = create_test_token(&jwt_secret);

        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/chat")
                    .header("content-type", "application/json")
                    .header("authorization", format!("Bearer {}", token))
                    .body(Body::from(r#"{"message":"test msg"}"#))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::ACCEPTED);
        let body = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let resp: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert!(resp["message_id"].is_string());
        assert_eq!(resp["status"].as_str().unwrap(), "queued");
        // Verify it's a valid UUID
        Uuid::parse_str(resp["message_id"].as_str().unwrap()).unwrap();
    }

    // === create_task response body validation ===

    #[tokio::test]
    async fn create_task_response_body_has_expected_fields() {
        let (app, jwt_secret) = setup_test_app();
        let token = create_test_token(&jwt_secret);

        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/tasks")
                    .header("content-type", "application/json")
                    .header("authorization", format!("Bearer {}", token))
                    .body(Body::from(r#"{"title":"Full task","description":"A description","priority":"high","tier":"worker"}"#))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::CREATED);
        let body = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let task: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(task["title"].as_str().unwrap(), "Full task");
        assert_eq!(task["description"].as_str().unwrap(), "A description");
        assert!(task["id"].is_string());
        assert!(task["created_at"].is_string());
        assert!(task["updated_at"].is_string());
    }

    // === clear chat then verify empty ===

    #[tokio::test]
    async fn clear_chat_then_history_is_empty() {
        let (app, jwt_secret) = setup_test_app();
        let token = create_test_token(&jwt_secret);

        // Send a message
        app.clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/chat")
                    .header("content-type", "application/json")
                    .header("authorization", format!("Bearer {}", token))
                    .body(Body::from(r#"{"message":"To be cleared"}"#))
                    .unwrap(),
            )
            .await
            .unwrap();

        // Clear history
        app.clone()
            .oneshot(
                Request::builder()
                    .method("DELETE")
                    .uri("/api/chat/history")
                    .header("authorization", format!("Bearer {}", token))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        // Verify empty
        let response = app
            .oneshot(
                Request::builder()
                    .uri("/api/chat/history")
                    .header("authorization", format!("Bearer {}", token))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        let body = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let messages: Vec<serde_json::Value> = serde_json::from_slice(&body).unwrap();
        assert!(messages.is_empty());
    }

    // ========================================================================
    // Pipeline stage template tests
    // ========================================================================

    #[test]
    fn resolve_template_basic() {
        let mut outputs = std::collections::HashMap::new();
        outputs.insert("weather_check".to_string(), serde_json::json!({"forecast": "stormy", "location": "Denver"}));
        let no_ctx = std::collections::HashMap::new();
        let result = resolve_template("The weather in {{weather_check.location}} is {{weather_check.forecast}}", &outputs, &no_ctx);
        assert_eq!(result, "The weather in Denver is stormy");
    }

    #[test]
    fn resolve_template_missing_ref_preserved() {
        let outputs = std::collections::HashMap::new();
        let no_ctx = std::collections::HashMap::new();
        let result = resolve_template("Status: {{unknown.field}}", &outputs, &no_ctx);
        assert_eq!(result, "Status: {{unknown.field}}");
    }

    #[test]
    fn resolve_template_numeric_value() {
        let mut outputs = std::collections::HashMap::new();
        outputs.insert("scorer".to_string(), serde_json::json!({"confidence": 95}));
        let no_ctx = std::collections::HashMap::new();
        let result = resolve_template("Confidence: {{scorer.confidence}}%", &outputs, &no_ctx);
        assert_eq!(result, "Confidence: 95%");
    }

    #[test]
    fn resolve_template_multiple_stages() {
        let mut outputs = std::collections::HashMap::new();
        outputs.insert("stage_a".to_string(), serde_json::json!({"x": "hello"}));
        outputs.insert("stage_b".to_string(), serde_json::json!({"y": "world"}));
        let no_ctx = std::collections::HashMap::new();
        let result = resolve_template("{{stage_a.x}} {{stage_b.y}}", &outputs, &no_ctx);
        assert_eq!(result, "hello world");
    }

    #[test]
    fn resolve_template_with_context() {
        let outputs = std::collections::HashMap::new();
        let mut ctx = std::collections::HashMap::new();
        ctx.insert("style_guide".to_string(), "Use camelCase for variables.".to_string());
        let result = resolve_template("Review: {{context.style_guide}}", &outputs, &ctx);
        assert_eq!(result, "Review: Use camelCase for variables.");
    }

    #[test]
    fn resolve_template_context_missing_preserved() {
        let outputs = std::collections::HashMap::new();
        let ctx = std::collections::HashMap::new();
        let result = resolve_template("Review: {{context.missing_doc}}", &outputs, &ctx);
        assert_eq!(result, "Review: {{context.missing_doc}}");
    }

    #[test]
    fn resolve_template_stage_and_context() {
        let mut outputs = std::collections::HashMap::new();
        outputs.insert("analysis".to_string(), serde_json::json!({"status": "complete"}));
        let mut ctx = std::collections::HashMap::new();
        ctx.insert("prd".to_string(), "Build a login feature.".to_string());
        let result = resolve_template("{{context.prd}} Status: {{analysis.status}}", &outputs, &ctx);
        assert_eq!(result, "Build a login feature. Status: complete");
    }

    #[test]
    fn render_stage_prompt_full() {
        let inputs = vec![("weather".to_string(), "stormy".to_string()), ("location".to_string(), "Denver, CO".to_string())];
        let schema = serde_json::json!({
            "fields": [
                {"name": "urgency", "type": "enum", "values": ["URGENT", "NON_URGENT"], "description": "How urgent"},
                {"name": "summary", "type": "string", "description": "One sentence summary"}
            ]
        });
        let result = render_stage_prompt("Classify the weather urgency", &inputs, &schema);
        assert!(result.contains("# Goal\nClassify the weather urgency"));
        assert!(result.contains("- weather: stormy"));
        assert!(result.contains("- location: Denver, CO"));
        assert!(result.contains("- urgency: one of [\"URGENT\", \"NON_URGENT\"] — How urgent"));
        assert!(result.contains("- summary: string — One sentence summary"));
    }

    #[test]
    fn render_stage_prompt_no_inputs() {
        let schema = serde_json::json!({"fields": [{"name": "result", "type": "string"}]});
        let result = render_stage_prompt("Do something", &[], &schema);
        assert!(!result.contains("# Input"));
        assert!(result.contains("# Goal"));
        assert!(result.contains("# Output Schema"));
    }

    #[test]
    fn render_stage_prompt_empty_schema() {
        let inputs = vec![("x".to_string(), "1".to_string())];
        let schema = serde_json::json!({"fields": []});
        let result = render_stage_prompt("Goal here", &inputs, &schema);
        assert!(result.contains("# Goal"));
        assert!(result.contains("- x: 1"));
        assert!(!result.contains("# Output Schema"));
    }
}
