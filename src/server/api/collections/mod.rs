//! Workflow collection management and execution endpoints

use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use uuid::Uuid;

use crate::db::pg_repo::PgRepo;
use crate::db::traits::WorkflowCollectionRepo;
use crate::server::auth as auth_utils;
use crate::server::collection_dag_executor::CollectionDagExecutor;
use crate::server::state::AppState;

// ============================================================================
// Request/Response Types
// ============================================================================

/// Response for a single workflow collection.
#[derive(Serialize, utoipa::ToSchema)]
pub struct CollectionResponse {
    pub id: Uuid,
    pub user_id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub execution_mode: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Request body for creating a collection.
#[derive(Deserialize, utoipa::ToSchema)]
pub struct CreateCollectionRequest {
    pub name: String,
    pub description: Option<String>,
    #[serde(default = "default_execution_mode")]
    pub execution_mode: String,
}

fn default_execution_mode() -> String {
    "parallel".to_string()
}

/// Request body for updating a collection.
#[derive(Deserialize, utoipa::ToSchema)]
pub struct UpdateCollectionRequest {
    pub name: Option<String>,
    pub description: Option<String>,
    pub execution_mode: Option<String>,
}

/// Response for collection run status.
#[derive(Serialize, utoipa::ToSchema)]
pub struct CollectionRunResponse {
    pub id: Uuid,
    pub collection_id: Uuid,
    pub user_id: Uuid,
    pub status: String,
    pub started_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
    pub error: Option<String>,
}

/// Response for execution variables.
#[derive(Serialize, utoipa::ToSchema)]
pub struct ExecutionVariableResponse {
    pub id: Uuid,
    pub variable_name: String,
    pub variable_path: String,
    pub value: serde_json::Value,
    pub created_at: DateTime<Utc>,
}

// ============================================================================
// Collection CRUD Endpoints
// ============================================================================

/// GET /api/collections - List all collections for the authenticated user.
#[utoipa::path(
    get,
    path = "/api/collections",
    tag = "Collections",
    security(("bearer_auth" = [])),
    responses(
        (status = 200, description = "List of collections", body = Vec<CollectionResponse>)
    )
)]
pub async fn list_collections(
    State(state): State<AppState>,
    auth: auth_utils::AuthUser,
) -> Result<Json<Vec<CollectionResponse>>, StatusCode> {
    let db = state.db.as_ref().ok_or(StatusCode::INTERNAL_SERVER_ERROR)?;
    let repo: Arc<dyn WorkflowCollectionRepo> = Arc::new(PgRepo::new(db.clone()));

    let rows = repo
        .list_collections(auth.user_id.0)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let items = rows
        .into_iter()
        .map(|r| CollectionResponse {
            id: r.id,
            user_id: r.user_id,
            name: r.name,
            description: r.description,
            execution_mode: r.execution_mode,
            created_at: r.created_at,
            updated_at: r.updated_at,
        })
        .collect();

    Ok(Json(items))
}

/// GET /api/collections/:id - Get a collection by ID.
#[utoipa::path(
    get,
    path = "/api/collections/{id}",
    tag = "Collections",
    security(("bearer_auth" = [])),
    params(
        ("id" = Uuid, Path, description = "Collection ID")
    ),
    responses(
        (status = 200, description = "Collection details", body = CollectionResponse),
        (status = 404, description = "Collection not found")
    )
)]
pub async fn get_collection(
    State(state): State<AppState>,
    auth: auth_utils::AuthUser,
    Path(id): Path<Uuid>,
) -> Result<Json<CollectionResponse>, StatusCode> {
    let db = state.db.as_ref().ok_or(StatusCode::INTERNAL_SERVER_ERROR)?;
    let repo: Arc<dyn WorkflowCollectionRepo> = Arc::new(PgRepo::new(db.clone()));

    let row = repo
        .get_collection(id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::NOT_FOUND)?;

    // Verify ownership
    if row.user_id != auth.user_id.0 {
        return Err(StatusCode::FORBIDDEN);
    }

    Ok(Json(CollectionResponse {
        id: row.id,
        user_id: row.user_id,
        name: row.name,
        description: row.description,
        execution_mode: row.execution_mode,
        created_at: row.created_at,
        updated_at: row.updated_at,
    }))
}

