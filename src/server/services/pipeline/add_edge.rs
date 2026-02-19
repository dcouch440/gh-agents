//! Add a dependency edge between pipeline steps.

use uuid::Uuid;

use crate::db::traits::WorkflowRepo;
use crate::server::services::ServiceError;

use super::cycle::would_create_cycle;
use super::recompute::recompute_execution_order;
use super::types::{ExecutionOrderEntry, PipelineContext};

/// Add an edge between two pipeline steps (dependency).
///
/// Validates: no self-edges, no duplicates, no cycles.
/// Returns the recomputed execution sequence.
pub async fn add_edge(
    repo: &dyn WorkflowRepo,
    ctx: &PipelineContext,
    from_step_id: Uuid,
    to_step_id: Uuid,
) -> Result<Vec<ExecutionOrderEntry>, ServiceError> {
    // No self-edges
    if from_step_id == to_step_id {
        return Err(ServiceError::Validation(
            "Cannot create an edge from a step to itself".into(),
        ));
    }

    let parent_step = repo
        .get_step(ctx.parent_step_id)
        .await
        .map_err(|e| ServiceError::Internal(e.into()))?
        .ok_or_else(|| ServiceError::not_found("Parent step"))?;

    let pipeline_id = parent_step
        .child_workflow_id
        .ok_or_else(|| ServiceError::Validation("Parent step has no pipeline".into()))?;

    let edges = repo
        .list_edges(pipeline_id)
        .await
        .map_err(|e| ServiceError::Internal(e.into()))?;

    // No duplicates
    if edges
        .iter()
        .any(|e| e.from_step_id == from_step_id && e.to_step_id == to_step_id)
    {
        return Err(ServiceError::Conflict("Edge already exists".into()));
    }

    // No cycles
    if would_create_cycle(from_step_id, to_step_id, &edges) {
        return Err(ServiceError::Validation(
            "Adding this edge would create a cycle".into(),
        ));
    }

    // Create the edge
    repo.add_edge(pipeline_id, from_step_id, to_step_id)
        .await
        .map_err(|e| ServiceError::Internal(e.into()))?;

    // Recompute execution order
    recompute_execution_order(repo, pipeline_id).await
}
