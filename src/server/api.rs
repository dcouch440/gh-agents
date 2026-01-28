//! REST API endpoint handlers

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    Json,
};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::state::AppState;
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
}
