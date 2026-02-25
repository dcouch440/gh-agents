//! Stroke rasterizer — converts freeform drawing coordinates into an ASCII
//! character grid that LLMs can read as plain text.
//!
//! Excalidraw `freedraw` and `line` elements store strokes as coordinate
//! arrays. This module maps those coordinates into a fixed-size character
//! grid, producing a multi-line string where filled cells represent pen
//! strokes. No images, no multimodal — just coordinate math to characters.
//!
//! # Grid dimensions
//!
//! Default: 48 columns × 24 rows. The 2:1 ratio compensates for monospace
//! characters being roughly twice as tall as they are wide, producing a
//! visually proportional output.
//!
//! # Characters
//!
//! - `█` (U+2588) — filled cell (stroke passes through)
//! - `·` (U+00B7) — empty cell

use super::types::CanvasBounds;

/// Filled cell character.
const FILLED: char = '█';

/// Empty cell character.
const EMPTY: char = '·';

// ============================================================================
// Public API
// ============================================================================

/// Rasterize pen strokes into an ASCII character grid.
///
/// Each stroke is a sequence of `[x, y]` points in absolute canvas
/// coordinates. The points are mapped into the grid defined by `bounds`,
/// and consecutive points are connected using Bresenham's line algorithm
/// to ensure smooth coverage even for fast pen movements.
///
/// Returns `None` if `strokes` is empty or `bounds` has zero area.
pub(crate) fn rasterize_strokes(
    strokes: &[Vec<[f64; 2]>],
    bounds: &CanvasBounds,
    cols: usize,
    rows: usize,
) -> Option<String> {
    if strokes.is_empty() || cols == 0 || rows == 0 {
        return None;
    }
    if bounds.width <= 0.0 || bounds.height <= 0.0 {
        return None;
    }

    let mut grid = vec![vec![false; cols]; rows];

    for stroke in strokes {
        if stroke.is_empty() {
            continue;
        }

        // Single-point stroke (dot)
        if stroke.len() == 1 {
            let (c, r) = to_grid(stroke[0][0], stroke[0][1], bounds, cols, rows);
            grid[r][c] = true;
            continue;
        }

        // Walk consecutive point pairs, drawing lines between them
        for pair in stroke.windows(2) {
            let (c0, r0) = to_grid(pair[0][0], pair[0][1], bounds, cols, rows);
            let (c1, r1) = to_grid(pair[1][0], pair[1][1], bounds, cols, rows);
            bresenham(&mut grid, c0, r0, c1, r1);
        }
    }

    // Render grid to string, trimming trailing empty rows
    let mut last_filled_row = 0;
    for (i, row) in grid.iter().enumerate() {
        if row.iter().any(|&cell| cell) {
            last_filled_row = i;
        }
    }

    let rendered: Vec<String> = grid[..=last_filled_row]
        .iter()
        .map(|row| {
            row.iter()
                .map(|&cell| if cell { FILLED } else { EMPTY })
                .collect()
        })
        .collect();

    if rendered.is_empty() || rendered.iter().all(|row| row.chars().all(|c| c == EMPTY)) {
        return None;
    }

    Some(rendered.join("\n"))
}

// ============================================================================
// Coordinate Mapping
// ============================================================================

/// Map an absolute canvas point to grid coordinates.
///
/// Returns `(col, row)` clamped to valid grid indices.
fn to_grid(x: f64, y: f64, bounds: &CanvasBounds, cols: usize, rows: usize) -> (usize, usize) {
    let col = ((x - bounds.x) / bounds.width * cols as f64).clamp(0.0, (cols - 1) as f64) as usize;
    let row = ((y - bounds.y) / bounds.height * rows as f64).clamp(0.0, (rows - 1) as f64) as usize;
    (col, row)
}

// ============================================================================
// Bresenham's Line Algorithm
// ============================================================================

/// Draw a line from `(c0, r0)` to `(c1, r1)` on the grid using Bresenham's
/// line algorithm. Marks every cell along the path as `true`.
fn bresenham(grid: &mut [Vec<bool>], c0: usize, r0: usize, c1: usize, r1: usize) {
    let mut x0 = c0 as isize;
    let mut y0 = r0 as isize;
    let x1 = c1 as isize;
    let y1 = r1 as isize;

    let dx = (x1 - x0).abs();
    let dy = -(y1 - y0).abs();
    let sx: isize = if x0 < x1 { 1 } else { -1 };
    let sy: isize = if y0 < y1 { 1 } else { -1 };
    let mut err = dx + dy;

    loop {
        // Mark current cell
        if y0 >= 0 && (y0 as usize) < grid.len() && x0 >= 0 && (x0 as usize) < grid[0].len() {
            grid[y0 as usize][x0 as usize] = true;
        }

        if x0 == x1 && y0 == y1 {
            break;
        }

        let e2 = 2 * err;
        if e2 >= dy {
            err += dy;
            x0 += sx;
        }
        if e2 <= dx {
            err += dx;
            y0 += sy;
        }
    }
}
