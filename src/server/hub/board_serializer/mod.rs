//! Board serializer — converts raw Excalidraw elements into structured
//! canvas snapshots and changesets.
//!
//! The serializer is the first stage of the Visual Dispatch pipeline.
//! It reads the raw element array sent from the frontend on board submit,
//! classifies each element (node candidate, edge, annotation, or noise),
//! resolves spatial proximity for annotations, and produces a structured
//! [`CanvasSnapshot`].
//!
//! The snapshot can then be diffed against a previous snapshot to produce
//! a [`CanvasChangeset`] — the semantic diff that drives the three-phase
//! pipeline (structural → design → dispatch).
//!
//! # Usage
//!
//! ```ignore
//! use crate::server::hub::board_serializer::{classify_board, diff_snapshots};
//!
//! // On board submit: classify raw elements into a snapshot
//! let snapshot = classify_board(&excalidraw_elements);
//!
//! // Diff against previous snapshot to get changeset
//! let changeset = diff_snapshots(&previous_snapshot, &snapshot);
//! ```

pub mod types;
mod classify;
mod diff;
mod filter;
mod rasterize;
mod resolve;
mod snapshot;

mod tests;

pub use types::*;

/// Default annotation proximity threshold (px).
///
/// Text elements within this distance of a node's bounding box are classified
/// as annotations for that node. Text beyond this distance from all nodes
/// becomes a global note.
const DEFAULT_ANNOTATION_THRESHOLD: f64 = 100.0;

/// Classify raw Excalidraw elements into a structured canvas snapshot.
///
/// Uses the default annotation proximity threshold (100px).
pub fn classify_board(elements: &[ExcalidrawElement]) -> CanvasSnapshot {
    classify_board_with_threshold(elements, DEFAULT_ANNOTATION_THRESHOLD)
}

/// Classify raw Excalidraw elements with a custom annotation proximity threshold.
pub fn classify_board_with_threshold(
    elements: &[ExcalidrawElement],
    annotation_threshold: f64,
) -> CanvasSnapshot {
    let mut classified = classify::classify(elements);
    let unbound = std::mem::take(&mut classified.unbound_text);
    let annotations =
        resolve::resolve_annotations(&classified.nodes, unbound, annotation_threshold);
    snapshot::build_snapshot(classified, annotations)
}

/// Diff two canvas snapshots, producing a categorized changeset.
///
/// Matches elements by their Excalidraw element IDs (stable across sessions).
/// Categorizes differences into: new, updated, deleted, moved (nodes) and
/// new, deleted, rewired (edges).
pub fn diff_snapshots(previous: &CanvasSnapshot, current: &CanvasSnapshot) -> CanvasChangeset {
    diff::diff_snapshots(previous, current)
}

/// Filter and score a changeset, producing a tiered dispatch plan.
///
/// Removes noise (whitespace-only changes, oscillations, canvas pans, line
/// reordering), scores remaining changes by token-level significance, and
/// sorts meaningful changes in topological order (upstream first).
///
/// # Arguments
///
/// * `changeset` — The raw changeset from [`diff_snapshots`].
/// * `current_edges` — All edges in the current snapshot (for topological sorting).
/// * `baseline` — Optional baseline snapshot (last agent-processed state) for
///   oscillation detection. Pass `None` to skip oscillation filtering.
/// * `config` — Filtering thresholds. Use [`FilterConfig::default()`] for standard values.
pub fn filter_changeset(
    changeset: &CanvasChangeset,
    current_edges: &[CanvasEdge],
    baseline: Option<&CanvasSnapshot>,
    config: &FilterConfig,
) -> FilteredChangeset {
    filter::filter_changeset(changeset, current_edges, baseline, config)
}
