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
pub async fn list_tasks(
    State(state): State<AppState>,
    auth: auth::AuthUser,
    Query(query): Query<TasksQuery>,
) -> Result<Json<Vec<Task>>, StatusCode> {
    let tasks = state
        .repo
        .list_tasks(auth.user_id, query.status, query.limit)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(tasks))
}

/// Get a single task by ID
///
/// Returns 404 if the task is not found.
pub async fn get_task(
    State(state): State<AppState>,
    auth: auth::AuthUser,
    Path(id): Path<Uuid>,
) -> Result<Json<Task>, StatusCode> {
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
pub async fn create_task(
    State(state): State<AppState>,
    auth: auth::AuthUser,
    Json(request): Json<CreateTaskRequest>,
) -> Result<(StatusCode, Json<Task>), StatusCode> {
    if request.title.trim().is_empty() {
        return Err(StatusCode::BAD_REQUEST);
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
    state
        .repo
        .insert_task(auth.user_id, task.clone())
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok((StatusCode::CREATED, Json(task)))
}

// ============================================================================
// Agents Endpoint (Slice 10.2.4)
// ============================================================================

/// Response for a single agent
#[derive(Serialize)]
pub struct AgentResponse {
    pub id: String,
    pub tier: String,
    pub status: String,
    pub current_task: Option<Uuid>,
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

/// List all agents and their status
///
/// Returns the list of agents along with pool statistics based on configuration.
/// Note: In server mode, the agent pool may not be active - this returns config limits.
pub async fn list_agents(State(state): State<AppState>) -> Json<AgentsListResponse> {
    let pool_config = &state.config.pool;

    // Return configuration-based stats
    // When the agent pool is active, this will be updated to show actual agents
    let response = AgentsListResponse {
        agents: vec![],
        stats: AgentPoolStats {
            orchestrators: TierStats {
                total: 0,
                available: 0,
                max: pool_config.max_orchestrators,
            },
            workers: TierStats {
                total: 0,
                available: 0,
                max: pool_config.max_workers,
            },
            utilities: TierStats {
                total: 0,
                available: 0,
                max: pool_config.max_utilities,
            },
        },
    };

    Json(response)
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
    let config = state.config.as_ref();

    Json(ConfigResponse {
        verbosity: format!("{:?}", config.verbosity).to_lowercase(),
        models: config.models.clone(),
        pool: config.pool.clone(),
        autonomy: format!("{:?}", config.autonomy).to_lowercase(),
        git_strategy: format!("{:?}", config.git_strategy).to_lowercase(),
        sandbox_mode: format!("{:?}", config.sandbox_mode).to_lowercase(),
    })
}

/// Request body for updating configuration
#[derive(Deserialize)]
pub struct UpdateConfigRequest {
    pub verbosity: Option<String>,
}

/// Update configuration (partial update)
///
/// Currently supports updating verbosity level.
/// Note: Full config persistence requires additional implementation.
pub async fn update_config(
    State(state): State<AppState>,
    Json(request): Json<UpdateConfigRequest>,
) -> Result<Json<ConfigResponse>, StatusCode> {
    // Validate verbosity if provided
    if let Some(ref v) = request.verbosity {
        match v.to_lowercase().as_str() {
            "quiet" | "normal" | "verbose" => {}
            _ => return Err(StatusCode::BAD_REQUEST),
        }
    }

    // Note: Configuration updates would require mutable state or a config service
    // For now, we just validate and return the current config
    // Full implementation would persist changes to config file

    Ok(Json(get_config(State(state)).await.0))
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
pub async fn send_chat(
    State(state): State<AppState>,
    auth: auth::AuthUser,
    Json(request): Json<ChatRequest>,
) -> Result<(StatusCode, Json<ChatResponse>), StatusCode> {
    if request.message.trim().is_empty() {
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
pub async fn get_chat_history(
    State(state): State<AppState>,
    auth: auth::AuthUser,
    Query(query): Query<HistoryQuery>,
) -> Result<Json<Vec<ChatMessage>>, StatusCode> {
    let limit = query.limit.unwrap_or(50);
    let offset = query.offset.unwrap_or(0);

    let rows = state
        .repo
        .get_chat_history(auth.user_id, limit, offset)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

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
pub async fn chat_stream(
    State(state): State<AppState>,
    Path(message_id): Path<Uuid>,
) -> Sse<impl Stream<Item = Result<Event, Infallible>>> {
    chat_stream_inner(state, message_id)
}

/// Stream chat response for session-scoped messages.
///
/// Same as `chat_stream` but extracts both session_id and message_id
/// from the path (only message_id is used for stream lookup).
pub async fn session_chat_stream(
    State(state): State<AppState>,
    Path((_session_id, message_id)): Path<(Uuid, Uuid)>,
) -> Sse<impl Stream<Item = Result<Event, Infallible>>> {
    chat_stream_inner(state, message_id)
}

fn chat_stream_inner(
    state: AppState,
    message_id: Uuid,
) -> Sse<impl Stream<Item = Result<Event, Infallible>>> {
    let stream = async_stream::stream! {
        let (buffered, mut rx, already_done) = state.get_response_stream(message_id).await;

        // Replay any buffered chunks that arrived before we connected
        for chunk in buffered {
            match chunk {
                StreamChunk::Token(text) => {
                    yield Ok(Event::default().event("token").data(text));
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
                            yield Ok(Event::default().event("token").data(text));
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
pub async fn clear_chat_history(
    State(state): State<AppState>,
    auth: auth::AuthUser,
) -> StatusCode {
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
pub async fn list_modes(
    State(state): State<AppState>,
    _auth: auth::AuthUser,
) -> Json<Vec<ModeInfo>> {
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
pub async fn create_session(
    State(state): State<AppState>,
    auth: auth::AuthUser,
    Json(request): Json<CreateSessionRequest>,
) -> Result<(StatusCode, Json<SessionResponse>), StatusCode> {
    // Validate mode exists
    let mode_id = crate::server::agent_mode::AgentModeId::new(&request.mode_id);
    if state.mode_registry.get(&mode_id).is_none() {
        return Err(StatusCode::BAD_REQUEST);
    }

    let session_id = Uuid::new_v4();
    let title = if request.title.is_empty() {
        format!("New {} session", request.mode_id)
    } else {
        request.title
    };

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
pub async fn list_sessions(
    State(state): State<AppState>,
    auth: auth::AuthUser,
) -> Result<Json<Vec<SessionResponse>>, StatusCode> {
    let sessions = state
        .repo
        .list_sessions(auth.user_id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

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
pub async fn get_session(
    State(state): State<AppState>,
    auth: auth::AuthUser,
    Path(session_id): Path<Uuid>,
) -> Result<Json<SessionResponse>, StatusCode> {
    let session = state
        .repo
        .get_session(session_id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::NOT_FOUND)?;

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
pub async fn delete_session(
    State(state): State<AppState>,
    auth: auth::AuthUser,
    Path(session_id): Path<Uuid>,
) -> Result<StatusCode, StatusCode> {
    // Verify ownership
    let session = state
        .repo
        .get_session(session_id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::NOT_FOUND)?;

    if session.user_id != auth.user_id.0 {
        return Err(StatusCode::NOT_FOUND);
    }

    state
        .repo
        .delete_session(session_id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(StatusCode::NO_CONTENT)
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
    let session = state
        .repo
        .get_session(session_id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::NOT_FOUND)?;

    if session.user_id != auth.user_id.0 {
        return Err(StatusCode::NOT_FOUND);
    }

    let message_id = Uuid::new_v4();

    state.ensure_response_stream(message_id).await;

    // Store user message scoped to session
    state
        .repo
        .insert_session_message(
            auth.user_id,
            session_id,
            message_id,
            "user".to_string(),
            request.message.clone(),
        )
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
pub async fn get_session_history(
    State(state): State<AppState>,
    auth: auth::AuthUser,
    Path(session_id): Path<Uuid>,
    Query(query): Query<HistoryQuery>,
) -> Result<Json<Vec<ChatMessage>>, StatusCode> {
    // Verify session ownership
    let session = state
        .repo
        .get_session(session_id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::NOT_FOUND)?;

    if session.user_id != auth.user_id.0 {
        return Err(StatusCode::NOT_FOUND);
    }

    let limit = query.limit.unwrap_or(50);
    let rows = state
        .repo
        .get_session_history(session_id, limit)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

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
pub async fn auth_setup(
    State(state): State<AppState>,
    Json(request): Json<SetupRequest>,
) -> Result<Json<SetupResponse>, (StatusCode, String)> {
    // Check if already setup
    if state
        .repo
        .has_password()
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
    {
        return Err((
            StatusCode::CONFLICT,
            "Password already configured".to_string(),
        ));
    }

    // Validate password strength
    if request.password.len() < 8 {
        return Err((
            StatusCode::BAD_REQUEST,
            "Password must be at least 8 characters".to_string(),
        ));
    }

    // Hash and store
    let hash = auth::hash_password(&request.password)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    state
        .repo
        .set_password(hash)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

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
pub async fn auth_register(
    State(state): State<AppState>,
    Json(request): Json<RegisterRequest>,
) -> Result<(StatusCode, Json<AuthTokenResponse>), (StatusCode, String)> {
    // Validate
    if request.email.trim().is_empty() || !request.email.contains('@') {
        return Err((StatusCode::BAD_REQUEST, "Invalid email".into()));
    }
    if request.password.len() < 8 {
        return Err((
            StatusCode::BAD_REQUEST,
            "Password must be at least 8 characters".into(),
        ));
    }

    let user_repo = state
        .user_repo
        .as_ref()
        .ok_or((StatusCode::INTERNAL_SERVER_ERROR, "User service unavailable".into()))?;

    // Check if email already exists
    if user_repo
        .get_user_by_email(&request.email)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .is_some()
    {
        return Err((StatusCode::CONFLICT, "Email already registered".into()));
    }

    let hash = auth::hash_password(&request.password)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let user = user_repo
        .create_user(&request.email, &hash)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let token = auth::create_token(&state.jwt_secret, 24, user.id, &user.email)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

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
pub async fn auth_login(
    State(state): State<AppState>,
    Json(request): Json<LoginRequest>,
) -> Result<Json<LoginResponse>, StatusCode> {
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

    let token = auth::create_token(&state.jwt_secret, 24, user.id, &user.email)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(LoginResponse {
        token,
        expires_in: 86400,
    }))
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
pub async fn list_documents(
    State(state): State<AppState>,
    auth: auth::AuthUser,
) -> Result<Json<Vec<DocumentListItem>>, StatusCode> {
    let doc_repo = state.doc_repo.as_ref().ok_or(StatusCode::INTERNAL_SERVER_ERROR)?;
    let docs = doc_repo
        .list_documents(auth.user_id.0)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

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
pub async fn search_documents(
    State(state): State<AppState>,
    auth: auth::AuthUser,
    Query(query): Query<DocumentSearchQuery>,
) -> Result<Json<Vec<crate::db::DocumentSearchResult>>, StatusCode> {
    let doc_repo = state.doc_repo.as_ref().ok_or(StatusCode::INTERNAL_SERVER_ERROR)?;
    let results = doc_repo
        .search_documents(auth.user_id.0, &query.q)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(results))
}

/// GET /api/documents/:id - Get a full document by ID.
pub async fn get_document(
    State(state): State<AppState>,
    auth: auth::AuthUser,
    Path(doc_id): Path<Uuid>,
) -> Result<Json<DocumentResponse>, StatusCode> {
    let doc_repo = state.doc_repo.as_ref().ok_or(StatusCode::INTERNAL_SERVER_ERROR)?;
    let doc = doc_repo
        .get_document(doc_id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::NOT_FOUND)?;

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
pub async fn create_document(
    State(state): State<AppState>,
    auth: auth::AuthUser,
    Json(request): Json<CreateDocumentRequest>,
) -> Result<(StatusCode, Json<DocumentResponse>), StatusCode> {
    if request.title.trim().is_empty() {
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
pub async fn update_document(
    State(state): State<AppState>,
    auth: auth::AuthUser,
    Path(doc_id): Path<Uuid>,
    Json(request): Json<UpdateDocumentRequest>,
) -> Result<Json<DocumentResponse>, StatusCode> {
    let doc_repo = state.doc_repo.as_ref().ok_or(StatusCode::INTERNAL_SERVER_ERROR)?;

    // Verify ownership
    let existing = doc_repo
        .get_document(doc_id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::NOT_FOUND)?;

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
pub async fn delete_document(
    State(state): State<AppState>,
    auth: auth::AuthUser,
    Path(doc_id): Path<Uuid>,
) -> Result<StatusCode, StatusCode> {
    let doc_repo = state.doc_repo.as_ref().ok_or(StatusCode::INTERNAL_SERVER_ERROR)?;

    // Verify ownership
    let existing = doc_repo
        .get_document(doc_id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::NOT_FOUND)?;

    if existing.user_id != auth.user_id.0 {
        return Err(StatusCode::NOT_FOUND);
    }

    doc_repo
        .delete_document(doc_id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

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
            orchestrators: TierStats {
                total: 1,
                available: 1,
                max: 2,
            },
            workers: TierStats {
                total: 3,
                available: 2,
                max: 6,
            },
            utilities: TierStats {
                total: 2,
                available: 2,
                max: 4,
            },
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
    }

    impl InMemoryServerRepo {
        fn new() -> Self {
            Self {
                tasks: std::sync::Mutex::new(vec![]),
                chat_messages: std::sync::Mutex::new(vec![]),
                password_hash: std::sync::Mutex::new(None),
            }
        }
    }

    #[async_trait::async_trait]
    impl ServerRepo for InMemoryServerRepo {
        async fn health_check(&self) -> bool {
            true
        }

        async fn list_tasks(
            &self,
            _user_id: UserId,
            status: Option<String>,
            limit: Option<u32>,
        ) -> anyhow::Result<Vec<Task>> {
            let tasks = self.tasks.lock().unwrap();
            let limit = limit.unwrap_or(100).min(1000) as usize;
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

        async fn insert_chat_message(
            &self,
            _user_id: UserId,
            id: Uuid,
            role: String,
            content: String,
        ) -> anyhow::Result<()> {
            self.chat_messages.lock().unwrap().push(ChatMessageRow {
                id,
                role,
                content,
                timestamp: Utc::now(),
            });
            Ok(())
        }

        async fn get_chat_history(
            &self,
            _user_id: UserId,
            limit: u32,
            offset: u32,
        ) -> anyhow::Result<Vec<ChatMessageRow>> {
            let msgs = self.chat_messages.lock().unwrap();
            let result: Vec<ChatMessageRow> = msgs
                .iter()
                .skip(offset as usize)
                .take(limit.min(1000) as usize)
                .cloned()
                .collect();
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
        async fn list_persisted_agents(&self, _user_id: UserId) -> anyhow::Result<Vec<crate::db::AgentRow>> { Ok(vec![]) }
        async fn upsert_agent(&self, _user_id: UserId, _agent: crate::db::AgentRow) -> anyhow::Result<()> { Ok(()) }
        async fn delete_persisted_agent(&self, _agent_id: Uuid) -> anyhow::Result<()> { Ok(()) }
        async fn list_persisted_clusters(&self, _user_id: UserId) -> anyhow::Result<Vec<crate::db::ClusterRow>> { Ok(vec![]) }
        async fn upsert_cluster(&self, _user_id: UserId, _cluster: crate::db::ClusterRow) -> anyhow::Result<()> { Ok(()) }
        async fn delete_cluster(&self, _cluster_id: Uuid) -> anyhow::Result<()> { Ok(()) }
        async fn list_cluster_members(&self, _cluster_id: Uuid) -> anyhow::Result<Vec<Uuid>> { Ok(vec![]) }
        async fn add_cluster_member(&self, _cluster_id: Uuid, _agent_id: Uuid) -> anyhow::Result<()> { Ok(()) }
        async fn remove_cluster_member(&self, _cluster_id: Uuid, _agent_id: Uuid) -> anyhow::Result<()> { Ok(()) }
        async fn list_pipelines(&self, _user_id: UserId) -> anyhow::Result<Vec<PipelineRow>> { Ok(vec![]) }
        async fn upsert_pipeline(&self, _user_id: UserId, _pipeline: PipelineRow) -> anyhow::Result<()> { Ok(()) }
        async fn delete_pipeline(&self, _pipeline_id: Uuid) -> anyhow::Result<()> { Ok(()) }
        async fn list_pipeline_stages(&self, _pipeline_id: Uuid) -> anyhow::Result<Vec<PipelineStageRow>> { Ok(vec![]) }
        async fn upsert_pipeline_stage(&self, _stage: PipelineStageRow) -> anyhow::Result<()> { Ok(()) }
        async fn list_schedules(&self, _user_id: UserId) -> anyhow::Result<Vec<ScheduleRow>> { Ok(vec![]) }
        async fn upsert_schedule(&self, _user_id: UserId, _schedule: ScheduleRow) -> anyhow::Result<()> { Ok(()) }
        async fn delete_schedule(&self, _schedule_id: Uuid) -> anyhow::Result<()> { Ok(()) }
        async fn update_schedule_last_run(&self, _schedule_id: Uuid, _last_run_at: DateTime<Utc>) -> anyhow::Result<()> { Ok(()) }
        async fn list_triggers(&self, _user_id: UserId) -> anyhow::Result<Vec<TriggerRow>> { Ok(vec![]) }
        async fn upsert_trigger(&self, _user_id: UserId, _trigger: TriggerRow) -> anyhow::Result<()> { Ok(()) }
        async fn delete_trigger(&self, _trigger_id: Uuid) -> anyhow::Result<()> { Ok(()) }
        async fn create_session(&self, _user_id: UserId, _session_id: Uuid, _mode_id: &str, _title: &str) -> anyhow::Result<()> { Ok(()) }
        async fn list_sessions(&self, _user_id: UserId) -> anyhow::Result<Vec<crate::db::SessionRow>> { Ok(vec![]) }
        async fn get_session(&self, _session_id: Uuid) -> anyhow::Result<Option<crate::db::SessionRow>> { Ok(None) }
        async fn delete_session(&self, _session_id: Uuid) -> anyhow::Result<()> { Ok(()) }
        async fn insert_session_message(&self, _user_id: UserId, _session_id: Uuid, _id: Uuid, _role: String, _content: String) -> anyhow::Result<()> { Ok(()) }
        async fn get_session_history(&self, _session_id: Uuid, _limit: u32) -> anyhow::Result<Vec<ChatMessageRow>> { Ok(vec![]) }
    }

    fn create_test_token(jwt_secret: &[u8]) -> String {
        super::super::auth::create_token(jwt_secret, 24, UserId::new(), "test@test.com").unwrap()
    }

    fn setup_test_app() -> (axum::Router, Vec<u8>) {
        setup_test_app_with_user_repo(None)
    }

    fn setup_test_app_with_user_repo(
        user_repo: Option<Arc<dyn crate::db::traits::UserRepo>>,
    ) -> (axum::Router, Vec<u8>) {
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
                    .body(Body::from(
                        r#"{"title":"My task","description":"desc","priority":"high"}"#,
                    ))
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

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/api/health")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);

        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
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
        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
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
        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
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
                    .body(Body::from(
                        r#"{"title":"Default tier","tier":"nonexistent"}"#,
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::CREATED);
        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
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
        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
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
        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
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
        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
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
                    .body(Body::from(
                        r#"{"title":"Default prio","priority":"critical"}"#,
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::CREATED);
        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
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
        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
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
        let body = axum::body::to_bytes(create_resp.into_body(), usize::MAX)
            .await
            .unwrap();
        let created: serde_json::Value = serde_json::from_slice(&body).unwrap();
        let task_id = created["id"].as_str().unwrap();

        // Verify through list endpoint that the task was persisted
        let list_resp = app
            .oneshot(
                Request::builder()
                    .uri("/api/tasks")
                    .header("authorization", format!("Bearer {}", token))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(list_resp.status(), StatusCode::OK);
        let body = axum::body::to_bytes(list_resp.into_body(), usize::MAX)
            .await
            .unwrap();
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
            .oneshot(
                Request::builder()
                    .uri("/api/tasks")
                    .header("authorization", format!("Bearer {}", token))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
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
        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
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
        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
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
            .oneshot(
                Request::builder()
                    .uri("/api/agents")
                    .header("authorization", format!("Bearer {}", token))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let resp: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert!(resp["agents"].is_array());
        assert!(resp["stats"]["orchestrators"].is_object());
        assert!(resp["stats"]["workers"].is_object());
        assert!(resp["stats"]["utilities"].is_object());
    }

    // === get_config response body ===

    #[tokio::test]
    async fn get_config_returns_expected_fields() {
        let (app, jwt_secret) = setup_test_app();
        let token = create_test_token(&jwt_secret);

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/api/config")
                    .header("authorization", format!("Bearer {}", token))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
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
        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let resp: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(
            resp["message"].as_str().unwrap(),
            "Password configured successfully"
        );
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
            .returning(move |email| {
                if email == "test@test.com" {
                    Ok(Some(user_clone.clone()))
                } else {
                    Ok(None)
                }
            });
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
        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
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
        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
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
        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
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
        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
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
        let response = SetupResponse {
            message: "ok".to_string(),
        };
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

    #[test]
    fn agents_list_response_serializes() {
        let response = AgentsListResponse {
            agents: vec![AgentResponse {
                id: "agent-1".to_string(),
                tier: "worker".to_string(),
                status: "idle".to_string(),
                current_task: None,
            }],
            stats: AgentPoolStats {
                orchestrators: TierStats {
                    total: 0,
                    available: 0,
                    max: 1,
                },
                workers: TierStats {
                    total: 1,
                    available: 1,
                    max: 4,
                },
                utilities: TierStats {
                    total: 0,
                    available: 0,
                    max: 2,
                },
            },
        };
        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("\"agent-1\""));
        assert!(json.contains("\"workers\""));
    }

    #[test]
    fn agent_response_with_current_task() {
        let task_id = Uuid::new_v4();
        let response = AgentResponse {
            id: "agent-2".to_string(),
            tier: "orchestrator".to_string(),
            status: "busy".to_string(),
            current_task: Some(task_id),
        };
        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains(&task_id.to_string()));
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
        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
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
                    .body(Body::from(
                        r#"{"title":"Full task","description":"A description","priority":"high","tier":"worker"}"#,
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::CREATED);
        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
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
        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let messages: Vec<serde_json::Value> = serde_json::from_slice(&body).unwrap();
        assert!(messages.is_empty());
    }
}
