//! Snapshot assembly — combines classified elements and resolved annotations
//! into a [`CanvasSnapshot`].

use super::classify::{ClassifiedEdge, ClassifiedElements, ClassifiedNode, ClassifiedStroke};
use super::encode;
use super::rasterize;
use super::rasterize_png;
use super::resolve::ResolvedAnnotations;
use super::types::*;

// ============================================================================
// Public API
// ============================================================================

/// Build a [`CanvasSnapshot`] from classified elements and resolved annotations.
///
/// Maps each [`ClassifiedNode`] to a [`CanvasNode`] (attaching annotations
/// from the resolution map), each [`ClassifiedEdge`] to a [`CanvasEdge`],
/// and carries forward global notes.
pub(crate) fn build_snapshot(
    classified: ClassifiedElements,
    annotations: ResolvedAnnotations,
) -> CanvasSnapshot {
    let ClassifiedElements {
        nodes,
        edges,
        unbound_text: _, // already consumed by resolve
        strokes,
    } = classified;

    let canvas_nodes = nodes
        .into_iter()
        .map(|n| build_node(n, &annotations, &strokes))
        .collect();
    let canvas_edges = edges.into_iter().map(build_edge).collect();

    CanvasSnapshot {
        nodes: canvas_nodes,
        edges: canvas_edges,
        global_notes: annotations.global_notes,
    }
}

// ============================================================================
// Helpers
// ============================================================================

/// Default rasterizer grid dimensions.
const RASTER_COLS: usize = 48;
const RASTER_ROWS: usize = 24;

fn build_node(
    node: ClassifiedNode,
    annotations: &ResolvedAnnotations,
    strokes: &[ClassifiedStroke],
) -> CanvasNode {
    let node_annotations = annotations
        .node_annotations
        .get(&node.id)
        .cloned()
        .unwrap_or_default();

    let bounds = CanvasBounds {
        x: node.x,
        y: node.y,
        width: node.width,
        height: node.height,
    };

    // Collect strokes assigned to this node (with pressure)
    let node_strokes_3: Vec<Vec<[f64; 3]>> = strokes
        .iter()
        .filter(|s| s.node_id.as_deref() == Some(&node.id))
        .map(|s| s.points.clone())
        .collect();

    // Project to [x, y] for ASCII rasterizer and coordinate encoder
    let node_strokes_2: Vec<Vec<[f64; 2]>> = node_strokes_3
        .iter()
        .map(|stroke| stroke.iter().map(|p| [p[0], p[1]]).collect())
        .collect();

    let sketch = if node_strokes_2.is_empty() {
        None
    } else {
        rasterize::rasterize_strokes(&node_strokes_2, &bounds, RASTER_COLS, RASTER_ROWS)
    };

    let stroke_encoding = if node_strokes_2.is_empty() {
        None
    } else {
        encode::encode_strokes(&node_strokes_2, &bounds)
    };

    let stroke_png_base64 = if node_strokes_3.is_empty() {
        None
    } else {
        rasterize_png::rasterize_strokes_png(&node_strokes_3, &bounds, 1536, 10, 5)
    };

    CanvasNode {
        element_id: node.id,
        raw_text: node.text,
        bounds,
        annotations: node_annotations,
        sketch,
        stroke_encoding,
        stroke_png_base64,
    }
}

fn build_edge(edge: ClassifiedEdge) -> CanvasEdge {
    CanvasEdge {
        element_id: edge.id,
        source_node_id: edge.source_id,
        target_node_id: edge.target_id,
    }
}
