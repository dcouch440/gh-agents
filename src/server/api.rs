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
use crate::types::{AgentPoolConfig, AgentTier, Priority, Task, TierModels};

// ============================================================================
// Health Endpoint (Slice 10.2.1)
// ============================================================================

/// Health check response
#[derive(Serialize)]
pub struct HealthResponse {
    pub status: String,
    pub version: String,
    pub db_connected: bool,
}

/// Enhanced health check endpoint
///
/// Returns JSON with status details including version and database connectivity.
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
#[derive(Deserialize)]
pub struct TasksQuery {
    pub status: Option<String>,
    pub limit: Option<u32>,
}

/// List all tasks with optional filtering
///
/// Supports query parameters:
/// - `status`: Filter by task status (pending, in_progress, completed, etc.)
/// - `limit`: Maximum number of tasks to return (default 100, max 1000)
pub async fn list_tasks(State(state): State<AppState>, auth: auth::AuthUser, Query(query): Query<TasksQuery>) -> Result<Json<Vec<Task>>, StatusCode> {
    let tasks = state.repo.list_tasks(auth.user_id, query.status, query.limit).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(tasks))
}

/// Get a single task by ID
///
/// Returns 404 if the task is not found.
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
#[derive(Deserialize)]
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
pub async fn create_task(State(state): State<AppState>, auth: auth::AuthUser, Json(request): Json<CreateTaskRequest>) -> Result<(StatusCode, Json<Task>), StatusCode> {
    if request.title.trim().is_empty() || request.title.len() > MAX_TITLE_LENGTH {
        return Err(StatusCode::BAD_REQUEST);
    }
    if let Some(ref desc) = request.description {
        if desc.len() > MAX_DESCRIPTION_LENGTH {
            return Err(StatusCode::BAD_REQUEST);
        }
    }

    // Parse tier (default to Worker)
    let tier = request
        .tier
        .as_ref()
        .map(|t| match t.to_lowercase().as_str() {
            "orchestrator" => AgentTier::Orchestrator,
            "utility" => AgentTier::Utility,
            _ => AgentTier::Worker,
        })
        .unwrap_or(AgentTier::Worker);

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
    let mut task = Task::new(request.title.trim(), tier);
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
#[derive(Serialize)]
pub struct AgentResponse {
    pub id: String,
    pub tier: String,
    pub persona_name: String,
    pub persona_prompt: String,
    pub persona_style: String,
    pub model_provider: String,
    pub model_id: String,
    pub model_max_tokens: i32,
    pub model_temperature: f32,
    pub status: String,
}

impl AgentResponse {
    fn from_row(row: crate::db::AgentRow) -> Self {
        Self {
            id: row.id.to_string(),
            tier: row.tier,
            persona_name: row.persona_name,
            persona_prompt: row.persona_prompt,
            persona_style: row.persona_style,
            model_provider: row.model_provider,
            model_id: row.model_id,
            model_max_tokens: row.model_max_tokens,
            model_temperature: row.model_temperature,
            status: row.status,
        }
    }
}

/// Response for the agents list endpoint
#[derive(Serialize)]
pub struct AgentsListResponse {
    pub agents: Vec<AgentResponse>,
    pub stats: AgentPoolStats,
}

/// Agent pool statistics
#[derive(Serialize)]
pub struct AgentPoolStats {
    pub orchestrators: TierStats,
    pub workers: TierStats,
    pub utilities: TierStats,
}

/// Statistics for a single tier
#[derive(Serialize)]
pub struct TierStats {
    pub total: usize,
    pub available: usize,
    pub max: u8,
}

/// Request to create a new agent
#[derive(Deserialize)]
pub struct CreateAgentRequest {
    pub tier: String,
    pub persona_name: String,
    pub persona_prompt: Option<String>,
    pub persona_style: Option<String>,
    pub model_provider: Option<String>,
    pub model_id: String,
    pub model_max_tokens: Option<i32>,
    pub model_temperature: Option<f32>,
}

/// Request to update an existing agent
#[derive(Deserialize)]
pub struct UpdateAgentRequest {
    pub tier: Option<String>,
    pub persona_name: Option<String>,
    pub persona_prompt: Option<String>,
    pub persona_style: Option<String>,
    pub model_provider: Option<String>,
    pub model_id: Option<String>,
    pub model_max_tokens: Option<i32>,
    pub model_temperature: Option<f32>,
}

/// List all agents and their status
pub async fn list_agents(State(state): State<AppState>, auth: auth::AuthUser) -> Result<Json<AgentsListResponse>, StatusCode> {
    let config = state.config.read().await;
    let pool_config = &config.pool;

    let rows = state.repo.list_persisted_agents(auth.user_id).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let agents: Vec<AgentResponse> = rows.into_iter().map(AgentResponse::from_row).collect();

    let count_tier = |tier: &str| agents.iter().filter(|a| a.tier == tier).count();

    let response = AgentsListResponse {
        stats: AgentPoolStats {
            orchestrators: TierStats {
                total: count_tier("orchestrator"),
                available: count_tier("orchestrator"),
                max: pool_config.max_orchestrators,
            },
            workers: TierStats {
                total: count_tier("worker"),
                available: count_tier("worker"),
                max: pool_config.max_workers,
            },
            utilities: TierStats {
                total: count_tier("utility"),
                available: count_tier("utility"),
                max: pool_config.max_utilities,
            },
        },
        agents,
    };

    Ok(Json(response))
}

/// Create a new agent
pub async fn create_agent(State(state): State<AppState>, auth: auth::AuthUser, Json(request): Json<CreateAgentRequest>) -> Result<(StatusCode, Json<AgentResponse>), StatusCode> {
    if request.tier.trim().is_empty() || request.model_id.trim().is_empty() {
        return Err(StatusCode::BAD_REQUEST);
    }
    if request.persona_name.len() > MAX_TITLE_LENGTH {
        return Err(StatusCode::BAD_REQUEST);
    }
    if let Some(ref prompt) = request.persona_prompt {
        if prompt.len() > MAX_PROMPT_LENGTH {
            return Err(StatusCode::BAD_REQUEST);
        }
    }

    let row = crate::db::AgentRow {
        id: Uuid::new_v4(),
        tier: request.tier.trim().to_lowercase(),
        persona_name: request.persona_name.trim().to_string(),
        persona_prompt: request.persona_prompt.unwrap_or_default(),
        persona_style: request.persona_style.unwrap_or_else(|| "casual".to_string()),
        model_provider: request.model_provider.unwrap_or_else(|| "anthropic".to_string()),
        model_id: request.model_id.trim().to_string(),
        model_max_tokens: request.model_max_tokens.unwrap_or(4096),
        model_temperature: request.model_temperature.unwrap_or(0.7),
        status: "idle".to_string(),
        router_mode: false,
    };

    state.repo.upsert_agent(auth.user_id, row.clone()).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok((StatusCode::CREATED, Json(AgentResponse::from_row(row))))
}

/// Get a single agent by ID
pub async fn get_agent(State(state): State<AppState>, _auth: auth::AuthUser, Path(id): Path<Uuid>) -> Result<Json<AgentResponse>, StatusCode> {
    let row = state.repo.get_persisted_agent(id).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?.ok_or(StatusCode::NOT_FOUND)?;

    Ok(Json(AgentResponse::from_row(row)))
}

/// Update an existing agent (partial)
pub async fn update_agent(State(state): State<AppState>, auth: auth::AuthUser, Path(id): Path<Uuid>, Json(request): Json<UpdateAgentRequest>) -> Result<Json<AgentResponse>, StatusCode> {
    let existing = state.repo.get_persisted_agent(id).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?.ok_or(StatusCode::NOT_FOUND)?;

    let updated = crate::db::AgentRow {
        id: existing.id,
        tier: request.tier.unwrap_or(existing.tier),
        persona_name: request.persona_name.unwrap_or(existing.persona_name),
        persona_prompt: request.persona_prompt.unwrap_or(existing.persona_prompt),
        persona_style: request.persona_style.unwrap_or(existing.persona_style),
        model_provider: request.model_provider.unwrap_or(existing.model_provider),
        model_id: request.model_id.unwrap_or(existing.model_id),
        model_max_tokens: request.model_max_tokens.unwrap_or(existing.model_max_tokens),
        model_temperature: request.model_temperature.unwrap_or(existing.model_temperature),
        status: existing.status,
        router_mode: existing.router_mode,
    };

    state.repo.upsert_agent(auth.user_id, updated.clone()).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(AgentResponse::from_row(updated)))
}

