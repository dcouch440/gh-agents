//! Changeset filtering and scoring — removes noise from a [`CanvasChangeset`]
//! and produces a tiered, scored [`FilteredChangeset`] for dispatch decisions.
//!
//! # Filter pipeline
//!
//! 1. **Pan detection**: If ALL moved nodes share the same (dx, dy) delta
//!    within epsilon tolerance, they are a canvas pan — not meaningful rearrangement.
//! 2. **Whitespace normalization**: Collapse whitespace, trim. If normalized
//!    old and new text match, reclassify as noise.
//! 3. **Oscillation detection**: Compare updated nodes against a baseline
//!    snapshot. If the current text matches baseline text for a node, the user
//!    undid the change — net zero, drop as noise.
//! 4. **Reorder detection**: Split text into lines, compare as sorted sets.
//!    Same set means the user just reordered bullets — noise.
//! 5. **Scoring**: Token-level change ratio for surviving updates.
//! 6. **Topological sort**: Order meaningful changes by dependency
//!    (upstream first) using edges from the current snapshot.

use std::collections::{HashMap, HashSet, VecDeque};

use super::types::*;

// ============================================================================
// Public API
// ============================================================================

/// Filter and score a raw changeset, producing a tiered dispatch plan.
///
/// # Arguments
///
/// * `changeset` — The raw changeset from `diff_snapshots()`.
/// * `current_edges` — All edges in the current snapshot (for topological sorting).
/// * `baseline` — Optional baseline snapshot (last agent-processed state) for
///   oscillation detection. Pass `None` to skip oscillation filtering.
/// * `config` — Filtering thresholds. Use `FilterConfig::default()` for standard values.
pub(crate) fn filter_changeset(
    changeset: &CanvasChangeset,
    current_edges: &[CanvasEdge],
    baseline: Option<&CanvasSnapshot>,
    config: &FilterConfig,
) -> FilteredChangeset {
    let mut noise: Vec<FilteredNoise> = Vec::new();
    let mut meaningful: Vec<ScoredChange> = Vec::new();

    // --- Tier 1: Agentless (structural changes that don't need AI) ---

    let (surviving_moves, pan_noise) = detect_pan(&changeset.moved_nodes, config.pan_epsilon);
    noise.extend(pan_noise);

    let agentless = AgentlessChanges {
        deleted_node_ids: changeset.deleted_node_ids.clone(),
        deleted_edge_ids: changeset.deleted_edge_ids.clone(),
        rewired_edges: changeset.rewired_edges.clone(),
        moved_nodes: surviving_moves,
    };

    // --- Tier 2 & 3: Filter updated_nodes through noise pipeline ---

    // Pre-index baseline nodes by ID for O(1) oscillation lookups (was O(N) per update).
    let baseline_index: Option<HashMap<&str, &CanvasNode>> = baseline.map(|base| {
        base.nodes
            .iter()
            .map(|n| (n.element_id.as_str(), n))
            .collect()
    });

    for update in &changeset.updated_nodes {
        // Filter 1: whitespace normalization
        if is_whitespace_only(update) {
            noise.push(FilteredNoise {
                element_id: update.element_id.clone(),
                reason: NoiseReason::WhitespaceOnly,
            });
            continue;
        }

        // Filter 2: oscillation detection
        if let Some(ref index) = baseline_index {
            if is_oscillation(update, index) {
                noise.push(FilteredNoise {
                    element_id: update.element_id.clone(),
                    reason: NoiseReason::Oscillation,
                });
                continue;
            }
        }

        // Filter 3: reorder detection
        if is_reorder_only(update) {
            noise.push(FilteredNoise {
                element_id: update.element_id.clone(),
                reason: NoiseReason::ReorderOnly,
            });
            continue;
        }

        // Survived all filters — score it
        let ratio = token_change_ratio(&update.old_text, &update.new_text);
        let significance = score_from_ratio(ratio);

        meaningful.push(ScoredChange::UpdatedNode {
            update: update.clone(),
            significance,
            token_change_ratio: ratio,
        });
    }

    // New nodes — always high significance
    for node in &changeset.new_nodes {
        meaningful.push(ScoredChange::NewNode {
            node: node.clone(),
            significance: ChangeSignificance::High,
        });
    }

    // New edges — always medium significance
    for edge in &changeset.new_edges {
        meaningful.push(ScoredChange::NewEdge {
            edge: edge.clone(),
            significance: ChangeSignificance::Medium,
        });
    }

    // --- Topological sort meaningful changes ---

    let meaningful_node_ids: Vec<String> = meaningful
        .iter()
        .filter_map(|c| match c {
            ScoredChange::NewNode { node, .. } => Some(node.element_id.clone()),
            ScoredChange::UpdatedNode { update, .. } => Some(update.element_id.clone()),
            ScoredChange::NewEdge { .. } => None,
        })
        .collect();

    let sorted_ids = topological_sort_nodes(&meaningful_node_ids, current_edges);

    let order_map: HashMap<&str, usize> = sorted_ids
        .iter()
        .enumerate()
        .map(|(i, id)| (id.as_str(), i))
        .collect();

    meaningful.sort_by_key(|change| match change {
        ScoredChange::NewNode { node, .. } => order_map
            .get(node.element_id.as_str())
            .copied()
            .unwrap_or(usize::MAX),
        ScoredChange::UpdatedNode { update, .. } => order_map
            .get(update.element_id.as_str())
            .copied()
            .unwrap_or(usize::MAX),
        ScoredChange::NewEdge { edge, .. } => {
            let src = order_map
                .get(edge.source_node_id.as_str())
                .copied()
                .unwrap_or(usize::MAX);
            let tgt = order_map
                .get(edge.target_node_id.as_str())
                .copied()
                .unwrap_or(usize::MAX);
            src.max(tgt)
        }
    });

    // --- Aggregate score ---

    let aggregate_score = if meaningful.is_empty() {
        0.0
    } else {
        let total: f64 = meaningful.iter().map(|c| c.significance().score()).sum();
        total / meaningful.len() as f64
    };

    let should_dispatch = aggregate_score >= config.dispatch_threshold;

    FilteredChangeset {
        agentless,
        noise,
        meaningful,
        aggregate_score,
        should_dispatch,
    }
}

