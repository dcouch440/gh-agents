//! Element classification — categorizes raw Excalidraw elements into
//! node candidates, edges, and unbound text.
//!
//! Four-pass algorithm:
//! 1. Identify node candidates (rectangles with bound text)
//! 2. Identify edges (arrows binding two node candidates)
//! 3. Collect unbound text (free-floating text elements)
//! 4. Collect strokes (freedraw/line elements assigned to nodes by bounding-box overlap)

use std::collections::{HashMap, HashSet};

use super::types::*;

// ============================================================================
// Internal Classification Types
// ============================================================================

/// A rectangle that has been identified as a node candidate.
pub(crate) struct ClassifiedNode {
    pub id: String,
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
    pub text: String,
}

/// An arrow that connects two node candidates.
pub(crate) struct ClassifiedEdge {
    pub id: String,
    pub source_id: String,
    pub target_id: String,
}

/// A text element that is not bound to any shape.
pub(crate) struct UnboundText {
    pub id: String,
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
    pub text: String,
}

/// A stroke (freedraw or line) assigned to a node by bounding-box overlap.
pub(crate) struct ClassifiedStroke {
    /// Absolute canvas coordinates for this stroke.
    pub points: Vec<[f64; 2]>,
    /// Element ID of the node this stroke is inside, or `None` if it
    /// overlaps zero or multiple nodes.
    pub node_id: Option<String>,
}

/// The result of classifying an array of Excalidraw elements.
pub(crate) struct ClassifiedElements {
    pub nodes: Vec<ClassifiedNode>,
    pub edges: Vec<ClassifiedEdge>,
    pub unbound_text: Vec<UnboundText>,
    pub strokes: Vec<ClassifiedStroke>,
}

// ============================================================================
// Public API
// ============================================================================

