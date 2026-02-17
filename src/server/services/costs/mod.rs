//! Cost service: spend tracking and model breakdown.

use chrono::{DateTime, Utc};
use uuid::Uuid;

use crate::db::traits::{ModelSpendRow, TokenLedgerRepo};

use super::error::ServiceError;

#[cfg(test)]
mod tests;

// ============================================================================
// Types
// ============================================================================

pub struct CostBreakdown {
    pub total_spend: f64,
    pub models: Vec<ModelSpendRow>,
}

// ============================================================================
// Service functions
// ============================================================================

pub async fn get_costs(
    repo: &dyn TokenLedgerRepo,
    user_id: Uuid,
    since: Option<DateTime<Utc>>,
) -> Result<CostBreakdown, ServiceError> {
    let total_spend = repo.get_user_spend(user_id, since).await?;
    let models = repo.get_model_breakdown(user_id, since).await?;
    Ok(CostBreakdown {
        total_spend,
        models,
    })
}