/// Delete an agent by ID
pub async fn delete_agent(State(state): State<AppState>, _auth: auth::AuthUser, Path(id): Path<Uuid>) -> Result<StatusCode, StatusCode> {
    state.repo.delete_persisted_agent(id).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(StatusCode::NO_CONTENT)
}

// ============================================================================
// Tool Endpoints
// ============================================================================

/// Response for a single tool
#[derive(Serialize)]
pub struct ToolResponse {
    pub id: String,
    pub name: String,
    pub description: String,
    pub category: String,
    pub parameter_schema: serde_json::Value,
    pub output_schema: serde_json::Value,
    pub enabled: bool,
    pub cluster_id: Option<String>,
    pub is_builtin: bool,
}

impl ToolResponse {
    fn from_row(row: crate::db::ToolRow) -> Self {
        Self {
            id: row.id.to_string(),
            name: row.name,
            description: row.description,
            category: row.category,
            parameter_schema: row.parameter_schema,
            output_schema: row.output_schema,
            enabled: row.enabled,
            cluster_id: row.cluster_id.map(|id| id.to_string()),
            is_builtin: row.is_builtin,
        }
    }
}

/// Request to create a new tool
#[derive(Deserialize)]
pub struct CreateToolRequest {
    pub name: String,
    pub description: Option<String>,
    pub category: Option<String>,
    pub parameter_schema: Option<serde_json::Value>,
    pub output_schema: Option<serde_json::Value>,
    pub enabled: Option<bool>,
    pub cluster_id: Option<String>,
}

/// Request to update an existing tool
#[derive(Deserialize)]
pub struct UpdateToolRequest {
    pub name: Option<String>,
    pub description: Option<String>,
    pub category: Option<String>,
    pub parameter_schema: Option<serde_json::Value>,
    pub output_schema: Option<serde_json::Value>,
    pub enabled: Option<bool>,
    pub cluster_id: Option<String>,
}

/// Request to set tools for an agent
#[derive(Deserialize)]
pub struct SetAgentToolsRequest {
    pub tool_ids: Vec<String>,
}

/// Response for agent tools
#[derive(Serialize)]
pub struct AgentToolsResponse {
    pub agent_id: String,
    pub tools: Vec<ToolResponse>,
}

/// List all tools
pub async fn list_tools(State(state): State<AppState>, auth: auth::AuthUser) -> Result<Json<Vec<ToolResponse>>, StatusCode> {
    let rows = state.repo.list_tools(auth.user_id).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let tools = rows.into_iter().map(ToolResponse::from_row).collect();
    Ok(Json(tools))
}

/// Create a new tool
pub async fn create_tool(State(state): State<AppState>, auth: auth::AuthUser, Json(request): Json<CreateToolRequest>) -> Result<(StatusCode, Json<ToolResponse>), StatusCode> {
    if request.name.trim().is_empty() {
        return Err(StatusCode::BAD_REQUEST);
    }

    let cluster_id = request.cluster_id.as_deref().map(Uuid::parse_str).transpose().map_err(|_| StatusCode::BAD_REQUEST)?;

    let row = crate::db::ToolRow {
        id: Uuid::new_v4(),
        name: request.name.trim().to_string(),
        description: request.description.unwrap_or_default(),
        category: request.category.unwrap_or_else(|| "general".to_string()),
        parameter_schema: request.parameter_schema.unwrap_or_else(|| serde_json::json!({})),
        output_schema: request.output_schema.unwrap_or_else(|| serde_json::json!({})),
        enabled: request.enabled.unwrap_or(true),
        cluster_id,
        is_builtin: false,
    };

    state.repo.upsert_tool(auth.user_id, row.clone()).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok((StatusCode::CREATED, Json(ToolResponse::from_row(row))))
}

/// Get a single tool by ID
pub async fn get_tool(State(state): State<AppState>, _auth: auth::AuthUser, Path(id): Path<Uuid>) -> Result<Json<ToolResponse>, StatusCode> {
    let row = state.repo.get_tool(id).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?.ok_or(StatusCode::NOT_FOUND)?;

    Ok(Json(ToolResponse::from_row(row)))
}

