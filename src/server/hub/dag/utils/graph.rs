//! Graph operations: topological sort and edge traversal.

use std::collections::{BinaryHeap, HashMap, HashSet};

use anyhow::{anyhow, Result};
use uuid::Uuid;

use crate::db::{WorkflowStepEdgeRow, WorkflowStepRow};

/// Returns step IDs in topological order. Errors if cycles are detected.
///
/// Uses Kahn's algorithm with a max-heap keyed by `display_order` for
/// deterministic tie-breaking among nodes at the same topological level.
pub fn topological_sort(
    steps: &[WorkflowStepRow],
    edges: &[WorkflowStepEdgeRow],
) -> Result<Vec<Uuid>> {
    let step_ids: HashSet<Uuid> = steps.iter().map(|s| s.id).collect();
    let mut in_degree: HashMap<Uuid, usize> = step_ids.iter().map(|id| (*id, 0)).collect();
    let mut adjacency: HashMap<Uuid, Vec<Uuid>> = step_ids.iter().map(|id| (*id, vec![])).collect();

    for edge in edges {
        if step_ids.contains(&edge.from_step_id) && step_ids.contains(&edge.to_step_id) {
            adjacency
                .entry(edge.from_step_id)
                .or_default()
                .push(edge.to_step_id);
            *in_degree.entry(edge.to_step_id).or_default() += 1;
        }
    }

    // Build display_order lookup for deterministic tie-breaking
    let step_order: HashMap<Uuid, i32> = steps.iter().map(|s| (s.id, s.display_order)).collect();

    // Max-heap keyed by (display_order, uuid) — pops highest display_order first
    let mut heap: BinaryHeap<(i32, Uuid)> = in_degree
        .iter()
        .filter(|(_, &deg)| deg == 0)
        .map(|(id, _)| (step_order.get(id).copied().unwrap_or(0), *id))
        .collect();

    let mut sorted = Vec::with_capacity(steps.len());
    while let Some((_, node)) = heap.pop() {
        sorted.push(node);
        if let Some(children) = adjacency.get(&node) {
            for child in children {
                if let Some(deg) = in_degree.get_mut(child) {
                    *deg -= 1;
                    if *deg == 0 {
                        heap.push((step_order.get(child).copied().unwrap_or(0), *child));
                    }
                }
            }
        }
    }

    if sorted.len() != step_ids.len() {
        return Err(anyhow!("Cycle detected in workflow DAG"));
    }

    Ok(sorted)
}

/// Returns step IDs grouped by topological level. Steps within the same level
/// have no dependency on each other and can execute in parallel.
///
/// Uses Kahn's algorithm with level tracking. Within each level, steps are
/// sorted by `display_order` for deterministic ordering.
pub fn topological_sort_levels(
    steps: &[WorkflowStepRow],
    edges: &[WorkflowStepEdgeRow],
) -> Result<Vec<Vec<Uuid>>> {
    let step_ids: HashSet<Uuid> = steps.iter().map(|s| s.id).collect();
    let mut in_degree: HashMap<Uuid, usize> = step_ids.iter().map(|id| (*id, 0)).collect();
    let mut adjacency: HashMap<Uuid, Vec<Uuid>> = step_ids.iter().map(|id| (*id, vec![])).collect();

    for edge in edges {
        if step_ids.contains(&edge.from_step_id) && step_ids.contains(&edge.to_step_id) {
            adjacency
                .entry(edge.from_step_id)
                .or_default()
                .push(edge.to_step_id);
            *in_degree.entry(edge.to_step_id).or_default() += 1;
        }
    }

    let step_order: HashMap<Uuid, i32> = steps.iter().map(|s| (s.id, s.display_order)).collect();

    let mut levels: Vec<Vec<Uuid>> = Vec::new();
    let mut current_level: Vec<Uuid> = in_degree
        .iter()
        .filter(|(_, &deg)| deg == 0)
        .map(|(id, _)| *id)
        .collect();
    current_level.sort_by(|a, b| {
        step_order
            .get(b)
            .unwrap_or(&0)
            .cmp(step_order.get(a).unwrap_or(&0))
    });

    let mut total_sorted = 0;
    while !current_level.is_empty() {
        total_sorted += current_level.len();
        let mut next_level: Vec<Uuid> = Vec::new();
        for &node in &current_level {
            if let Some(children) = adjacency.get(&node) {
                for &child in children {
                    if let Some(deg) = in_degree.get_mut(&child) {
                        *deg -= 1;
                        if *deg == 0 {
                            next_level.push(child);
                        }
                    }
                }
            }
        }
        next_level.sort_by(|a, b| {
            step_order
                .get(b)
                .unwrap_or(&0)
                .cmp(step_order.get(a).unwrap_or(&0))
        });
        levels.push(std::mem::take(&mut current_level));
        current_level = next_level;
    }

    if total_sorted != step_ids.len() {
        return Err(anyhow!("Cycle detected in workflow DAG"));
    }

    Ok(levels)
}

