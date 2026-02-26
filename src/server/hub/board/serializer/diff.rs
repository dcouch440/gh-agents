//! Snapshot diffing — compares two [`CanvasSnapshot`]s by element ID
//! and produces a [`CanvasChangeset`] categorizing all differences.

use std::collections::HashMap;

use super::types::*;

// ============================================================================
// Public API
// ============================================================================

/// Diff two canvas snapshots, producing a categorized changeset.
///
/// Elements are matched by their `element_id` (stable Excalidraw IDs).
///
/// # Node classification
///
/// | Previous | Current | Category |
/// |----------|---------|----------|
/// | absent   | present | `new_nodes` |
/// | present  | absent  | `deleted_node_ids` |
/// | present  | present, text or annotations changed | `updated_nodes` |
/// | present  | present, only bounds changed | `moved_nodes` |
/// | present  | present, nothing changed | (skipped) |
///
/// # Edge classification
///
/// | Previous | Current | Category |
/// |----------|---------|----------|
/// | absent   | present | `new_edges` |
/// | present  | absent  | `deleted_edge_ids` |
/// | present  | present, source or target changed | `rewired_edges` |
/// | present  | present, nothing changed | (skipped) |
pub fn diff_snapshots(previous: &CanvasSnapshot, current: &CanvasSnapshot) -> CanvasChangeset {
    let node_changes = diff_nodes(&previous.nodes, &current.nodes);
    let edge_changes = diff_edges(&previous.edges, &current.edges);

    CanvasChangeset {
        new_nodes: node_changes.new_nodes,
        updated_nodes: node_changes.updated_nodes,
        deleted_node_ids: node_changes.deleted_node_ids,
        moved_nodes: node_changes.moved_nodes,
        new_edges: edge_changes.new_edges,
        deleted_edge_ids: edge_changes.deleted_edge_ids,
        rewired_edges: edge_changes.rewired_edges,
    }
}

// ============================================================================
// Node Diffing
// ============================================================================

struct NodeChanges {
    new_nodes: Vec<CanvasNode>,
    updated_nodes: Vec<NodeUpdate>,
    deleted_node_ids: Vec<String>,
    moved_nodes: Vec<NodeMove>,
}

fn diff_nodes(previous: &[CanvasNode], current: &[CanvasNode]) -> NodeChanges {
    let prev_map: HashMap<&str, &CanvasNode> = previous
        .iter()
        .map(|n| (n.element_id.as_str(), n))
        .collect();
    let curr_map: HashMap<&str, &CanvasNode> =
        current.iter().map(|n| (n.element_id.as_str(), n)).collect();

    let mut new_nodes = Vec::new();
    let mut updated_nodes = Vec::new();
    let mut deleted_node_ids = Vec::new();
    let mut moved_nodes = Vec::new();

    // New and updated/moved
    for curr in current {
        match prev_map.get(curr.element_id.as_str()) {
            None => new_nodes.push(curr.clone()),
            Some(prev) => {
                let text_changed = prev.raw_text != curr.raw_text;
                let annotations_changed = prev.annotations != curr.annotations;
                let bounds_changed = prev.bounds != curr.bounds;

                if text_changed || annotations_changed {
                    updated_nodes.push(NodeUpdate {
                        element_id: curr.element_id.clone(),
                        old_text: prev.raw_text.clone(),
                        new_text: curr.raw_text.clone(),
                        old_annotations: prev.annotations.clone(),
                        new_annotations: curr.annotations.clone(),
                        sketch: curr.sketch.clone(),
                        stroke_encoding: curr.stroke_encoding.clone(),
                    });
                } else if bounds_changed {
                    moved_nodes.push(NodeMove {
                        element_id: curr.element_id.clone(),
                        old_bounds: prev.bounds.clone(),
                        new_bounds: curr.bounds.clone(),
                    });
                }
            }
        }
    }

    // Deleted
    for prev in previous {
        if !curr_map.contains_key(prev.element_id.as_str()) {
            deleted_node_ids.push(prev.element_id.clone());
        }
    }

    NodeChanges {
        new_nodes,
        updated_nodes,
        deleted_node_ids,
        moved_nodes,
    }
}

// ============================================================================
// Edge Diffing
// ============================================================================

struct EdgeChanges {
    new_edges: Vec<CanvasEdge>,
    deleted_edge_ids: Vec<String>,
    rewired_edges: Vec<EdgeRewire>,
}

fn diff_edges(previous: &[CanvasEdge], current: &[CanvasEdge]) -> EdgeChanges {
    let prev_map: HashMap<&str, &CanvasEdge> = previous
        .iter()
        .map(|e| (e.element_id.as_str(), e))
        .collect();
    let curr_map: HashMap<&str, &CanvasEdge> =
        current.iter().map(|e| (e.element_id.as_str(), e)).collect();

    let mut new_edges = Vec::new();
    let mut deleted_edge_ids = Vec::new();
    let mut rewired_edges = Vec::new();

    // New and rewired
    for curr in current {
        match prev_map.get(curr.element_id.as_str()) {
            None => new_edges.push(curr.clone()),
            Some(prev) => {
                let source_changed = prev.source_node_id != curr.source_node_id;
                let target_changed = prev.target_node_id != curr.target_node_id;

                if source_changed || target_changed {
                    rewired_edges.push(EdgeRewire {
                        element_id: curr.element_id.clone(),
                        old_source: prev.source_node_id.clone(),
                        old_target: prev.target_node_id.clone(),
                        new_source: curr.source_node_id.clone(),
                        new_target: curr.target_node_id.clone(),
                    });
                }
            }
        }
    }

    // Deleted
    for prev in previous {
        if !curr_map.contains_key(prev.element_id.as_str()) {
            deleted_edge_ids.push(prev.element_id.clone());
        }
    }

    EdgeChanges {
        new_edges,
        deleted_edge_ids,
        rewired_edges,
    }
}
