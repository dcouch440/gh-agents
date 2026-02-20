//! Topological execution order recomputation for pipeline steps.

use std::cmp::Reverse;
use std::collections::{BinaryHeap, HashMap, HashSet};

use uuid::Uuid;

use crate::db::traits::WorkflowRepo;
use crate::db::WorkflowStepEdgeRow;
use crate::server::services::ServiceError;

use super::types::ExecutionOrderEntry;

/// Recompute the topological execution order for all steps in a pipeline
/// using Kahn's algorithm with a min-heap tie-break.
///
/// Updates `display_order` on each child step in the DB and returns the
/// ordered sequence.
pub async fn recompute_execution_order(
    repo: &dyn WorkflowRepo,
    pipeline_id: Uuid,
) -> Result<Vec<ExecutionOrderEntry>, ServiceError> {
    let steps = repo
        .list_steps(pipeline_id)
        .await
        .map_err(|e| ServiceError::Internal(e.into()))?;

    let pipeline_steps: Vec<_> = steps.iter().collect();
    if pipeline_steps.is_empty() {
        return Ok(vec![]);
    }

    let edges = repo
        .list_edges(pipeline_id)
        .await
        .map_err(|e| ServiceError::Internal(e.into()))?;

    let step_ids: HashSet<Uuid> = pipeline_steps.iter().map(|s| s.id).collect();
    let step_index: HashMap<Uuid, usize> = pipeline_steps
        .iter()
        .enumerate()
        .map(|(i, s)| (s.id, i))
        .collect();

    compute_and_persist(repo, &pipeline_steps, &edges, &step_ids, &step_index).await
}

/// Inner: compute levels via Kahn's algorithm and persist order changes.
async fn compute_and_persist(
    repo: &dyn WorkflowRepo,
    steps: &[&crate::db::WorkflowStepRow],
    edges: &[WorkflowStepEdgeRow],
    step_ids: &HashSet<Uuid>,
    step_index: &HashMap<Uuid, usize>,
) -> Result<Vec<ExecutionOrderEntry>, ServiceError> {
    let n = steps.len();
    let mut in_degree = vec![0usize; n];
    let mut dependents: Vec<Vec<usize>> = vec![vec![]; n];

    // Only count edges between pipeline steps
    for edge in edges {
        if let (Some(&from_i), Some(&to_i)) = (
            step_index.get(&edge.from_step_id),
            step_index.get(&edge.to_step_id),
        ) {
            if step_ids.contains(&edge.from_step_id) && step_ids.contains(&edge.to_step_id) {
                in_degree[to_i] += 1;
                dependents[from_i].push(to_i);
            }
        }
    }

    // Kahn's with min-heap (tie-break by current display_order for stability)
    let mut heap: BinaryHeap<Reverse<(i32, usize)>> = BinaryHeap::new();
    for (i, &deg) in in_degree.iter().enumerate() {
        if deg == 0 {
            heap.push(Reverse((steps[i].display_order, i)));
        }
    }

    let mut sorted: Vec<usize> = Vec::with_capacity(n);
    while let Some(Reverse((_, i))) = heap.pop() {
        sorted.push(i);
        for &dep_i in &dependents[i] {
            in_degree[dep_i] -= 1;
            if in_degree[dep_i] == 0 {
                heap.push(Reverse((steps[dep_i].display_order, dep_i)));
            }
        }
    }

    // Update DB for steps whose order changed, build result
    let mut result = Vec::with_capacity(n);
    for (new_order, &i) in sorted.iter().enumerate() {
        let step = steps[i];
        let new_order_i32 = new_order as i32;

        if step.display_order != new_order_i32 {
            let mut updated = step.clone();
            updated.display_order = new_order_i32;
            repo.update_step(updated)
                .await
                .map_err(|e| ServiceError::Internal(e.into()))?;
        }

        let name = step.name.clone().unwrap_or_default();
        result.push(ExecutionOrderEntry {
            step_id: step.id,
            name,
            order: new_order_i32,
        });
    }

    // Include any steps not in sorted (cycle fallback — shouldn't happen)
    let sorted_set: HashSet<usize> = sorted.iter().copied().collect();
    for (i, step) in steps.iter().enumerate() {
        if !sorted_set.contains(&i) {
            result.push(ExecutionOrderEntry {
                step_id: step.id,
                name: step.name.clone().unwrap_or_default(),
                order: step.display_order,
            });
        }
    }

    Ok(result)
}
