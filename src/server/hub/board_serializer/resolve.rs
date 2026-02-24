//! Annotation resolution — assigns unbound text elements to their nearest
//! node candidate based on spatial proximity.
//!
//! Text within the threshold distance of a node becomes an annotation for
//! that node. Text beyond the threshold of all nodes becomes a global note.

use std::collections::HashMap;

use super::classify::{ClassifiedNode, UnboundText};
use super::types::GlobalNote;

// ============================================================================
// Internal Types
// ============================================================================

/// The result of resolving annotations: per-node annotation lists and
/// leftover global notes.
pub(crate) struct ResolvedAnnotations {
    /// Map from node element ID → list of annotation texts.
    pub node_annotations: HashMap<String, Vec<String>>,
    /// Text elements too far from any node.
    pub global_notes: Vec<GlobalNote>,
}

// ============================================================================
// Public API
// ============================================================================

/// Assign unbound text elements to their nearest node candidate, or classify
/// them as global notes if beyond the threshold distance.
///
/// # Proximity measure
///
/// For each unbound text element, we compute the center point and find the
/// minimum distance from that center to each node's axis-aligned bounding
/// box. If the center is inside the AABB, the distance is 0.
///
/// The text is assigned to the closest node within `threshold` pixels.
/// If no node is within range, the text becomes a global note.
pub(crate) fn resolve_annotations(
    nodes: &[ClassifiedNode],
    unbound_text: Vec<UnboundText>,
    threshold: f64,
) -> ResolvedAnnotations {
    let mut node_annotations: HashMap<String, Vec<String>> = HashMap::new();
    let mut global_notes = Vec::new();

    for text in unbound_text {
        let center_x = text.x + text.width / 2.0;
        let center_y = text.y + text.height / 2.0;

        let mut closest: Option<(&str, f64)> = None;

        for node in nodes {
            let dist = point_to_aabb_distance(center_x, center_y, node);
            match closest {
                None => closest = Some((&node.id, dist)),
                Some((_, best_dist)) if dist < best_dist => {
                    closest = Some((&node.id, dist));
                }
                _ => {}
            }
        }

        match closest {
            Some((node_id, dist)) if dist <= threshold => {
                node_annotations
                    .entry(node_id.to_string())
                    .or_default()
                    .push(text.text);
            }
            _ => {
                global_notes.push(GlobalNote {
                    element_id: text.id,
                    text: text.text,
                });
            }
        }
    }

    ResolvedAnnotations {
        node_annotations,
        global_notes,
    }
}

// ============================================================================
// Geometry
// ============================================================================

/// Minimum distance from a point to an axis-aligned bounding box.
/// Returns 0.0 if the point is inside the box.
fn point_to_aabb_distance(px: f64, py: f64, node: &ClassifiedNode) -> f64 {
    let clamped_x = px.clamp(node.x, node.x + node.width);
    let clamped_y = py.clamp(node.y, node.y + node.height);
    let dx = px - clamped_x;
    let dy = py - clamped_y;
    (dx * dx + dy * dy).sqrt()
}