/// POST /api/collections - Create a new collection.
#[utoipa::path(
    post,
    path = "/api/collections",
    tag = "Collections",
    security(("bearer_auth" = [])),
    request_body = CreateCollectionRequest,
    responses(
        (status = 201, description = "Collection created", body = CollectionResponse),
        (status = 400, description = "Invalid request")
    )
)]
pub async fn create_collection(
    State(state): State<AppState>,
    auth: auth_utils::AuthUser,
    Json(request): Json<CreateCollectionRequest>,
) -> Result<(StatusCode, Json<CollectionResponse>), StatusCode> {
    let db = state.db.as_ref().ok_or(StatusCode::INTERNAL_SERVER_ERROR)?;
    let repo: Arc<dyn WorkflowCollectionRepo> = Arc::new(PgRepo::new(db.clone()));

    // Validate execution_mode
    if request.execution_mode != "sequential" && request.execution_mode != "parallel" {
        return Err(StatusCode::BAD_REQUEST);
    }

    let row = repo
        .create_collection(
            auth.user_id.0,
            request.name,
            request.description,
            request.execution_mode,
        )
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok((
        StatusCode::CREATED,
        Json(CollectionResponse {
            id: row.id,
            user_id: row.user_id,
            name: row.name,
            description: row.description,
            execution_mode: row.execution_mode,
            created_at: row.created_at,
            updated_at: row.updated_at,
        }),
    ))
}

/// PUT /api/collections/:id - Update a collection.
#[utoipa::path(
    put,
    path = "/api/collections/{id}",
    tag = "Collections",
    security(("bearer_auth" = [])),
    params(
        ("id" = Uuid, Path, description = "Collection ID")
    ),
    request_body = UpdateCollectionRequest,
    responses(
        (status = 200, description = "Collection updated", body = CollectionResponse),
        (status = 404, description = "Collection not found"),
        (status = 400, description = "Invalid request")
    )
)]
pub async fn update_collection(
    State(state): State<AppState>,
    auth: auth_utils::AuthUser,
    Path(id): Path<Uuid>,
    Json(request): Json<UpdateCollectionRequest>,
) -> Result<Json<CollectionResponse>, StatusCode> {
    let db = state.db.as_ref().ok_or(StatusCode::INTERNAL_SERVER_ERROR)?;
    let repo: Arc<dyn WorkflowCollectionRepo> = Arc::new(PgRepo::new(db.clone()));

    // Verify ownership
    let existing = repo
        .get_collection(id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::NOT_FOUND)?;

    if existing.user_id != auth.user_id.0 {
        return Err(StatusCode::FORBIDDEN);
    }

    // Validate execution_mode if provided
    if let Some(ref mode) = request.execution_mode {
        if mode != "sequential" && mode != "parallel" {
            return Err(StatusCode::BAD_REQUEST);
        }
    }

    let row = repo
        .update_collection(id, request.name, request.description, request.execution_mode)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(CollectionResponse {
        id: row.id,
        user_id: row.user_id,
        name: row.name,
        description: row.description,
        execution_mode: row.execution_mode,
        created_at: row.created_at,
        updated_at: row.updated_at,
    }))
}

