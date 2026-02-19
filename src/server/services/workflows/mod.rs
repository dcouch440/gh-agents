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
    let wf = repo
        .get_workflow(workflow_id)
        .await?
        .ok_or_else(|| ServiceError::not_found("Workflow"))?;
    super::ownership::check_direct_owner(wf.user_id, user_id, "Workflow")?;
    Ok(wf)
}

/// Create a new workflow.
pub async fn create_workflow(
    repo: &dyn WorkflowRepo,
    user_id: Uuid,
    name: String,
    description: Option<String>,
    container_enabled: Option<bool>,
    target_repo_url: Option<String>,
    target_branch: Option<String>,
    vpn_enabled: Option<bool>,
) -> Result<WorkflowRow, ServiceError> {
    validation::validate_name(&name, "Workflow name")?;

    let row = repo
        .create_workflow(CreateWorkflowInput {
            user_id,
            name,
            description: description.unwrap_or_default(),
            container_enabled: container_enabled.unwrap_or(false),
            target_repo_url,
            target_branch,
            vpn_enabled: vpn_enabled.unwrap_or(false),
        })
        .await?;
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
    name: Option<String>,
    description: Option<String>,
    container_enabled: Option<bool>,
    target_repo_url: Option<Option<String>>,
    target_branch: Option<Option<String>>,
    vpn_enabled: Option<bool>,
) -> Result<WorkflowRow, ServiceError> {
    verify_workflow_ownership(repo, user_id, workflow_id).await?;

    if let Some(ref n) = name {
        validation::validate_name(n, "Workflow name")?;
    }

    let row = repo
        .update_workflow(UpdateWorkflowInput {
            id: workflow_id,
            name,
            description,
            container_enabled,
            target_repo_url,
            target_branch,
            vpn_enabled,
        })
        .await?;
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
