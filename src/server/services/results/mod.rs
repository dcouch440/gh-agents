//! Result service: structured output storage with ownership verification.

use uuid::Uuid;

use crate::db::traits::ResultRepo;
use crate::db::ResultRow;

use super::error::ServiceError;

#[cfg(test)]
mod tests;

// ============================================================================
// Service functions
// ============================================================================

pub async fn list_results(
    repo: &dyn ResultRepo,
    user_id: Uuid,
    output_schema_id: Option<Uuid>,
) -> Result<Vec<ResultRow>, ServiceError> {
    let rows = match output_schema_id {
        Some(schema_id) => repo.list_results_by_schema(user_id, schema_id).await?,
        None => repo.list_results(user_id).await?,
    };
    Ok(rows)
}

pub async fn get_result(
    repo: &dyn ResultRepo,
    user_id: Uuid,
    id: Uuid,
) -> Result<ResultRow, ServiceError> {
    let row = repo
        .get_result(id)
        .await?
        .ok_or_else(|| ServiceError::not_found("Result"))?;
    if row.user_id != user_id {
        return Err(ServiceError::not_found("Result"));
    }
    Ok(row)
}

pub async fn delete_result(
    repo: &dyn ResultRepo,
    user_id: Uuid,
    id: Uuid,
) -> Result<(), ServiceError> {
    let row = repo
        .get_result(id)
        .await?
        .ok_or_else(|| ServiceError::not_found("Result"))?;
    if row.user_id != user_id {
        return Err(ServiceError::not_found("Result"));
    }
    repo.delete_result(id).await?;
    Ok(())
}
