//! Collection service: create, read, update, delete workflow collections.

use uuid::Uuid;

use crate::db::traits::WorkflowCollectionRepo;
use crate::db::{CollectionRunRow, WorkflowCollectionRow};

use super::error::ServiceError;

/// Valid execution modes for a collection.
const VALID_EXECUTION_MODES: &[&str] = &["sequential", "parallel"];

/// Verify the caller owns this collection.
async fn verify_ownership(
    repo: &dyn WorkflowCollectionRepo,
    user_id: Uuid,
    collection_id: Uuid,
) -> Result<WorkflowCollectionRow, ServiceError> {
    super::ownership::fetch_and_check_owner(
        || repo.get_collection(collection_id),
        user_id,
        |r| r.user_id,
        "Collection",
    )
    .await
}

/// Validate that an execution mode is either "sequential" or "parallel".
fn validate_execution_mode(mode: &str) -> Result<(), ServiceError> {
    if !VALID_EXECUTION_MODES.contains(&mode) {
        return Err(ServiceError::validation(
            "execution_mode must be 'sequential' or 'parallel'",
        ));
    }
    Ok(())
}

/// Create a new workflow collection.
pub async fn create_collection(
    repo: &dyn WorkflowCollectionRepo,
    user_id: Uuid,
    name: String,
    description: Option<String>,
    execution_mode: String,
) -> Result<WorkflowCollectionRow, ServiceError> {
    validate_execution_mode(&execution_mode)?;
    let row = repo
        .create_collection(user_id, name, description, execution_mode)
        .await?;
    Ok(row)
}

/// Get a single collection by ID, verifying ownership.
pub async fn get_collection(
    repo: &dyn WorkflowCollectionRepo,
    user_id: Uuid,
    collection_id: Uuid,
) -> Result<WorkflowCollectionRow, ServiceError> {
    verify_ownership(repo, user_id, collection_id).await
}

/// List all collections for a user.
pub async fn list_collections(
    repo: &dyn WorkflowCollectionRepo,
    user_id: Uuid,
) -> Result<Vec<WorkflowCollectionRow>, ServiceError> {
    let rows = repo.list_collections(user_id).await?;
    Ok(rows)
}

/// Update an existing collection (partial update).
pub async fn update_collection(
    repo: &dyn WorkflowCollectionRepo,
    user_id: Uuid,
    collection_id: Uuid,
    name: Option<String>,
    description: Option<String>,
    execution_mode: Option<String>,
) -> Result<WorkflowCollectionRow, ServiceError> {
    verify_ownership(repo, user_id, collection_id).await?;

    if let Some(ref mode) = execution_mode {
        validate_execution_mode(mode)?;
    }

    let row = repo
        .update_collection(collection_id, name, description, execution_mode)
        .await?;
    Ok(row)
}

/// Delete a collection by ID, verifying ownership.
pub async fn delete_collection(
    repo: &dyn WorkflowCollectionRepo,
    user_id: Uuid,
    collection_id: Uuid,
) -> Result<(), ServiceError> {
    verify_ownership(repo, user_id, collection_id).await?;
    repo.delete_collection(collection_id).await?;
    Ok(())
}

/// Get collection run status, verifying ownership via the run's user_id.
pub async fn get_collection_run_status(
    repo: &dyn WorkflowCollectionRepo,
    user_id: Uuid,
    run_id: Uuid,
) -> Result<CollectionRunRow, ServiceError> {
    let row = repo
        .get_collection_run(run_id)
        .await?
        .ok_or_else(|| ServiceError::not_found("Collection run"))?;
    if row.user_id != user_id {
        return Err(ServiceError::not_found("Collection run"));
    }
    Ok(row)
}

mod tests;