/// Returns step IDs that have no incoming edges (entry points).
pub fn find_entry_steps(steps: &[WorkflowStepRow], edges: &[WorkflowStepEdgeRow]) -> Vec<Uuid> {
    let has_incoming: HashSet<Uuid> = edges.iter().map(|e| e.to_step_id).collect();
    steps
        .iter()
        .filter(|s| !has_incoming.contains(&s.id))
        .map(|s| s.id)
        .collect()
}

/// Returns step IDs that a given step depends on (parents).
pub fn get_parent_steps(step_id: Uuid, edges: &[WorkflowStepEdgeRow]) -> Vec<Uuid> {
    edges
        .iter()
        .filter(|e| e.to_step_id == step_id)
        .map(|e| e.from_step_id)
        .collect()
}

/// Returns step IDs that depend on a given step (children).
pub fn get_child_steps(step_id: Uuid, edges: &[WorkflowStepEdgeRow]) -> Vec<Uuid> {
    edges
        .iter()
        .filter(|e| e.from_step_id == step_id)
        .map(|e| e.to_step_id)
        .collect()
}

/// Compute the set of steps that can be skipped because all their downstream
/// consumers are pinned (or themselves dead-path).
///
/// A non-terminal, non-pinned step is dead-path if **every** child is either
/// pinned or dead-path. Terminal steps (no outgoing edges) are never dead-path.
/// Iterates to a fixed point.
pub fn compute_dead_path_steps(
    steps: &[WorkflowStepRow],
    edges: &[WorkflowStepEdgeRow],
) -> HashSet<Uuid> {
    let step_ids: HashSet<Uuid> = steps.iter().map(|s| s.id).collect();
    let pinned: HashSet<Uuid> = steps.iter().filter(|s| s.pinned).map(|s| s.id).collect();

    // Build children map
    let mut children: HashMap<Uuid, Vec<Uuid>> = step_ids.iter().map(|id| (*id, vec![])).collect();
    for edge in edges {
        if step_ids.contains(&edge.from_step_id) && step_ids.contains(&edge.to_step_id) {
            children
                .entry(edge.from_step_id)
                .or_default()
                .push(edge.to_step_id);
        }
    }

    let mut dead_path: HashSet<Uuid> = HashSet::new();

    // Iterate to fixed point
    loop {
        let mut changed = false;
        for step in steps {
            if step.pinned || dead_path.contains(&step.id) {
                continue;
            }
            let kids = children.get(&step.id).map(|v| v.as_slice()).unwrap_or(&[]);
            // Terminal nodes are never dead-path
            if kids.is_empty() {
                continue;
            }
            // Dead-path if ALL children are pinned or dead-path
            if kids
                .iter()
                .all(|kid| pinned.contains(kid) || dead_path.contains(kid))
            {
                dead_path.insert(step.id);
                changed = true;
            }
        }
        if !changed {
            break;
        }
    }

    dead_path
}