// ============================================================================
// Pan Detection
// ============================================================================

/// Detect if all moved nodes share the same movement delta (canvas pan).
///
/// Returns `(surviving_moves, pan_noise)`. If all deltas match within epsilon,
/// all moves become noise. Single moves always survive (can't distinguish
/// pan from intentional placement with one node).
fn detect_pan(moved_nodes: &[NodeMove], epsilon: f64) -> (Vec<NodeMove>, Vec<FilteredNoise>) {
    if moved_nodes.len() < 2 {
        return (moved_nodes.to_vec(), vec![]);
    }

    let deltas: Vec<(f64, f64)> = moved_nodes
        .iter()
        .map(|m| {
            (
                m.new_bounds.x - m.old_bounds.x,
                m.new_bounds.y - m.old_bounds.y,
            )
        })
        .collect();

    let (ref_dx, ref_dy) = deltas[0];
    let all_same = deltas
        .iter()
        .all(|(dx, dy)| (dx - ref_dx).abs() < epsilon && (dy - ref_dy).abs() < epsilon);

    if all_same {
        let noise = moved_nodes
            .iter()
            .map(|m| FilteredNoise {
                element_id: m.element_id.clone(),
                reason: NoiseReason::CanvasPan,
            })
            .collect();
        (vec![], noise)
    } else {
        (moved_nodes.to_vec(), vec![])
    }
}

// ============================================================================
// Whitespace Normalization
// ============================================================================

