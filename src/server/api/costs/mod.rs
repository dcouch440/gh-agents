//! Cost tracking and reporting endpoints

use axum::{
    extract::{Query, State},
    http::StatusCode,
    Json,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::server::auth as auth_utils;
use crate::server::state::AppState;

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
pub async fn get_costs(State(state): State<AppState>, auth: auth_utils::AuthUser, Query(q): Query<CostQuery>) -> Result<Json<CostResponse>, StatusCode> {
    let repo = state.token_ledger_repo.as_ref().ok_or(StatusCode::INTERNAL_SERVER_ERROR)?;
    let total_spend = repo.get_user_spend(auth.user_id.0, q.since).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let models = repo.get_model_breakdown(auth.user_id.0, q.since).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Json(CostResponse { total_spend, models }))
}

#[cfg(test)]
mod tests;
