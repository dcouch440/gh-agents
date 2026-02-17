//! Output schema service: create, read, update, delete output schemas.

use uuid::Uuid;

use crate::db::traits::OutputSchemaRepo;
use crate::db::OutputSchemaRow;

use super::error::ServiceError;
use super::validation;

/// Verify strict ownership: the schema must exist AND have a `user_id` that
/// matches the caller. System schemas (`user_id = None`) are NOT editable, so
/// they fail this check.
async fn verify_ownership(
    repo: &dyn OutputSchemaRepo,
    user_id: Uuid,
    schema_id: Uuid,
) -> Result<OutputSchemaRow, ServiceError> {
    let schema = repo
        .get_output_schema(schema_id)
        .await?
        .ok_or_else(|| ServiceError::not_found("Output schema"))?;
    match schema.user_id {
        Some(owner) if owner != user_id => return Err(ServiceError::not_found("Output schema")),
        None => return Err(ServiceError::not_found("Output schema")),
        _ => {}
    }
    Ok(schema)
}

/// Create a new output schema owned by the given user.
pub async fn create_output_schema(
    repo: &dyn OutputSchemaRepo,
    user_id: Uuid,
    name: String,
    schema: serde_json::Value,
) -> Result<OutputSchemaRow, ServiceError> {
    validation::validate_name(&name, "name")?;
    let row = repo
        .create_output_schema(Some(user_id), name, schema)
        .await?;
    Ok(row)
}

/// Get a single output schema by ID.
///
/// System schemas (`user_id = None`) are visible to all users.
/// User-owned schemas are only visible to their owner.
pub async fn get_output_schema(
    repo: &dyn OutputSchemaRepo,
    user_id: Uuid,
    schema_id: Uuid,
) -> Result<OutputSchemaRow, ServiceError> {
    let schema = repo
        .get_output_schema(schema_id)
        .await?
        .ok_or_else(|| ServiceError::not_found("Output schema"))?;
    if let Some(owner) = schema.user_id {
        if owner != user_id {
            return Err(ServiceError::not_found("Output schema"));
        }
    }
    Ok(schema)
}

/// List output schemas for a user.
pub async fn list_output_schemas(
    repo: &dyn OutputSchemaRepo,
    user_id: Uuid,
) -> Result<Vec<OutputSchemaRow>, ServiceError> {
    let rows = repo.list_output_schemas(user_id).await?;
    Ok(rows)
}

/// Update an existing output schema (partial update).
///
/// Only user-owned schemas can be updated; system schemas are read-only.
pub async fn update_output_schema(
    repo: &dyn OutputSchemaRepo,
    user_id: Uuid,
    schema_id: Uuid,
    name: Option<String>,
    schema: Option<serde_json::Value>,
) -> Result<OutputSchemaRow, ServiceError> {
    verify_ownership(repo, user_id, schema_id).await?;

    if let Some(ref n) = name {
        validation::validate_name(n, "name")?;
    }

    let row = repo.update_output_schema(schema_id, name, schema).await?;
    Ok(row)
}

/// Delete an output schema by ID.
///
/// Only user-owned schemas can be deleted; system schemas are read-only.
pub async fn delete_output_schema(
    repo: &dyn OutputSchemaRepo,
    user_id: Uuid,
    schema_id: Uuid,
) -> Result<(), ServiceError> {
    verify_ownership(repo, user_id, schema_id).await?;
    repo.delete_output_schema(schema_id).await?;
    Ok(())
}

mod tests;
