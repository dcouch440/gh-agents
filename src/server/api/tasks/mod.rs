//! Task management endpoints

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    Json,
};
use chrono::Utc;
use serde::Deserialize;
use uuid::Uuid;

use crate::server::auth as auth_utils;
use crate::server::state::AppState;
use crate::types::{Priority, Task};

use super::{MAX_DESCRIPTION_LENGTH, MAX_TITLE_LENGTH};

/// Query parameters for listing tasks
#[derive(Deserialize, utoipa::ToSchema, utoipa::IntoParams)]
pub struct TasksQuery {
    pub status: Option<String>,
    pub limit: Option<u32>,
}

/// Request body for creating a new task
#[derive(Deserialize, utoipa::ToSchema)]
pub struct CreateTaskRequest {
    pub title: String,
    pub description: Option<String>,
    pub priority: Option<String>,
    pub tier: Option<String>,
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
pub async fn list_tasks(
    State(state): State<AppState>,
    auth: auth_utils::AuthUser,
    Query(query): Query<TasksQuery>,
) -> Result<Json<Vec<Task>>, StatusCode> {
    let tasks = state
        .repo()
        .list_tasks(auth.user_id, query.status, query.limit)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

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
pub async fn get_task(
    State(state): State<AppState>,
    auth: auth_utils::AuthUser,
    Path(id): Path<Uuid>,
) -> Result<Json<Task>, StatusCode> {
    let task = state
        .repo()
        .get_task_by_uuid(auth.user_id, id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::NOT_FOUND)?;

    Ok(Json(task))
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
pub async fn create_task(
    State(state): State<AppState>,
    auth: auth_utils::AuthUser,
    Json(request): Json<CreateTaskRequest>,
) -> Result<(StatusCode, Json<Task>), StatusCode> {
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
    state
        .repo()
        .insert_task(auth.user_id, task.clone())
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok((StatusCode::CREATED, Json(task)))
}
#[cfg(test)]
mod tests;