/// Classify raw Excalidraw elements into nodes, edges, and unbound text.
///
/// # Algorithm
///
/// 1. Build an element lookup map (`id → &element`) for O(1) access.
/// 2. **Pass 1**: Find node candidates — rectangles whose `bound_elements`
///    contain at least one `"text"` entry. Look up the text element and
///    extract its content.
/// 3. **Pass 2**: Find edges — arrows where both `start_binding` and
///    `end_binding` reference node candidates.
/// 4. **Pass 3**: Collect unbound text — text elements with no `container_id`
///    that weren't already consumed as node text.
/// 5. **Pass 4**: Collect strokes — freedraw and line elements converted to
///    absolute coordinates and assigned to the single overlapping node (if any).
pub(crate) fn classify(elements: &[ExcalidrawElement]) -> ClassifiedElements {
    // Build lookup: element_id → &ExcalidrawElement
    let lookup: HashMap<&str, &ExcalidrawElement> = elements
        .iter()
        .filter_map(|el| match el {
            ExcalidrawElement::Rectangle(r) if !r.is_deleted => Some((r.id.as_str(), el)),
            ExcalidrawElement::Arrow(a) if !a.is_deleted => Some((a.id.as_str(), el)),
            ExcalidrawElement::Text(t) if !t.is_deleted => Some((t.id.as_str(), el)),
            _ => None,
        })
        .collect();

    // Pass 1: Find node candidates
    let mut nodes = Vec::new();
    let mut consumed_text_ids: HashSet<String> = HashSet::new();

    for el in elements {
        let ExcalidrawElement::Rectangle(rect) = el else {
            continue;
        };
        if rect.is_deleted {
            continue;
        }

        // Find the first bound text element
        let bound_text = rect.bound_elements.iter().find_map(|bound_ref| {
            if bound_ref.kind != "text" {
                return None;
            }
            match lookup.get(bound_ref.id.as_str()) {
                Some(ExcalidrawElement::Text(t)) if !t.text.is_empty() => Some(t),
                _ => None,
            }
        });

        if let Some(text_el) = bound_text {
            consumed_text_ids.insert(text_el.id.clone());
            nodes.push(ClassifiedNode {
                id: rect.id.clone(),
                x: rect.x,
                y: rect.y,
                width: rect.width,
                height: rect.height,
                text: text_el.text.clone(),
            });
        }
    }

    // Pass 1b: Spatial containment fallback — if a rectangle has no bound text
    // but a free-floating text element is geometrically inside it, adopt the text.
    // Collects ALL unbound text whose center falls inside the rectangle and
    // concatenates it (newline-separated) as the node's raw_text.
    // This handles users who select the text tool and type inside a box instead
    // of double-clicking the box (which creates an Excalidraw binding).
    let node_rect_ids: HashSet<&str> = nodes.iter().map(|n| n.id.as_str()).collect();

    // Collect adoptions as (rect_id, rect_bounds, Vec<(text_id, text_content)>)
    let mut adoptions: Vec<(String, f64, f64, f64, f64, Vec<(String, String)>)> = Vec::new();

    for el in elements {
        let ExcalidrawElement::Rectangle(rect) = el else {
            continue;
        };
        if rect.is_deleted || node_rect_ids.contains(rect.id.as_str()) {
            continue;
        }

        let mut texts: Vec<(String, String)> = Vec::new();
        for candidate in elements {
            let ExcalidrawElement::Text(t) = candidate else {
                continue;
            };
            if t.is_deleted || t.text.is_empty() || t.container_id.is_some() {
                continue;
            }
            if consumed_text_ids.contains(&t.id) {
                continue;
            }
            let text_cx = t.x + t.width / 2.0;
            let text_cy = t.y + t.height / 2.0;
            if text_cx >= rect.x
                && text_cx <= rect.x + rect.width
                && text_cy >= rect.y
                && text_cy <= rect.y + rect.height
            {
                texts.push((t.id.clone(), t.text.clone()));
            }
        }

        if !texts.is_empty() {
            adoptions.push((rect.id.clone(), rect.x, rect.y, rect.width, rect.height, texts));
        }
    }

    for (rect_id, x, y, width, height, texts) in adoptions {
        let mut combined = String::new();
        for (text_id, text_content) in &texts {
            if consumed_text_ids.contains(text_id.as_str()) {
                continue;
            }
            consumed_text_ids.insert(text_id.clone());
            if !combined.is_empty() {
                combined.push('\n');
            }
            combined.push_str(text_content);
        }
        if !combined.is_empty() {
            nodes.push(ClassifiedNode {
                id: rect_id,
                x,
                y,
                width,
                height,
                text: combined,
            });
        }
    }

    // Build node candidate ID set for edge validation
    let node_ids: HashSet<&str> = nodes.iter().map(|n| n.id.as_str()).collect();

    // Pass 2: Find edges
    let mut edges = Vec::new();
    for el in elements {
        let ExcalidrawElement::Arrow(arrow) = el else {
            continue;
        };
        if arrow.is_deleted {
            continue;
        }

        let (Some(start), Some(end)) = (&arrow.start_binding, &arrow.end_binding) else {
            continue;
        };

        // Both endpoints must reference node candidates
        if node_ids.contains(start.element_id.as_str())
            && node_ids.contains(end.element_id.as_str())
        {
            edges.push(ClassifiedEdge {
                id: arrow.id.clone(),
                source_id: start.element_id.clone(),
                target_id: end.element_id.clone(),
            });
        }
    }

    // Pass 3: Collect unbound text
    let mut unbound_text = Vec::new();
    for el in elements {
        let ExcalidrawElement::Text(text) = el else {
            continue;
        };
        if text.is_deleted {
            continue;
        }
        // Skip text that was consumed as node text
        if consumed_text_ids.contains(text.id.as_str()) {
            continue;
        }
        // Skip text bound inside a shape (container_id set)
        if text.container_id.is_some() {
            continue;
        }

        if !text.text.is_empty() {
            unbound_text.push(UnboundText {
                id: text.id.clone(),
                x: text.x,
                y: text.y,
                width: text.width,
                height: text.height,
                text: text.text.clone(),
            });
        }
    }

    // Pass 4: Collect strokes (freedraw + line elements)
    let mut strokes = Vec::new();
    for el in elements {
        let (base_x, base_y, points, is_deleted) = match el {
            ExcalidrawElement::Freedraw(fd) => (fd.x, fd.y, &fd.points, fd.is_deleted),
            ExcalidrawElement::Line(ln) => (ln.x, ln.y, &ln.points, ln.is_deleted),
            _ => continue,
        };
        if is_deleted || points.is_empty() {
            continue;
        }

        // Convert relative points to absolute canvas coordinates
        let abs_points: Vec<[f64; 2]> = points
            .iter()
            .filter_map(|pt| {
                if pt.len() >= 2 {
                    Some([base_x + pt[0], base_y + pt[1]])
                } else {
                    None
                }
            })
            .collect();

        if abs_points.is_empty() {
            continue;
        }

        // Compute stroke bounding box
        let stroke_min_x = abs_points
            .iter()
            .map(|p| p[0])
            .fold(f64::INFINITY, f64::min);
        let stroke_max_x = abs_points
            .iter()
            .map(|p| p[0])
            .fold(f64::NEG_INFINITY, f64::max);
        let stroke_min_y = abs_points
            .iter()
            .map(|p| p[1])
            .fold(f64::INFINITY, f64::min);
        let stroke_max_y = abs_points
            .iter()
            .map(|p| p[1])
            .fold(f64::NEG_INFINITY, f64::max);

        // Find which nodes this stroke overlaps (AABB overlap test)
        let mut overlapping_node_id = None;
        let mut overlap_count = 0;

        for node in &nodes {
            let node_max_x = node.x + node.width;
            let node_max_y = node.y + node.height;

            if stroke_min_x <= node_max_x
                && stroke_max_x >= node.x
                && stroke_min_y <= node_max_y
                && stroke_max_y >= node.y
            {
                overlap_count += 1;
                overlapping_node_id = Some(node.id.clone());
            }
        }

        // Only assign to a node if exactly one node overlaps
        let node_id = if overlap_count == 1 {
            overlapping_node_id
        } else {
            None
        };

        strokes.push(ClassifiedStroke {
            points: abs_points,
            node_id,
        });
    }

    ClassifiedElements {
        nodes,
        edges,
        unbound_text,
        strokes,
    }
}