/// Update an existing tool (partial)
pub async fn update_tool(State(state): State<AppState>, auth: auth::AuthUser, Path(id): Path<Uuid>, Json(request): Json<UpdateToolRequest>) -> Result<Json<ToolResponse>, StatusCode> {
    let existing = state.repo.get_tool(id).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?.ok_or(StatusCode::NOT_FOUND)?;

    let cluster_id = match &request.cluster_id {
        Some(cid) => Some(Uuid::parse_str(cid).map_err(|_| StatusCode::BAD_REQUEST)?),
        None => existing.cluster_id,
    };

    let updated = crate::db::ToolRow {
        id: existing.id,
        name: request.name.unwrap_or(existing.name),
        description: request.description.unwrap_or(existing.description),
        category: request.category.unwrap_or(existing.category),
        parameter_schema: request.parameter_schema.unwrap_or(existing.parameter_schema),
        output_schema: request.output_schema.unwrap_or(existing.output_schema),
        enabled: request.enabled.unwrap_or(existing.enabled),
        cluster_id,
        is_builtin: existing.is_builtin,
    };

    state.repo.upsert_tool(auth.user_id, updated.clone()).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(ToolResponse::from_row(updated)))
}

/// Delete a tool by ID
pub async fn delete_tool(State(state): State<AppState>, _auth: auth::AuthUser, Path(id): Path<Uuid>) -> Result<StatusCode, StatusCode> {
    state.repo.delete_tool(id).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(StatusCode::NO_CONTENT)
}

/// Get tools assigned to an agent
pub async fn get_agent_tools(State(state): State<AppState>, _auth: auth::AuthUser, Path(agent_id): Path<Uuid>) -> Result<Json<AgentToolsResponse>, StatusCode> {
    let rows = state.repo.get_agent_tools(agent_id).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let tools = rows.into_iter().map(ToolResponse::from_row).collect();

    Ok(Json(AgentToolsResponse {
        agent_id: agent_id.to_string(),
        tools,
    }))
}

/// Set tools for an agent (replaces existing)
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

#[derive(Deserialize)]
pub struct SetAgentContextRequest {
    pub document_ids: Vec<String>,
}

#[derive(Serialize)]
pub struct AgentContextResponse {
    pub agent_id: String,
    pub documents: Vec<DocumentListItem>,
}

/// Get context documents assigned to an agent
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
#[derive(Serialize)]
pub struct ConfigResponse {
    pub verbosity: String,
    pub models: TierModels,
    pub pool: AgentPoolConfig,
    pub autonomy: String,
    pub git_strategy: String,
    pub sandbox_mode: String,
}

/// Get current configuration
pub async fn get_config(State(state): State<AppState>) -> Json<ConfigResponse> {
    let config = state.config.read().await;

    Json(ConfigResponse {
        verbosity: format!("{:?}", config.verbosity).to_lowercase(),
        models: config.models.clone(),
        pool: config.pool.clone(),
        autonomy: format!("{:?}", config.autonomy).to_lowercase(),
        git_strategy: format!("{:?}", config.git_strategy).to_lowercase(),
        sandbox_mode: format!("{:?}", config.sandbox_mode).to_lowercase(),
    })
}

/// Request body for updating a single model tier's config
#[derive(Deserialize)]
pub struct UpdateModelConfig {
    pub model_id: Option<String>,
    pub max_tokens: Option<u32>,
    pub temperature: Option<f32>,
}

/// Request body for updating model configs by tier
#[derive(Deserialize)]
pub struct UpdateModelsRequest {
    pub orchestrator: Option<UpdateModelConfig>,
    pub worker: Option<UpdateModelConfig>,
    pub utility: Option<UpdateModelConfig>,
}

/// Request body for updating pool sizes
#[derive(Deserialize)]
pub struct UpdatePoolRequest {
    pub max_orchestrators: Option<u8>,
    pub max_workers: Option<u8>,
    pub max_utilities: Option<u8>,
}

/// Request body for updating configuration
#[derive(Deserialize)]
pub struct UpdateConfigRequest {
    pub verbosity: Option<String>,
    pub models: Option<UpdateModelsRequest>,
    pub pool: Option<UpdatePoolRequest>,
    pub autonomy: Option<String>,
    pub git_strategy: Option<String>,
    pub sandbox_mode: Option<String>,
}

/// Update configuration (partial update)
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

    // Models
    if let Some(ref models) = request.models {
        fn apply_model(target: &mut crate::types::ModelConfig, update: &UpdateModelConfig) {
            if let Some(ref id) = update.model_id {
                target.model_id = id.clone();
            }
            if let Some(tokens) = update.max_tokens {
                target.max_tokens = tokens;
            }
            if let Some(temp) = update.temperature {
                target.temperature = temp;
            }
        }
        if let Some(ref o) = models.orchestrator {
            apply_model(&mut config.models.orchestrator, o);
        }
        if let Some(ref w) = models.worker {
            apply_model(&mut config.models.worker, w);
        }
        if let Some(ref u) = models.utility {
            apply_model(&mut config.models.utility, u);
        }
    }

    // Pool
    if let Some(ref pool) = request.pool {
        if let Some(v) = pool.max_orchestrators {
            config.pool.max_orchestrators = v;
        }
        if let Some(v) = pool.max_workers {
            config.pool.max_workers = v;
        }
        if let Some(v) = pool.max_utilities {
            config.pool.max_utilities = v;
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
        models: config.models.clone(),
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
#[derive(Deserialize)]
pub struct ChatRequest {
    pub message: String,
}

/// Response for sending a chat message
#[derive(Serialize)]
pub struct ChatResponse {
    pub message_id: Uuid,
    pub status: String,
}

/// Send a chat message to the orchestrator
///
/// Returns 202 Accepted with the message ID.
/// The message is queued for processing by the orchestrator.
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
            mode_id: crate::server::agent_mode::AgentModeId::new("home"),
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
#[derive(Deserialize)]
pub struct HistoryQuery {
    pub limit: Option<u32>,
    pub offset: Option<u32>,
}

/// A chat message in the response
#[derive(Serialize)]
pub struct ChatMessage {
    pub id: Uuid,
    pub role: String,
    pub content: String,
    pub timestamp: DateTime<Utc>,
}

/// Get chat history with pagination
///
/// Returns messages in chronological order.
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
pub async fn chat_stream(State(state): State<AppState>, Path(message_id): Path<Uuid>) -> Sse<impl Stream<Item = Result<Event, Infallible>>> {
    chat_stream_inner(state, message_id)
}

/// Stream chat response for session-scoped messages.
///
/// Same as `chat_stream` but extracts both session_id and message_id
/// from the path (only message_id is used for stream lookup).
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
#[derive(Serialize)]
pub struct ModeInfo {
    pub id: String,
    pub name: String,
    pub description: String,
}

/// List available agent modes
pub async fn list_modes(State(state): State<AppState>, _auth: auth::AuthUser) -> Json<Vec<ModeInfo>> {
    let modes: Vec<ModeInfo> = state
        .mode_registry
        .list()
        .into_iter()
        .map(|m| ModeInfo {
            id: m.id.0.clone(),
            name: m.name.clone(),
            description: m.description.clone(),
        })
        .collect();
    Json(modes)
}

/// Request body for creating a session
#[derive(Deserialize)]
pub struct CreateSessionRequest {
    pub mode_id: String,
    #[serde(default)]
    pub title: String,
}

/// Request body for updating a session
#[derive(Deserialize)]
pub struct UpdateSessionRequest {
    pub title: String,
}

/// Response for session creation
#[derive(Serialize)]
pub struct SessionResponse {
    pub id: Uuid,
    pub mode_id: String,
    pub title: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Create a new chat session
pub async fn create_session(State(state): State<AppState>, auth: auth::AuthUser, Json(request): Json<CreateSessionRequest>) -> Result<(StatusCode, Json<SessionResponse>), StatusCode> {
    // Validate mode exists
    let mode_id = crate::server::agent_mode::AgentModeId::new(&request.mode_id);
    if state.mode_registry.get(&mode_id).is_none() {
        return Err(StatusCode::BAD_REQUEST);
    }

    let session_id = Uuid::new_v4();
    let title = if request.title.is_empty() { format!("New {} session", request.mode_id) } else { request.title };

    state
        .repo
        .create_session(auth.user_id, session_id, &request.mode_id, &title)
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
            title: session.title,
            created_at: session.created_at,
            updated_at: session.updated_at,
        }),
    ))
}

