//! Structured output result storage and retrieval endpoints

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    Json,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::server::auth as auth_utils;
use crate::server::state::AppState;

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
pub async fn list_results(
    State(state): State<AppState>,
    auth: auth_utils::AuthUser,
    Query(q): Query<ResultQuery>,
) -> Result<Json<Vec<ResultResponse>>, StatusCode> {
    let repo = state
        .result_repo()
        .ok_or(StatusCode::INTERNAL_SERVER_ERROR)?;
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
pub async fn get_result(
    State(state): State<AppState>,
    auth: auth_utils::AuthUser,
    Path(id): Path<Uuid>,
) -> Result<Json<ResultResponse>, StatusCode> {
    let repo = state
        .result_repo()
        .ok_or(StatusCode::INTERNAL_SERVER_ERROR)?;
    let row = repo
        .get_result(id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::NOT_FOUND)?;
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
pub async fn delete_result(
    State(state): State<AppState>,
    auth: auth_utils::AuthUser,
    Path(id): Path<Uuid>,
) -> Result<StatusCode, StatusCode> {
    let repo = state
        .result_repo()
        .ok_or(StatusCode::INTERNAL_SERVER_ERROR)?;
    let row = repo
        .get_result(id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::NOT_FOUND)?;
    if row.user_id != auth.user_id.0 {
        return Err(StatusCode::NOT_FOUND);
    }
    repo.delete_result(id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(StatusCode::NO_CONTENT)
}

#[cfg(test)]
mod tests;
