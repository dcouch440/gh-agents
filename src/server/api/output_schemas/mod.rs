//! Output schema management endpoints

use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::AppError;
use crate::server::auth as auth_utils;
use crate::server::services::output_schemas;
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

fn schema_response(row: crate::db::OutputSchemaRow) -> OutputSchemaResponse {
    OutputSchemaResponse {
        id: row.id,
        name: row.name,
        schema: row.schema,
        created_at: row.created_at,
    }
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
) -> Result<Json<Vec<OutputSchemaResponse>>, AppError> {
    let rows =
        output_schemas::list_output_schemas(state.repos().output_schemas.as_ref(), auth.user_id.0)
            .await?;
    Ok(Json(rows.into_iter().map(schema_response).collect()))
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
) -> Result<(StatusCode, Json<OutputSchemaResponse>), AppError> {
    let row = output_schemas::create_output_schema(
        state.repos().output_schemas.as_ref(),
        auth.user_id.0,
        request.name,
        request.schema,
    )
    .await?;
    Ok((StatusCode::CREATED, Json(schema_response(row))))
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
) -> Result<Json<OutputSchemaResponse>, AppError> {
    let row = output_schemas::get_output_schema(
        state.repos().output_schemas.as_ref(),
        auth.user_id.0,
        id,
    )
    .await?;
    Ok(Json(schema_response(row)))
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
) -> Result<Json<OutputSchemaResponse>, AppError> {
    let row = output_schemas::update_output_schema(
        state.repos().output_schemas.as_ref(),
        auth.user_id.0,
        id,
        request.name,
        request.schema,
    )
    .await?;
    Ok(Json(schema_response(row)))
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
) -> Result<StatusCode, AppError> {
    output_schemas::delete_output_schema(state.repos().output_schemas.as_ref(), auth.user_id.0, id)
        .await?;
    Ok(StatusCode::NO_CONTENT)
}

#[cfg(test)]
mod tests;
