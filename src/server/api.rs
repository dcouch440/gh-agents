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

use super::state::{AppState, OrchestratorMessage, StreamChunk};
use crate::db;
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
    // Check database connectivity with a simple query
    let db_connected = sqlx::query("SELECT 1").fetch_one(&state.db).await.is_ok();

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
    Query(query): Query<TasksQuery>,
) -> Result<Json<Vec<Task>>, StatusCode> {
    let tasks = db::list_tasks(&state.db, query.status.as_deref(), query.limit)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(tasks))
}

/// Get a single task by ID
///
/// Returns 404 if the task is not found.
pub async fn get_task(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<Task>, StatusCode> {
    let task = db::get_task_by_uuid(&state.db, id)
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
    db::insert_task(&state.db, &task)
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
    Json(request): Json<ChatRequest>,
) -> Result<(StatusCode, Json<ChatResponse>), StatusCode> {
    if request.message.trim().is_empty() {
        return Err(StatusCode::BAD_REQUEST);
    }

    let message_id = Uuid::new_v4();

    // Store the user message in the database
    db::insert_chat_message(&state.db, &message_id, "user", &request.message)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    // Queue message to orchestrator
    state
        .orchestrator_tx
        .send(OrchestratorMessage {
            id: message_id,
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
    Query(query): Query<HistoryQuery>,
) -> Result<Json<Vec<ChatMessage>>, StatusCode> {
    let limit = query.limit.unwrap_or(50);
    let offset = query.offset.unwrap_or(0);

    let rows = db::get_chat_history(&state.db, limit, offset)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let messages: Vec<ChatMessage> = rows
        .into_iter()
        .filter_map(|row| {
            let id = Uuid::parse_str(&row.id).ok()?;
            let timestamp = chrono::DateTime::parse_from_rfc3339(&row.timestamp)
                .ok()?
                .with_timezone(&Utc);
            Some(ChatMessage {
                id,
                role: row.role,
                content: row.content,
                timestamp,
            })
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
    let stream = async_stream::stream! {
        // Subscribe to response stream for this message
        let mut rx = state.get_response_stream(message_id).await;

        loop {
            match rx.recv().await {
                Ok(chunk) => {
                    match chunk {
                        StreamChunk::Token(text) => {
                            yield Ok(Event::default().data(text));
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
                    // Channel closed, end stream
                    break;
                }
                Err(broadcast::error::RecvError::Lagged(_)) => {
                    // We lagged behind, continue receiving
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
pub async fn clear_chat_history(State(state): State<AppState>) -> StatusCode {
    match db::clear_chat_history(&state.db).await {
        Ok(_) => StatusCode::NO_CONTENT,
        Err(_) => StatusCode::INTERNAL_SERVER_ERROR,
    }
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
}
