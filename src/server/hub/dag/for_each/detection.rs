//! Phase 6B: Chained for-each pipeline detection.
//!
//! Pure graph analysis to identify chains of consecutive for-each steps
//! connected by single edges, enabling per-item pipeline optimization.

use std::collections::{HashMap, HashSet};

use uuid::Uuid;

use crate::db::{WorkflowStepEdgeRow, WorkflowStepRow};

/// A contiguous chain of for-each steps connected by single edges.
/// Items flow through the chain without barriers between stages.
#[derive(Debug, Clone)]
pub(crate) struct ForEachChain {
    /// Ordered step IDs — first is the entry, last feeds the barrier.
    pub(crate) step_ids: Vec<Uuid>,
}

/// Detect chains of consecutive for-each steps.
///
/// A chain is a maximal sequence `[S1, S2, ..., Sn]` (n >= 2) where:
/// - Every step has `execution_mode == "for_each"`
/// - Each `S_{i+1}` has exactly one parent that is `S_i`
/// - Each `S_i` has exactly one for-each child that is `S_{i+1}`
///
/// Fan-out (multiple children) and fan-in (multiple parents) break chains.
pub(crate) fn detect_for_each_chains(
    steps: &[WorkflowStepRow],
    edges: &[WorkflowStepEdgeRow],
) -> Vec<ForEachChain> {
    let step_map: HashMap<Uuid, &WorkflowStepRow> = steps.iter().map(|s| (s.id, s)).collect();

    // Build adjacency: children and parents per step
    let mut children: HashMap<Uuid, Vec<Uuid>> = HashMap::new();
    let mut parents: HashMap<Uuid, Vec<Uuid>> = HashMap::new();
    for edge in edges {
        children
            .entry(edge.from_step_id)
            .or_default()
            .push(edge.to_step_id);
        parents
            .entry(edge.to_step_id)
            .or_default()
            .push(edge.from_step_id);
    }

    let mut claimed: HashSet<Uuid> = HashSet::new();
    let mut chains = Vec::new();

    for step in steps {
        if claimed.contains(&step.id) {
            continue;
        }
        if step.execution_mode != "for_each" {
            continue;
        }

        // Start building a chain from this step
        let mut chain_ids = vec![step.id];
        let mut current = step.id;

        loop {
            let step_children = children.get(&current).cloned().unwrap_or_default();

            // Find for-each children
            let fe_children: Vec<Uuid> = step_children
                .iter()
                .filter(|cid| {
                    step_map
                        .get(cid)
                        .is_some_and(|s| s.execution_mode == "for_each")
                })
                .copied()
                .collect();

            // Must have exactly one for-each child to continue chain
            if fe_children.len() != 1 {
                break;
            }

            let next = fe_children[0];

            // The child must have exactly one parent (current step)
            let child_parents = parents.get(&next).cloned().unwrap_or_default();
            if child_parents.len() != 1 || child_parents[0] != current {
                break;
            }

            // Don't re-claim
            if claimed.contains(&next) {
                break;
            }

            chain_ids.push(next);
            current = next;
        }

        // Only record chains of length >= 2
        if chain_ids.len() >= 2 {
            for id in &chain_ids {
                claimed.insert(*id);
            }
            chains.push(ForEachChain {
                step_ids: chain_ids,
            });
        }
    }

    chains
}
