//! Output schema management endpoints

use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::constants::MAX_TITLE_LENGTH;
use crate::server::auth as auth_utils;
use crate::server::state::AppState;

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
pub async fn list_output_schemas(
    State(state): State<AppState>,
    auth: auth_utils::AuthUser,
) -> Result<Json<Vec<OutputSchemaResponse>>, StatusCode> {
    let repo = state
        .output_schema_repo()
        .ok_or(StatusCode::INTERNAL_SERVER_ERROR)?;
    let rows = repo
        .list_output_schemas(auth.user_id.0)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
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
pub async fn create_output_schema(
    State(state): State<AppState>,
    auth: auth_utils::AuthUser,
    Json(request): Json<CreateOutputSchemaRequest>,
) -> Result<(StatusCode, Json<OutputSchemaResponse>), StatusCode> {
    if request.name.trim().is_empty() || request.name.len() > MAX_TITLE_LENGTH {
        return Err(StatusCode::BAD_REQUEST);
    }
    let repo = state
        .output_schema_repo()
        .ok_or(StatusCode::INTERNAL_SERVER_ERROR)?;
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
pub async fn get_output_schema(
    State(state): State<AppState>,
    auth: auth_utils::AuthUser,
    Path(id): Path<Uuid>,
) -> Result<Json<OutputSchemaResponse>, StatusCode> {
    let repo = state
        .output_schema_repo()
        .ok_or(StatusCode::INTERNAL_SERVER_ERROR)?;
    let row = repo
        .get_output_schema(id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::NOT_FOUND)?;
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
    auth: auth_utils::AuthUser,
    Path(id): Path<Uuid>,
    Json(request): Json<UpdateOutputSchemaRequest>,
) -> Result<Json<OutputSchemaResponse>, StatusCode> {
    let repo = state
        .output_schema_repo()
        .ok_or(StatusCode::INTERNAL_SERVER_ERROR)?;
    let existing = repo
        .get_output_schema(id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::NOT_FOUND)?;
    if existing.user_id != auth.user_id.0 {
        return Err(StatusCode::NOT_FOUND);
    }
    if let Some(ref name) = request.name {
        if name.trim().is_empty() || name.len() > MAX_TITLE_LENGTH {
            return Err(StatusCode::BAD_REQUEST);
        }
    }
    let row = repo
        .update_output_schema(id, request.name, request.schema)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
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
pub async fn delete_output_schema(
    State(state): State<AppState>,
    auth: auth_utils::AuthUser,
    Path(id): Path<Uuid>,
) -> Result<StatusCode, StatusCode> {
    let repo = state
        .output_schema_repo()
        .ok_or(StatusCode::INTERNAL_SERVER_ERROR)?;
    let existing = repo
        .get_output_schema(id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::NOT_FOUND)?;
    if existing.user_id != auth.user_id.0 {
        return Err(StatusCode::NOT_FOUND);
    }
    repo.delete_output_schema(id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(StatusCode::NO_CONTENT)
}

#[cfg(test)]
mod tests;
