//! Remove a dependency edge between pipeline steps.

use uuid::Uuid;

use crate::db::traits::WorkflowRepo;
use crate::server::services::ServiceError;

use super::recompute::recompute_execution_order;
use super::types::{ExecutionOrderEntry, PipelineContext};

/// Remove an edge between two pipeline steps.
/// Returns the recomputed execution sequence.
pub async fn remove_edge(
    repo: &dyn WorkflowRepo,
    ctx: &PipelineContext,
    from_step_id: Uuid,
    to_step_id: Uuid,
) -> Result<Vec<ExecutionOrderEntry>, ServiceError> {
    let parent_step = repo
        .get_step(ctx.parent_step_id)
        .await
        .map_err(ServiceError::Internal)?
        .ok_or_else(|| ServiceError::not_found("Parent step"))?;

    let pipeline_id = parent_step
        .child_workflow_id
        .ok_or_else(|| ServiceError::Validation("Parent step has no pipeline".into()))?;

    let _deleted = repo
        .remove_edge(from_step_id, to_step_id)
        .await
        .map_err(ServiceError::Internal)?;

    recompute_execution_order(repo, pipeline_id).await
}