/// Normalize whitespace: collapse all whitespace runs to a single space, trim.
fn normalize_whitespace(s: &str) -> String {
    s.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// Check if a NodeUpdate is whitespace-only noise.
///
/// Normalizes both text and annotations. If all normalized forms match,
/// the change is purely cosmetic whitespace.
fn is_whitespace_only(update: &NodeUpdate) -> bool {
    if normalize_whitespace(&update.old_text) != normalize_whitespace(&update.new_text) {
        return false;
    }

    let old_ann: Vec<String> = update
        .old_annotations
        .iter()
        .map(|a| normalize_whitespace(a))
        .collect();
    let new_ann: Vec<String> = update
        .new_annotations
        .iter()
        .map(|a| normalize_whitespace(a))
        .collect();

    old_ann == new_ann
}

// ============================================================================
// Oscillation Detection
// ============================================================================

/// Check if a node update is an oscillation (current state matches baseline).
///
/// The baseline is the last agent-processed snapshot. If the user changed
/// text A → B → A, the diff sees an update (B → A), but comparing against
/// the baseline (A) reveals the net change is zero.
///
/// Takes a pre-built index for O(1) lookup instead of scanning the full snapshot.
fn is_oscillation(update: &NodeUpdate, baseline_index: &HashMap<&str, &CanvasNode>) -> bool {
    baseline_index
        .get(update.element_id.as_str())
        .is_some_and(|node| {
            node.raw_text == update.new_text && node.annotations == update.new_annotations
        })
}

// ============================================================================
// Reorder Detection
// ============================================================================

/// Check if text change is just line reordering (same set of non-empty lines).
///
/// Splits both old and new text into trimmed, non-empty lines and compares
/// as sorted arrays. If the sorted forms match and annotations are unchanged,
/// the user just reordered bullets.
fn is_reorder_only(update: &NodeUpdate) -> bool {
    if update.old_annotations != update.new_annotations {
        return false;
    }

    let mut old_lines: Vec<&str> = update
        .old_text
        .lines()
        .map(str::trim)
        .filter(|l| !l.is_empty())
        .collect();
    let mut new_lines: Vec<&str> = update
        .new_text
        .lines()
        .map(str::trim)
        .filter(|l| !l.is_empty())
        .collect();

    if old_lines.len() != new_lines.len() || old_lines.is_empty() {
        return false;
    }

    // Must have different order (otherwise diff wouldn't have flagged it)
    if old_lines == new_lines {
        return false;
    }

    old_lines.sort();
    new_lines.sort();
    old_lines == new_lines
}

// ============================================================================
// Token Scoring
// ============================================================================

/// Compute the token change ratio between old and new text.
///
/// Uses a hybrid algorithm: Myers word-level diff identifies which words
/// changed, then Sørensen-Dice character bigram similarity measures how
/// much each replacement actually differs. This correctly handles
/// morphological variants ("database" → "databases" ≈ 7% change) and
/// formatting changes ("JSON formatted" → "JSON-formatted" ≈ 15% change)
/// that the previous whitespace-split multiset approach over-scored.
fn token_change_ratio(old_text: &str, new_text: &str) -> f64 {
    let old_words: Vec<&str> = old_text.split_whitespace().collect();
    let new_words: Vec<&str> = new_text.split_whitespace().collect();

    let total = old_words.len().max(new_words.len());
    if total == 0 {
        return 0.0;
    }

    let ops = similar::capture_diff_slices(similar::Algorithm::Myers, &old_words, &new_words);

    let mut change_score: f64 = 0.0;

    for op in &ops {
        match *op {
            similar::DiffOp::Equal { .. } => {}
            similar::DiffOp::Delete { old_len, .. } => {
                change_score += old_len as f64;
            }
            similar::DiffOp::Insert { new_len, .. } => {
                change_score += new_len as f64;
            }
            similar::DiffOp::Replace {
                old_index,
                old_len,
                new_index,
                new_len,
            } => {
                let old_span = old_words[old_index..old_index + old_len].join(" ");
                let new_span = new_words[new_index..new_index + new_len].join(" ");
                let similarity = strsim::sorensen_dice(&old_span, &new_span);
                change_score += (1.0 - similarity) * old_len.max(new_len) as f64;
            }
        }
    }

    change_score / total as f64
}

/// Map a token change ratio to a significance level.
fn score_from_ratio(ratio: f64) -> ChangeSignificance {
    if ratio < 0.05 {
        ChangeSignificance::Low
    } else if ratio <= 0.20 {
        ChangeSignificance::Medium
    } else {
        ChangeSignificance::High
    }
}

// ============================================================================
// Topological Sort
// ============================================================================

/// Topological sort of node IDs using Kahn's algorithm.
///
/// Returns node IDs in dependency order (upstream first). Nodes not
/// referenced by any edge appear at the end in original order.
/// If cycles exist, falls back to original order for remaining nodes.
fn topological_sort_nodes(node_ids: &[String], edges: &[CanvasEdge]) -> Vec<String> {
    if node_ids.len() <= 1 {
        return node_ids.to_vec();
    }

    let id_set: HashSet<&str> = node_ids.iter().map(|s| s.as_str()).collect();

    let mut in_degree: HashMap<&str, usize> = node_ids.iter().map(|id| (id.as_str(), 0)).collect();
    let mut adjacency: HashMap<&str, Vec<&str>> =
        node_ids.iter().map(|id| (id.as_str(), vec![])).collect();

    for edge in edges {
        let src = edge.source_node_id.as_str();
        let tgt = edge.target_node_id.as_str();

        if id_set.contains(src) && id_set.contains(tgt) {
            adjacency.entry(src).or_default().push(tgt);
            *in_degree.entry(tgt).or_default() += 1;
        }
    }

    // Seed queue with zero-in-degree nodes (in original order for stability)
    let mut queue: VecDeque<&str> = node_ids
        .iter()
        .filter(|id| in_degree.get(id.as_str()).copied().unwrap_or(0) == 0)
        .map(|id| id.as_str())
        .collect();

    let mut sorted: Vec<String> = Vec::with_capacity(node_ids.len());
    let mut visited: HashSet<&str> = HashSet::new();

    while let Some(node) = queue.pop_front() {
        if !visited.insert(node) {
            continue;
        }
        sorted.push(node.to_string());

        if let Some(children) = adjacency.get(node) {
            for child in children {
                if let Some(deg) = in_degree.get_mut(child) {
                    *deg = deg.saturating_sub(1);
                    if *deg == 0 {
                        queue.push_back(child);
                    }
                }
            }
        }
    }

    // Append any nodes not reached (cycle fallback, preserving original order)
    for id in node_ids {
        if !visited.contains(id.as_str()) {
            sorted.push(id.clone());
        }
    }

    sorted
}