/// DELETE /api/collections/:id - Delete a collection.
#[utoipa::path(
    delete,
    path = "/api/collections/{id}",
    tag = "Collections",
    security(("bearer_auth" = [])),
    params(
        ("id" = Uuid, Path, description = "Collection ID")
    ),
    responses(
        (status = 204, description = "Collection deleted"),
        (status = 404, description = "Collection not found")
    )
)]
pub async fn delete_collection(
    State(state): State<AppState>,
    auth: auth_utils::AuthUser,
    Path(id): Path<Uuid>,
) -> Result<StatusCode, StatusCode> {
    let db = state.db.as_ref().ok_or(StatusCode::INTERNAL_SERVER_ERROR)?;
    let repo: Arc<dyn WorkflowCollectionRepo> = Arc::new(PgRepo::new(db.clone()));

    // Verify ownership
    let existing = repo
        .get_collection(id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::NOT_FOUND)?;

    if existing.user_id != auth.user_id.0 {
        return Err(StatusCode::FORBIDDEN);
    }

    repo.delete_collection(id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(StatusCode::NO_CONTENT)
}

// ============================================================================
// Execution Endpoints
// ============================================================================

/// POST /api/collections/:id/run - Execute a collection.
#[utoipa::path(
    post,
    path = "/api/collections/{id}/run",
    tag = "Collections",
    security(("bearer_auth" = [])),
    params(
        ("id" = Uuid, Path, description = "Collection ID")
    ),
    responses(
        (status = 202, description = "Collection execution started", body = CollectionRunResponse),
        (status = 404, description = "Collection not found")
    )
)]
pub async fn run_collection(
    State(state): State<AppState>,
    auth: auth_utils::AuthUser,
    Path(id): Path<Uuid>,
) -> Result<(StatusCode, Json<CollectionRunResponse>), StatusCode> {
    let db = state.db.as_ref().ok_or(StatusCode::INTERNAL_SERVER_ERROR)?;
    let repo = Arc::new(PgRepo::new(db.clone()));
    let workflow_repo = state
        .workflow_repo
        .as_ref()
        .ok_or(StatusCode::INTERNAL_SERVER_ERROR)?;

    // Verify ownership
    let collection = repo
        .get_collection(id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::NOT_FOUND)?;

    if collection.user_id != auth.user_id.0 {
        return Err(StatusCode::FORBIDDEN);
    }

    // Create executor and run collection
    let executor = CollectionDagExecutor::new(
        Arc::clone(&repo),
        Arc::clone(workflow_repo),
        Arc::new(state.clone()),
    );

    let run = executor
        .execute_collection(id, auth.user_id.0)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok((
        StatusCode::ACCEPTED,
        Json(CollectionRunResponse {
            id: run.id,
            collection_id: run.collection_id,
            user_id: run.user_id,
            status: run.status,
            started_at: run.started_at,
            completed_at: run.completed_at,
            error: run.error,
        }),
    ))
}

/// GET /api/collections/runs/:run_id/status - Get collection run status.
#[utoipa::path(
    get,
    path = "/api/collections/runs/{run_id}/status",
    tag = "Collections",
    security(("bearer_auth" = [])),
    params(
        ("run_id" = Uuid, Path, description = "Collection run ID")
    ),
    responses(
        (status = 200, description = "Collection run status", body = CollectionRunResponse),
        (status = 404, description = "Run not found")
    )
)]
pub async fn get_collection_run_status(
    State(state): State<AppState>,
    auth: auth_utils::AuthUser,
    Path(run_id): Path<Uuid>,
) -> Result<Json<CollectionRunResponse>, StatusCode> {
    let db = state.db.as_ref().ok_or(StatusCode::INTERNAL_SERVER_ERROR)?;
    let repo: Arc<dyn WorkflowCollectionRepo> = Arc::new(PgRepo::new(db.clone()));

    let row = repo
        .get_collection_run(run_id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::NOT_FOUND)?;

    // Verify ownership
    if row.user_id != auth.user_id.0 {
        return Err(StatusCode::FORBIDDEN);
    }

    Ok(Json(CollectionRunResponse {
        id: row.id,
        collection_id: row.collection_id,
        user_id: row.user_id,
        status: row.status,
        started_at: row.started_at,
        completed_at: row.completed_at,
        error: row.error,
    }))
}

/// GET /api/collections/runs/:run_id/variables - Get execution variables.
#[utoipa::path(
    get,
    path = "/api/collections/runs/{run_id}/variables",
    tag = "Collections",
    security(("bearer_auth" = [])),
    params(
        ("run_id" = Uuid, Path, description = "Collection run ID")
    ),
    responses(
        (status = 200, description = "Execution variables", body = Vec<ExecutionVariableResponse>),
        (status = 404, description = "Run not found")
    )
)]
pub async fn get_collection_variables(
    State(state): State<AppState>,
    auth: auth_utils::AuthUser,
    Path(run_id): Path<Uuid>,
) -> Result<Json<Vec<ExecutionVariableResponse>>, StatusCode> {
    let db = state.db.as_ref().ok_or(StatusCode::INTERNAL_SERVER_ERROR)?;
    let repo: Arc<dyn WorkflowCollectionRepo> = Arc::new(PgRepo::new(db.clone()));

    // Verify ownership
    let run = repo
        .get_collection_run(run_id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::NOT_FOUND)?;

    if run.user_id != auth.user_id.0 {
        return Err(StatusCode::FORBIDDEN);
    }

    let rows = repo
        .get_execution_variables(run_id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let items = rows
        .into_iter()
        .map(|r| ExecutionVariableResponse {
            id: r.id,
            variable_name: r.variable_name,
            variable_path: r.variable_path,
            value: r.value,
            created_at: r.created_at,
        })
        .collect();

    Ok(Json(items))
}
