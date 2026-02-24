//! Edge service: add, remove, delete, list workflow edges.

use uuid::Uuid;

use crate::db::traits::WorkflowRepo;
use crate::db::WorkflowStepEdgeRow;

use super::error::ServiceError;
use super::workflows::verify_workflow_ownership;

/// Add an edge between two steps, verifying ownership and that the
/// target step is not a context node.
pub async fn add_edge(
    repo: &dyn WorkflowRepo,
    user_id: Uuid,
    workflow_id: Uuid,
    from_step_id: Uuid,
    to_step_id: Uuid,
) -> Result<WorkflowStepEdgeRow, ServiceError> {
    verify_workflow_ownership(repo, user_id, workflow_id).await?;

    // Context nodes are source-only (cannot be edge targets)
    let to_step = repo
        .get_step(to_step_id)
        .await?
        .ok_or_else(|| ServiceError::not_found("Target step"))?;
    if to_step.execution_mode == "context" {
        return Err(ServiceError::validation(
            "Context nodes cannot receive incoming edges",
        ));
    }

    let edge = repo.add_edge(workflow_id, from_step_id, to_step_id).await?;
    Ok(edge)
}

/// Remove an edge by step pair, verifying ownership.
/// Returns the deleted edge row for event broadcasting.
pub async fn remove_edge(
    repo: &dyn WorkflowRepo,
    user_id: Uuid,
    workflow_id: Uuid,
    from_step_id: Uuid,
    to_step_id: Uuid,
) -> Result<WorkflowStepEdgeRow, ServiceError> {
    verify_workflow_ownership(repo, user_id, workflow_id).await?;
    let edge = repo.remove_edge(from_step_id, to_step_id).await?;
    Ok(edge)
}

/// Delete an edge by its ID, verifying workflow ownership.
/// Returns the deleted edge row for event broadcasting.
pub async fn delete_edge_by_id(
    repo: &dyn WorkflowRepo,
    user_id: Uuid,
    workflow_id: Uuid,
    edge_id: Uuid,
) -> Result<WorkflowStepEdgeRow, ServiceError> {
    verify_workflow_ownership(repo, user_id, workflow_id).await?;
    let edge = repo.delete_edge_by_id(edge_id).await?;
    Ok(edge)
}

/// List edges for a workflow, verifying ownership.
pub async fn list_edges(
    repo: &dyn WorkflowRepo,
    user_id: Uuid,
    workflow_id: Uuid,
) -> Result<Vec<WorkflowStepEdgeRow>, ServiceError> {
    verify_workflow_ownership(repo, user_id, workflow_id).await?;
    let rows = repo.list_edges(workflow_id).await?;
    Ok(rows)
}

mod tests;