/// List sessions for the current user
pub async fn list_sessions(State(state): State<AppState>, auth: auth::AuthUser) -> Result<Json<Vec<SessionResponse>>, StatusCode> {
    let sessions = state.repo.list_sessions(auth.user_id).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let response: Vec<SessionResponse> = sessions
        .into_iter()
        .map(|s| SessionResponse {
            id: s.id,
            mode_id: s.mode_id,
            title: s.title,
            created_at: s.created_at,
            updated_at: s.updated_at,
        })
        .collect();

    Ok(Json(response))
}

/// Get a specific session
pub async fn get_session(State(state): State<AppState>, auth: auth::AuthUser, Path(session_id): Path<Uuid>) -> Result<Json<SessionResponse>, StatusCode> {
    let session = state.repo.get_session(session_id).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?.ok_or(StatusCode::NOT_FOUND)?;

    // Verify ownership
    if session.user_id != auth.user_id.0 {
        return Err(StatusCode::NOT_FOUND);
    }

    Ok(Json(SessionResponse {
        id: session.id,
        mode_id: session.mode_id,
        title: session.title,
        created_at: session.created_at,
        updated_at: session.updated_at,
    }))
}

/// Delete a session
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
        title: updated.title,
        created_at: updated.created_at,
        updated_at: updated.updated_at,
    }))
}

/// Send a message to a session
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
            mode_id: crate::server::agent_mode::AgentModeId::new(&session.mode_id),
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
#[derive(Deserialize)]
pub struct SetupRequest {
    pub password: String,
}

/// Response for auth setup
#[derive(Serialize)]
pub struct SetupResponse {
    pub message: String,
}

/// POST /api/auth/setup - First-run password configuration
///
/// This endpoint is only available when no password has been configured yet.
/// Once a password is set, this endpoint returns 409 Conflict.
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
#[derive(Deserialize)]
pub struct RegisterRequest {
    pub email: String,
    pub password: String,
}

/// Response for registration
#[derive(Serialize)]
pub struct AuthTokenResponse {
    pub token: String,
    pub expires_in: u64,
    pub user: UserResponse,
}

/// User info in API responses
#[derive(Serialize)]
pub struct UserResponse {
    pub id: String,
    pub email: String,
    pub github_login: Option<String>,
}

/// POST /api/auth/register - Register a new user
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
#[derive(Deserialize)]
pub struct LoginRequest {
    pub email: String,
    pub password: String,
}

/// Response for successful login
#[derive(Serialize)]
pub struct LoginResponse {
    pub token: String,
    pub expires_in: u64,
}

/// POST /api/auth/login - Authenticate and get JWT token
///
/// Verifies the provided password and returns a JWT token valid for 24 hours.
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
#[derive(Serialize)]
pub struct MeResponse {
    pub user: String,
    pub authenticated: bool,
    pub token_expires: usize,
}

/// GET /api/auth/me - Get current user info from token
///
/// Requires a valid JWT token in Authorization header.
pub async fn auth_me(auth: auth::AuthUser) -> Json<MeResponse> {
    Json(MeResponse {
        user: auth.user_id.to_string(),
        authenticated: true,
        token_expires: auth.claims.exp,
    })
}

// ============================================================================
// Document Endpoints
// ============================================================================

/// List item for documents (excludes content).
#[derive(Serialize)]
pub struct DocumentListItem {
    pub id: Uuid,
    pub title: String,
    pub summary: String,
    pub ref_tag: String,
    pub tags: Vec<String>,
    pub doc_type: String,
    pub updated_at: DateTime<Utc>,
}

/// Response for a full document (includes content).
#[derive(Serialize)]
pub struct DocumentResponse {
    pub id: Uuid,
    pub title: String,
    pub content: String,
    pub summary: String,
    pub ref_tag: String,
    pub tags: Vec<String>,
    pub doc_type: String,
    pub session_id: Option<Uuid>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Request body for creating a document.
#[derive(Deserialize)]
pub struct CreateDocumentRequest {
    pub title: String,
    pub content: String,
    pub doc_type: Option<String>,
    pub session_id: Option<Uuid>,
    pub tags: Option<Vec<String>>,
}

/// Request body for updating a document.
#[derive(Deserialize)]
pub struct UpdateDocumentRequest {
    pub content: Option<String>,
    pub title: Option<String>,
    pub tags: Option<Vec<String>>,
}

/// Query parameters for document search.
#[derive(Deserialize)]
pub struct DocumentSearchQuery {
    pub q: String,
}

/// GET /api/documents - List all documents for the authenticated user.
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
pub async fn search_documents(State(state): State<AppState>, auth: auth::AuthUser, Query(query): Query<DocumentSearchQuery>) -> Result<Json<Vec<crate::db::DocumentSearchResult>>, StatusCode> {
    let doc_repo = state.doc_repo.as_ref().ok_or(StatusCode::INTERNAL_SERVER_ERROR)?;
    let results = doc_repo.search_documents(auth.user_id.0, &query.q).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(results))
}

