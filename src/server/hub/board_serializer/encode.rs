//! Stroke encoder — RDP-simplifies freehand strokes and encodes them as
//! compact JSON coordinates for LLM consumption.
//!
//! # Encoding
//!
//! Produces a JSON string like:
//! ```json
//! {"canvas":[480,360],"strokes":[{"points":[[20,20],[460,20]]}]}
//! ```
//!
//! Coordinates are relative to the node's bounding box origin (top-left = 0,0)
//! and rounded to integers. The `canvas` field gives the bounding box dimensions.
//!
//! # RDP Algorithm
//!
//! Ramer-Douglas-Peucker simplification reduces polyline vertices while
//! preserving shape. A 20-point freehand stroke typically reduces to 3-5
//! key vertices, cutting token cost from ~1,200 (ASCII grid) to ~30-100.

use serde_json::json;

use super::types::CanvasBounds;

/// Default RDP epsilon in canvas pixels.
///
/// Points within this perpendicular distance of the line between their
/// neighbors are removed. 3.0px filters hand tremor while preserving
/// intentional shape changes.
const DEFAULT_RDP_EPSILON: f64 = 3.0;

// ============================================================================
// Public API
// ============================================================================

/// Encode pen strokes as RDP-simplified JSON coordinates.
///
/// Each stroke is a sequence of `[x, y]` points in absolute canvas
/// coordinates. Points are normalized to node-relative coordinates
/// (origin = top-left of bounds), simplified via RDP, and rounded
/// to integers.
///
/// Returns `None` if `strokes` is empty or `bounds` has zero area.
pub(crate) fn encode_strokes(strokes: &[Vec<[f64; 2]>], bounds: &CanvasBounds) -> Option<String> {
    if strokes.is_empty() {
        return None;
    }
    if bounds.width <= 0.0 || bounds.height <= 0.0 {
        return None;
    }

    let mut encoded_strokes = Vec::new();

    for stroke in strokes {
        if stroke.is_empty() {
            continue;
        }

        let simplified = rdp_simplify(stroke, DEFAULT_RDP_EPSILON);
        if simplified.is_empty() {
            continue;
        }

        let relative = to_relative_rounded(&simplified, bounds);
        encoded_strokes.push(json!({ "points": relative }));
    }

    if encoded_strokes.is_empty() {
        return None;
    }

    let output = json!({
        "canvas": [bounds.width.round() as i64, bounds.height.round() as i64],
        "strokes": encoded_strokes,
    });

    Some(output.to_string())
}

// ============================================================================
// RDP Simplification
// ============================================================================

/// Simplify a polyline using the Ramer-Douglas-Peucker algorithm.
///
/// Returns a new polyline with redundant points removed. Points whose
/// perpendicular distance to the line between their neighbors is less
/// than `epsilon` are dropped.
fn rdp_simplify(points: &[[f64; 2]], epsilon: f64) -> Vec<[f64; 2]> {
    if points.len() <= 2 {
        return points.to_vec();
    }

    let first = points[0];
    let last = points[points.len() - 1];

    // Find the point with maximum perpendicular distance from the
    // line between first and last.
    let mut max_dist = 0.0_f64;
    let mut max_idx = 0;

    for (i, point) in points.iter().enumerate().skip(1).take(points.len() - 2) {
        let dist = perpendicular_distance(*point, first, last);
        if dist > max_dist {
            max_dist = dist;
            max_idx = i;
        }
    }

    if max_dist > epsilon {
        // Recurse on both halves, joining at the split point.
        let mut left = rdp_simplify(&points[..=max_idx], epsilon);
        let right = rdp_simplify(&points[max_idx..], epsilon);
        // Drop the duplicate point at the join.
        left.pop();
        left.extend(right);
        left
    } else {
        // All interior points are within tolerance — keep only endpoints.
        vec![first, last]
    }
}

/// Perpendicular distance from point `p` to the line defined by `a` and `b`.
///
/// If `a == b` (degenerate line), returns the Euclidean distance from `p` to `a`.
fn perpendicular_distance(p: [f64; 2], a: [f64; 2], b: [f64; 2]) -> f64 {
    let dx = b[0] - a[0];
    let dy = b[1] - a[1];
    let line_len_sq = dx * dx + dy * dy;

    if line_len_sq < f64::EPSILON {
        // Degenerate line — just Euclidean distance to the point.
        return ((p[0] - a[0]).powi(2) + (p[1] - a[1]).powi(2)).sqrt();
    }

    // Standard point-to-line distance formula.
    ((dy * p[0] - dx * p[1] + b[0] * a[1] - b[1] * a[0]).abs()) / line_len_sq.sqrt()
}

// ============================================================================
// Coordinate Conversion
// ============================================================================

/// Convert absolute canvas coordinates to bounds-relative and round to integers.
fn to_relative_rounded(points: &[[f64; 2]], bounds: &CanvasBounds) -> Vec<[i64; 2]> {
    points
        .iter()
        .map(|p| {
            [
                (p[0] - bounds.x).round() as i64,
                (p[1] - bounds.y).round() as i64,
            ]
        })
        .collect()
}
