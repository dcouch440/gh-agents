//! Pipeline destruction: remove child workflow and all its contents.

use crate::db::traits::WorkflowRepo;
use crate::server::services::ServiceError;

use super::types::PipelineContext;

/// Remove the pipeline: delete all child steps, edges, the Designer step,
/// and clear `child_workflow_id` on the parent step.
///
/// If the parent step has no pipeline, this is a no-op.
pub async fn destroy_pipeline(
    repo: &dyn WorkflowRepo,
    ctx: &PipelineContext,
) -> Result<(), ServiceError> {
    let step = repo
        .get_step(ctx.parent_step_id)
        .await
        .map_err(|e| ServiceError::Internal(e.into()))?
        .ok_or_else(|| ServiceError::not_found("Parent step"))?;

    let pipeline_id = match step.child_workflow_id {
        Some(id) => id,
        None => return Ok(()),
    };

    // Delete all steps in the child workflow (edges are cascade-deleted or
    // removed explicitly to be safe)
    let child_steps = repo
        .list_steps(pipeline_id)
        .await
        .map_err(|e| ServiceError::Internal(e.into()))?;

    let edges = repo
        .list_edges(pipeline_id)
        .await
        .map_err(|e| ServiceError::Internal(e.into()))?;

    // Remove all edges first
    for edge in &edges {
        repo.remove_edge(edge.from_step_id, edge.to_step_id)
            .await
            .map_err(|e| ServiceError::Internal(e.into()))?;
    }

    // Remove all steps (including Designer)
    for child_step in &child_steps {
        repo.delete_step(child_step.id)
            .await
            .map_err(|e| ServiceError::Internal(e.into()))?;
    }

    // Clear child_workflow_id on the parent step
    let mut updated = step;
    updated.child_workflow_id = None;
    repo.update_step(updated)
        .await
        .map_err(|e| ServiceError::Internal(e.into()))?;

    Ok(())
}