/// GET /api/documents/:id - Get a full document by ID.
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
#[derive(Serialize)]
pub struct OutputSchemaResponse {
    pub id: Uuid,
    pub name: String,
    pub schema: serde_json::Value,
    pub created_at: DateTime<Utc>,
}

/// Request body for creating an output schema.
#[derive(Deserialize)]
pub struct CreateOutputSchemaRequest {
    pub name: String,
    pub schema: serde_json::Value,
}

/// Request body for updating an output schema.
#[derive(Deserialize)]
pub struct UpdateOutputSchemaRequest {
    pub name: Option<String>,
    pub schema: Option<serde_json::Value>,
}

/// GET /api/output-schemas - List all output schemas for the authenticated user.
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
#[derive(Serialize)]
pub struct PromptTemplateResponse {
    pub id: Uuid,
    pub name: String,
    pub content: String,
    pub created_at: DateTime<Utc>,
}

/// Request body for creating a prompt template.
#[derive(Deserialize)]
pub struct CreatePromptTemplateRequest {
    pub name: String,
    pub content: String,
}

/// Request body for updating a prompt template.
#[derive(Deserialize)]
pub struct UpdatePromptTemplateRequest {
    pub name: Option<String>,
    pub content: Option<String>,
}

/// GET /api/prompt-templates
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

#[derive(Serialize)]
pub struct StageMemberResponse {
    pub id: Uuid,
    pub pipeline_id: Uuid,
    pub stage_number: i32,
    pub workflow_id: Uuid,
    pub display_order: i32,
}

#[derive(Deserialize)]
pub struct CreateStageMemberRequest {
    pub workflow_id: Uuid,
    pub display_order: Option<i32>,
}

#[derive(Deserialize)]
pub struct UpdateStageMemberRequest {
    pub display_order: i32,
}

#[derive(Deserialize)]
pub struct StageMemberPath {
    pub pid: Uuid,
    pub num: i32,
}

#[derive(Deserialize)]
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

#[derive(Serialize)]
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
    pub status: String,
    pub input_tokens: i64,
    pub output_tokens: i64,
    pub cost_usd: f32,
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
            status: r.status,
            input_tokens: r.input_tokens,
            output_tokens: r.output_tokens,
            cost_usd: r.cost_usd,
            started_at: r.started_at,
            completed_at: r.completed_at,
        }
    }
}

#[derive(Serialize)]
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

pub async fn get_agent_execution(State(state): State<AppState>, _auth: auth::AuthUser, Path(id): Path<Uuid>) -> Result<Json<AgentExecutionResponse>, StatusCode> {
    let repo = state.agent_execution_repo.as_ref().ok_or(StatusCode::INTERNAL_SERVER_ERROR)?;
    let row = repo.get_agent_execution(id).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?.ok_or(StatusCode::NOT_FOUND)?;
    Ok(Json(AgentExecutionResponse::from(row)))
}

pub async fn list_execution_messages(State(state): State<AppState>, _auth: auth::AuthUser, Path(id): Path<Uuid>) -> Result<Json<Vec<ExecutionMessageResponse>>, StatusCode> {
    let repo = state.agent_execution_repo.as_ref().ok_or(StatusCode::INTERNAL_SERVER_ERROR)?;
    // Verify execution exists
    repo.get_agent_execution(id).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?.ok_or(StatusCode::NOT_FOUND)?;
    let rows = repo.list_execution_messages(id).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Json(rows.into_iter().map(ExecutionMessageResponse::from).collect()))
}

// ============================================================================
// Cost Endpoints
// ============================================================================

#[derive(Deserialize)]
pub struct CostQuery {
    pub since: Option<DateTime<Utc>>,
}

#[derive(Serialize)]
pub struct CostResponse {
    pub total_spend: f64,
    pub models: Vec<crate::db::traits::ModelSpendRow>,
}

pub async fn get_costs(State(state): State<AppState>, auth: auth::AuthUser, Query(q): Query<CostQuery>) -> Result<Json<CostResponse>, StatusCode> {
    let repo = state.token_ledger_repo.as_ref().ok_or(StatusCode::INTERNAL_SERVER_ERROR)?;
    let total_spend = repo.get_user_spend(auth.user_id.0, q.since).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let models = repo.get_model_breakdown(auth.user_id.0, q.since).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Json(CostResponse { total_spend, models }))
}

// ============================================================================
// Result Endpoints
// ============================================================================

#[derive(Serialize)]
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

#[derive(Deserialize)]
pub struct ResultQuery {
    pub output_schema_id: Option<Uuid>,
}

pub async fn list_results(State(state): State<AppState>, auth: auth::AuthUser, Query(q): Query<ResultQuery>) -> Result<Json<Vec<ResultResponse>>, StatusCode> {
    let repo = state.result_repo.as_ref().ok_or(StatusCode::INTERNAL_SERVER_ERROR)?;
    let rows = match q.output_schema_id {
        Some(schema_id) => repo.list_results_by_schema(auth.user_id.0, schema_id).await,
        None => repo.list_results(auth.user_id.0).await,
    }
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Json(rows.into_iter().map(ResultResponse::from).collect()))
}

pub async fn get_result(State(state): State<AppState>, auth: auth::AuthUser, Path(id): Path<Uuid>) -> Result<Json<ResultResponse>, StatusCode> {
    let repo = state.result_repo.as_ref().ok_or(StatusCode::INTERNAL_SERVER_ERROR)?;
    let row = repo.get_result(id).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?.ok_or(StatusCode::NOT_FOUND)?;
    if row.user_id != auth.user_id.0 {
        return Err(StatusCode::NOT_FOUND);
    }
    Ok(Json(ResultResponse::from(row)))
}

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

#[derive(Serialize)]
pub struct WorkflowResponse {
    pub id: Uuid,
    pub name: String,
    pub description: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Deserialize)]
pub struct CreateWorkflowRequest {
    pub name: String,
    pub description: Option<String>,
}

#[derive(Deserialize)]
pub struct UpdateWorkflowRequest {
    pub name: Option<String>,
    pub description: Option<String>,
}

