//! Cycle detection for pipeline step graphs.

use std::collections::{HashMap, HashSet, VecDeque};

use uuid::Uuid;

use crate::db::WorkflowStepEdgeRow;

/// Check if adding an edge from `from_id` to `to_id` would create a cycle.
///
/// Performs BFS from `to_id` following existing outgoing edges. If
/// `from_id` is reachable, adding the proposed edge creates a cycle.
pub(crate) fn would_create_cycle(
    from_id: Uuid,
    to_id: Uuid,
    edges: &[WorkflowStepEdgeRow],
) -> bool {
    let mut adjacency: HashMap<Uuid, Vec<Uuid>> = HashMap::new();
    for edge in edges {
        adjacency
            .entry(edge.from_step_id)
            .or_default()
            .push(edge.to_step_id);
    }

    let mut visited = HashSet::new();
    let mut queue = VecDeque::new();
    queue.push_back(to_id);

    while let Some(current) = queue.pop_front() {
        if current == from_id {
            return true;
        }
        if !visited.insert(current) {
            continue;
        }
        if let Some(neighbors) = adjacency.get(&current) {
            queue.extend(neighbors);
        }
    }

    false
}
