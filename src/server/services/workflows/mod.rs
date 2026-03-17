//! Workflow service: create, read, update, delete workflows.

use uuid::Uuid;

use crate::db::traits::{CreateWorkflowInput, UpdateWorkflowInput, WorkflowRepo};
use crate::db::WorkflowRow;

use super::error::ServiceError;
use super::validation;

/// Verify the caller owns this workflow.
pub(crate) async fn verify_workflow_ownership(
    repo: &dyn WorkflowRepo,
    user_id: Uuid,
    workflow_id: Uuid,
) -> Result<WorkflowRow, ServiceError> {
    super::ownership::fetch_and_check_owner(
        || repo.get_workflow(workflow_id),
        user_id,
        |w| w.user_id,
        "Workflow",
    )
    .await
}

/// Create a new workflow.
pub async fn create_workflow(
    repo: &dyn WorkflowRepo,
    input: CreateWorkflowInput,
) -> Result<WorkflowRow, ServiceError> {
    validation::validate_name(&input.name, "Workflow name")?;
    let row = repo.create_workflow(input).await?;
    Ok(row)
}

/// Get a workflow by ID, verifying ownership.
pub async fn get_workflow(
    repo: &dyn WorkflowRepo,
    user_id: Uuid,
    workflow_id: Uuid,
) -> Result<WorkflowRow, ServiceError> {
    verify_workflow_ownership(repo, user_id, workflow_id).await
}

/// List workflows for a user.
pub async fn list_workflows(
    repo: &dyn WorkflowRepo,
    user_id: Uuid,
) -> Result<Vec<WorkflowRow>, ServiceError> {
    let rows = repo.list_workflows(user_id).await?;
    Ok(rows)
}

/// Update a workflow (partial update), verifying ownership.
pub async fn update_workflow(
    repo: &dyn WorkflowRepo,
    user_id: Uuid,
    workflow_id: Uuid,
    input: UpdateWorkflowInput,
) -> Result<WorkflowRow, ServiceError> {
    verify_workflow_ownership(repo, user_id, workflow_id).await?;

    if let Some(ref n) = input.name {
        validation::validate_name(n, "Workflow name")?;
    }

    let row = repo.update_workflow(input).await?;
    Ok(row)
}

/// Delete a workflow by ID, verifying ownership.
pub async fn delete_workflow(
    repo: &dyn WorkflowRepo,
    user_id: Uuid,
    workflow_id: Uuid,
) -> Result<(), ServiceError> {
    verify_workflow_ownership(repo, user_id, workflow_id).await?;
    repo.delete_workflow(workflow_id).await?;
    Ok(())
}

mod tests;