#[derive(Serialize)]
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
    pub display_order: i32,
}

#[derive(Deserialize)]
pub struct CreateStepRequest {
    pub agent_id: Uuid,
    pub execution_mode: Option<String>,
    pub for_each_ref: Option<String>,
    pub prompt_template_id: Option<Uuid>,
    pub prompt_template: Option<String>,
    pub output_schema_id: Option<Uuid>,
    pub output_variable_name: Option<String>,
    pub interactive_agent_id: Option<Uuid>,
    pub display_order: Option<i32>,
}

#[derive(Deserialize)]
pub struct UpdateStepRequest {
    pub agent_id: Uuid,
    pub execution_mode: Option<String>,
    pub for_each_ref: Option<String>,
    pub prompt_template_id: Option<Uuid>,
    pub prompt_template: Option<String>,
    pub output_schema_id: Option<Uuid>,
    pub output_variable_name: Option<String>,
    pub interactive_agent_id: Option<Uuid>,
    pub display_order: Option<i32>,
}

#[derive(Deserialize)]
pub struct EdgeRequest {
    pub from_step_id: Uuid,
    pub to_step_id: Uuid,
}

#[derive(Serialize)]
pub struct EdgeResponse {
    pub from_step_id: Uuid,
    pub to_step_id: Uuid,
}

#[derive(Deserialize)]
pub struct StepDocumentRequest {
    pub document_id: Uuid,
}

#[derive(Serialize)]
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
        display_order: r.display_order,
    }
}

/// GET /api/workflows
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
        display_order: req.display_order.unwrap_or(0),
    };
    let row = repo.create_step(step).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok((StatusCode::CREATED, Json(step_response(row))))
}

/// GET /api/workflows/:id/steps
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

#[derive(Deserialize)]
pub struct WorkflowStepPath {
    pub wid: Uuid,
    pub sid: Uuid,
}

/// PUT /api/workflows/:wid/steps/:sid
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
        display_order: req.display_order.unwrap_or(existing.display_order),
    };
    let row = repo.update_step(step).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Json(step_response(row)))
}

/// DELETE /api/workflows/:wid/steps/:sid
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
#[derive(Deserialize)]
pub struct ContextResponseRequest {
    pub agent_id: Uuid,
    pub task_id: Uuid,
    pub context: String,
    pub files: Vec<FilePathContent>,
}

/// A file with path and content for context responses
#[derive(Deserialize)]
pub struct FilePathContent {
    pub path: String,
    pub content: String,
}

/// POST /api/context-response - Submit a human context response to an agent
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
// Usage Stats Endpoint (F5 - Cost Dashboard)
// ============================================================================

/// Get token usage summary for the last 24 hours.
pub async fn get_usage_stats(State(state): State<AppState>) -> Result<Json<Vec<crate::db::UsageSummaryRow>>, StatusCode> {
    let stats = state.repo.get_usage_summary(24).await.map_err(|e| {
        tracing::error!("get_usage_stats failed: {e:?}");
        StatusCode::INTERNAL_SERVER_ERROR
    })?;
    Ok(Json(stats))
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
    let mut all_text = stage.output_description.clone();
    if let Some(defs) = stage.input_definitions.as_array() {
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
    if let Some(defs) = stage.input_definitions.as_array() {
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
    let resolved_description = resolve_template(&stage.output_description, stage_outputs, &context_docs);

    render_stage_prompt(&resolved_description, &resolved_inputs, &stage.output_schema)
}

/// Request body for rendering a pipeline stage prompt.
#[derive(Deserialize)]
pub struct RenderStageRequest {
    /// Map of stage_name → JSON output from that stage.
    pub stage_outputs: std::collections::HashMap<String, serde_json::Value>,
}

/// Render a pipeline stage into a resolved prompt (HTTP endpoint wrapper).
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
// Stage Side Tasks
// ============================================================================

#[derive(Deserialize)]
pub struct CreateSideTaskRequest {
    pub agent_id: String,
    pub input_definitions: Option<serde_json::Value>,
    pub output_name: Option<String>,
    pub blocking: Option<bool>,
    pub output_schema: Option<serde_json::Value>,
}

#[derive(Serialize)]
pub struct SideTaskResponse {
    pub id: String,
    pub agent_id: String,
    pub input_definitions: serde_json::Value,
    pub output_name: String,
    pub blocking: bool,
    pub output_schema: serde_json::Value,
}

impl SideTaskResponse {
    fn from_row(row: crate::db::StageSideTaskRow) -> Self {
        Self {
            id: row.id.to_string(),
            agent_id: row.agent_id.to_string(),
            input_definitions: row.input_definitions,
            output_name: row.output_name,
            blocking: row.blocking,
            output_schema: row.output_schema,
        }
    }
}

pub async fn list_stage_side_tasks(State(state): State<AppState>, _user: auth::AuthUser, Path((pipeline_id, stage_number)): Path<(String, i32)>) -> Result<Json<Vec<SideTaskResponse>>, StatusCode> {
    let Ok(pipeline_uuid) = Uuid::parse_str(&pipeline_id) else {
        return Err(StatusCode::BAD_REQUEST);
    };
    let rows = state.repo.list_stage_side_tasks(pipeline_uuid, stage_number).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let tasks = rows.into_iter().map(SideTaskResponse::from_row).collect();
    Ok(Json(tasks))
}

pub async fn create_stage_side_task(
    State(state): State<AppState>,
    _user: auth::AuthUser,
    Path((pipeline_id, stage_number)): Path<(String, i32)>,
    Json(request): Json<CreateSideTaskRequest>,
) -> Result<(StatusCode, Json<SideTaskResponse>), StatusCode> {
    let Ok(pipeline_uuid) = Uuid::parse_str(&pipeline_id) else {
        return Err(StatusCode::BAD_REQUEST);
    };
    let Ok(agent_uuid) = Uuid::parse_str(&request.agent_id) else {
        return Err(StatusCode::BAD_REQUEST);
    };

    let row = crate::db::StageSideTaskRow {
        id: Uuid::new_v4(),
        pipeline_id: pipeline_uuid,
        stage_number,
        agent_id: agent_uuid,
        input_definitions: request.input_definitions.unwrap_or_else(|| serde_json::json!([])),
        output_name: request.output_name.unwrap_or_default(),
        blocking: request.blocking.unwrap_or(false),
        output_schema: request.output_schema.unwrap_or_else(|| serde_json::json!({"fields": []})),
    };

    state.repo.upsert_stage_side_task(row.clone()).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok((StatusCode::CREATED, Json(SideTaskResponse::from_row(row))))
}

pub async fn delete_stage_side_task(
    State(state): State<AppState>,
    _user: auth::AuthUser,
    Path((_pipeline_id, _stage_number, side_task_id)): Path<(String, i32, String)>,
) -> Result<StatusCode, StatusCode> {
    let Ok(side_task_uuid) = Uuid::parse_str(&side_task_id) else {
        return Err(StatusCode::BAD_REQUEST);
    };
    state.repo.delete_stage_side_task(side_task_uuid).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(StatusCode::NO_CONTENT)
}

// ============================================================================
// Pipeline Run Endpoints
// ============================================================================

#[derive(Deserialize)]
pub struct ApproveRunRequest {
    pub user_input: Option<String>,
}

#[derive(Serialize)]
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
            stage_outputs: row.stage_outputs,
            current_stage: row.current_stage,
            started_at: row.started_at.to_rfc3339(),
            completed_at: row.completed_at.map(|t| t.to_rfc3339()),
            total_input_tokens: row.total_input_tokens,
            total_output_tokens: row.total_output_tokens,
        }
    }
}

