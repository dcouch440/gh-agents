//! Remove a step from a pipeline.

use uuid::Uuid;

use crate::db::traits::WorkflowRepo;
use crate::server::services::ServiceError;

use super::recompute::recompute_execution_order;
use super::types::{ExecutionOrderEntry, PipelineContext};

/// Remove a step from the pipeline. Removes all edges touching it.
///
/// If the pipeline has no remaining steps after removal, destroys
/// the entire pipeline (child workflow and clears `child_workflow_id`
/// on the parent step).
///
/// Returns the recomputed execution sequence (empty if pipeline was destroyed).
pub async fn remove_step(
    repo: &dyn WorkflowRepo,
    ctx: &PipelineContext,
    step_id: Uuid,
) -> Result<Vec<ExecutionOrderEntry>, ServiceError> {
    let parent_step = repo
        .get_step(ctx.parent_step_id)
        .await
        .map_err(ServiceError::Internal)?
        .ok_or_else(|| ServiceError::not_found("Parent step"))?;

    let pipeline_id = parent_step
        .child_workflow_id
        .ok_or_else(|| ServiceError::Validation("Parent step has no pipeline".into()))?;

    // Remove all edges touching this step
    let edges = repo
        .list_edges(pipeline_id)
        .await
        .map_err(ServiceError::Internal)?;

    for edge in &edges {
        if edge.from_step_id == step_id || edge.to_step_id == step_id {
            repo.remove_edge(edge.from_step_id, edge.to_step_id)
                .await
                .map_err(ServiceError::Internal)?;
        }
    }

    // Delete the step
    repo.delete_step(step_id)
        .await
        .map_err(ServiceError::Internal)?;

    // Check if pipeline is now empty
    let remaining_steps = repo
        .list_steps(pipeline_id)
        .await
        .map_err(ServiceError::Internal)?;

    if remaining_steps.is_empty() {
        // Destroy the entire pipeline
        super::destroy::destroy_pipeline(repo, ctx).await?;
        return Ok(vec![]);
    }

    // Recompute execution order
    recompute_execution_order(repo, pipeline_id).await
}
