//! Cost tracking and reporting endpoints

use axum::{
    extract::{Query, State},
    Json,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use super::AppError;
use crate::server::auth as auth_utils;
use crate::server::services::costs as svc;
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
pub async fn get_costs(
    State(state): State<AppState>,
    auth: auth_utils::AuthUser,
    Query(q): Query<CostQuery>,
) -> Result<Json<CostResponse>, AppError> {
    let breakdown =
        svc::get_costs(state.repos().token_ledger.as_ref(), auth.user_id.0, q.since).await?;
    Ok(Json(CostResponse {
        total_spend: breakdown.total_spend,
        models: breakdown.models,
    }))
}

#[cfg(test)]
mod tests;