#[derive(Serialize)]
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

#[derive(Serialize)]
pub struct PipelineRunDetailResponse {
    #[serde(flatten)]
    pub run: PipelineRunResponse,
    pub stages: Vec<StageExecutionResponse>,
}

#[derive(Deserialize)]
pub struct ListRunsQuery {
    pub pipeline_id: Option<String>,
}

/// List pipeline runs, optionally filtered by pipeline_id.
pub async fn list_pipeline_runs(State(state): State<AppState>, _user: auth::AuthUser, Query(query): Query<ListRunsQuery>) -> Result<Json<Vec<PipelineRunResponse>>, StatusCode> {
    let pipeline_id = query.pipeline_id.as_deref().and_then(|s| Uuid::parse_str(s).ok()).ok_or(StatusCode::BAD_REQUEST)?;

    let runs = state.repo.list_pipeline_runs(pipeline_id).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(runs.into_iter().map(PipelineRunResponse::from_row).collect()))
}

/// Get a pipeline run with its stage executions.
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
                                        path: format!("context:{}", if doc.ref_tag.is_empty() { &doc.title } else { &doc.ref_tag }),
                                        content: doc.content.clone(),
                                    });
                                }
                            }
                            Some(aid.clone())
                        } else if let Some(cid) = &next_stage.cluster_id {
                            match state.repo.list_cluster_members(cid.0).await {
                                Ok(member_ids) => {
                                    let picked = member_ids.first().map(|mid| crate::agents::AgentId(*mid));
                                    if let Some(aid) = &picked {
                                        if let Ok(docs) = state.repo.get_agent_context(aid.0).await {
                                            for doc in &docs {
                                                context_reading.push(crate::agents::FileContent {
                                                    path: format!("context:{}", if doc.ref_tag.is_empty() { &doc.title } else { &doc.ref_tag }),
                                                    content: doc.content.clone(),
                                                });
                                            }
                                        }
                                    }
                                    picked
                                }
                                Err(_) => None,
                            }
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
            models: TierModels::default(),
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
            orchestrators: TierStats { total: 1, available: 1, max: 2 },
            workers: TierStats { total: 3, available: 2, max: 6 },
            utilities: TierStats { total: 2, available: 2, max: 4 },
        };
        let json = serde_json::to_string(&stats).unwrap();
        assert!(json.contains("\"orchestrators\""));
        assert!(json.contains("\"workers\""));
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
    use crate::db::{ChatMessageRow, PipelineRow, PipelineStageRow, ScheduleRow, TriggerRow};
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
        async fn list_persisted_clusters(&self, _user_id: UserId) -> anyhow::Result<Vec<crate::db::ClusterRow>> {
            Ok(vec![])
        }
        async fn upsert_cluster(&self, _user_id: UserId, _cluster: crate::db::ClusterRow) -> anyhow::Result<()> {
            Ok(())
        }
        async fn delete_cluster(&self, _cluster_id: Uuid) -> anyhow::Result<()> {
            Ok(())
        }
        async fn list_cluster_members(&self, _cluster_id: Uuid) -> anyhow::Result<Vec<Uuid>> {
            Ok(vec![])
        }
        async fn add_cluster_member(&self, _cluster_id: Uuid, _agent_id: Uuid) -> anyhow::Result<()> {
            Ok(())
        }
        async fn remove_cluster_member(&self, _cluster_id: Uuid, _agent_id: Uuid) -> anyhow::Result<()> {
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
        async fn list_stage_side_tasks(&self, _pipeline_id: Uuid, _stage_number: i32) -> anyhow::Result<Vec<crate::db::StageSideTaskRow>> {
            Ok(vec![])
        }
        async fn upsert_stage_side_task(&self, _side_task: crate::db::StageSideTaskRow) -> anyhow::Result<()> {
            Ok(())
        }
        async fn delete_stage_side_task(&self, _side_task_id: Uuid) -> anyhow::Result<()> {
            Ok(())
        }
        async fn list_schedules(&self, _user_id: UserId) -> anyhow::Result<Vec<ScheduleRow>> {
            Ok(vec![])
        }
        async fn upsert_schedule(&self, _user_id: UserId, _schedule: ScheduleRow) -> anyhow::Result<()> {
            Ok(())
        }
        async fn delete_schedule(&self, _schedule_id: Uuid) -> anyhow::Result<()> {
            Ok(())
        }
        async fn update_schedule_last_run(&self, _schedule_id: Uuid, _last_run_at: DateTime<Utc>) -> anyhow::Result<()> {
            Ok(())
        }
        async fn list_triggers(&self, _user_id: UserId) -> anyhow::Result<Vec<TriggerRow>> {
            Ok(vec![])
        }
        async fn upsert_trigger(&self, _user_id: UserId, _trigger: TriggerRow) -> anyhow::Result<()> {
            Ok(())
        }
        async fn delete_trigger(&self, _trigger_id: Uuid) -> anyhow::Result<()> {
            Ok(())
        }
        async fn create_session(&self, _user_id: UserId, _session_id: Uuid, _mode_id: &str, _title: &str) -> anyhow::Result<()> {
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
        async fn insert_token_usage(&self, _session_id: Option<Uuid>, _agent_id: Option<Uuid>, _tier: &str, _model_id: &str, _input_tokens: i64, _output_tokens: i64) -> anyhow::Result<()> {
            Ok(())
        }
        async fn get_usage_summary(&self, _since_hours: u32) -> anyhow::Result<Vec<crate::db::UsageSummaryRow>> {
            Ok(vec![])
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

        async fn insert_tool_call(
            &self,
            _session_id: Option<Uuid>,
            _message_id: Uuid,
            _round: i32,
            _tool_name: &str,
            _tool_use_id: &str,
            _input: &serde_json::Value,
            _output: &str,
            _latency_ms: i32,
        ) -> anyhow::Result<()> {
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
        let (mut state, rx) = AppState::with_repo(None, repo, None, config);
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

    // === Tier and priority parsing tests ===

    #[tokio::test]
    async fn create_task_with_orchestrator_tier() {
        let (app, jwt_secret) = setup_test_app();
        let token = create_test_token(&jwt_secret);

        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/tasks")
                    .header("content-type", "application/json")
                    .header("authorization", format!("Bearer {}", token))
                    .body(Body::from(r#"{"title":"Tier test","tier":"orchestrator"}"#))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::CREATED);
        let body = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let body_str = String::from_utf8(body.to_vec()).unwrap();
        assert!(body_str.contains("\"assigned_tier\":\"orchestrator\""));
    }

    #[tokio::test]
    async fn create_task_with_utility_tier() {
        let (app, jwt_secret) = setup_test_app();
        let token = create_test_token(&jwt_secret);

        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/tasks")
                    .header("content-type", "application/json")
                    .header("authorization", format!("Bearer {}", token))
                    .body(Body::from(r#"{"title":"Util test","tier":"utility"}"#))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::CREATED);
        let body = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let body_str = String::from_utf8(body.to_vec()).unwrap();
        assert!(body_str.contains("\"assigned_tier\":\"utility\""));
    }

    #[tokio::test]
    async fn create_task_with_unknown_tier_defaults_to_worker() {
        let (app, jwt_secret) = setup_test_app();
        let token = create_test_token(&jwt_secret);

        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/tasks")
                    .header("content-type", "application/json")
                    .header("authorization", format!("Bearer {}", token))
                    .body(Body::from(r#"{"title":"Default tier","tier":"nonexistent"}"#))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::CREATED);
        let body = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let body_str = String::from_utf8(body.to_vec()).unwrap();
        assert!(body_str.contains("\"assigned_tier\":\"worker\""));
    }

    #[tokio::test]
    async fn create_task_with_no_tier_defaults_to_worker() {
        let (app, jwt_secret) = setup_test_app();
        let token = create_test_token(&jwt_secret);

        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/tasks")
                    .header("content-type", "application/json")
                    .header("authorization", format!("Bearer {}", token))
                    .body(Body::from(r#"{"title":"No tier"}"#))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::CREATED);
        let body = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let body_str = String::from_utf8(body.to_vec()).unwrap();
        assert!(body_str.contains("\"assigned_tier\":\"worker\""));
    }

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
        assert!(resp["stats"]["orchestrators"].is_object());
        assert!(resp["stats"]["workers"].is_object());
        assert!(resp["stats"]["utilities"].is_object());
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
                        r#"{"name":"read_file","description":"Read a file","category":"file","parameter_schema":{"type":"object"},"output_schema":{"type":"object"}}"#,
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::CREATED);
        let body = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let tool: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(tool["name"], "read_file");
        assert_eq!(tool["category"], "file");
        assert_eq!(tool["enabled"], true);
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
                    .body(Body::from(r#"{"name":"write_file","description":"Write a file","category":"file"}"#))
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
                    .body(Body::from(r#"{"description":"Write content to a file","enabled":false}"#))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        let body = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let tool: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(tool["description"], "Write content to a file");
        assert_eq!(tool["enabled"], false);
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
        assert!(resp["models"].is_object());
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
            user: "admin".to_string(),
            authenticated: true,
            token_expires: 99999,
        };
        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("\"user\":\"admin\""));
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
        }
    }

    #[test]
    fn agents_list_response_serializes() {
        let response = AgentsListResponse {
            agents: vec![test_agent_response()],
            stats: AgentPoolStats {
                orchestrators: TierStats { total: 0, available: 0, max: 1 },
                workers: TierStats { total: 1, available: 1, max: 4 },
                utilities: TierStats { total: 0, available: 0, max: 2 },
            },
        };
        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("\"agent-1\""));
        assert!(json.contains("\"workers\""));
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

    // === Stage side task endpoint tests ===

    #[tokio::test]
    async fn list_stage_side_tasks_returns_200() {
        let (app, jwt_secret) = setup_test_app();
        let token = create_test_token(&jwt_secret);
        let pipeline_id = Uuid::new_v4();

        let response = app
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri(format!("/api/pipelines/{}/stages/0/side-tasks", pipeline_id))
                    .header("authorization", format!("Bearer {}", token))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn create_stage_side_task_returns_201() {
        let (app, jwt_secret) = setup_test_app();
        let token = create_test_token(&jwt_secret);
        let pipeline_id = Uuid::new_v4();
        let agent_id = Uuid::new_v4();

        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(format!("/api/pipelines/{}/stages/0/side-tasks", pipeline_id))
                    .header("content-type", "application/json")
                    .header("authorization", format!("Bearer {}", token))
                    .body(Body::from(
                        serde_json::json!({
                            "agent_id": agent_id.to_string(),
                            "output_name": "docs",
                            "blocking": false
                        })
                        .to_string(),
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::CREATED);
        let body = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let resp: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(resp["agent_id"].as_str().unwrap(), agent_id.to_string());
        assert_eq!(resp["output_name"].as_str().unwrap(), "docs");
        assert_eq!(resp["blocking"].as_bool().unwrap(), false);
    }

    #[tokio::test]
    async fn delete_stage_side_task_returns_204() {
        let (app, jwt_secret) = setup_test_app();
        let token = create_test_token(&jwt_secret);
        let pipeline_id = Uuid::new_v4();
        let side_task_id = Uuid::new_v4();

        let response = app
            .oneshot(
                Request::builder()
                    .method("DELETE")
                    .uri(format!("/api/pipelines/{}/stages/0/side-tasks/{}", pipeline_id, side_task_id))
                    .header("authorization", format!("Bearer {}", token))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::NO_CONTENT);
    }
}
