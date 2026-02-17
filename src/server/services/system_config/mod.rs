//! System config service: CRUD for system-wide configuration entries.

use uuid::Uuid;

use crate::db::traits::SystemConfigRepo;
use crate::db::SystemConfigRow;

use super::error::ServiceError;
use super::validation;

#[cfg(test)]
mod tests;

// ============================================================================
// Input types
// ============================================================================

pub struct UpsertSystemConfigInput {
    pub config_type: String,
    pub config_key: String,
    pub config_value: serde_json::Value,
    pub description: Option<String>,
    pub created_by: Option<Uuid>,
}

// ============================================================================
// Service functions
// ============================================================================

pub async fn list_system_configs(
    repo: &dyn SystemConfigRepo,
    config_type: Option<String>,
) -> Result<Vec<SystemConfigRow>, ServiceError> {
    Ok(repo.list_system_configs(config_type).await?)
}

pub async fn upsert_system_config(
    repo: &dyn SystemConfigRepo,
    input: UpsertSystemConfigInput,
) -> Result<SystemConfigRow, ServiceError> {
    validation::validate_required(&input.config_key, "config_key")?;
    validation::validate_required(&input.config_type, "config_type")?;

    Ok(repo
        .upsert_system_config(
            &input.config_type,
            &input.config_key,
            &input.config_value,
            input.description,
            input.created_by,
        )
        .await?)
}

pub async fn delete_system_config(
    repo: &dyn SystemConfigRepo,
    key: &str,
) -> Result<(), ServiceError> {
    repo.delete_system_config(key).await?;
    Ok(())
}
