//! Stroke PNG rasterizer — converts freehand pen strokes into a small PNG
//! image (base64-encoded) for vision-capable LLM prompts.
//!
//! Uses the `freehand` module (Rust port of `perfect-freehand`) to generate
//! pressure-sensitive outline polygons, then fills them with scanline raster.
//! This produces smooth, variable-width strokes matching the frontend rendering.
//!
//! # Output
//!
//! Returns a base64-encoded PNG string ready for inclusion in an LLM image
//! content block. The image is:
//!
//! - Cropped to the tight bounding box of the strokes (not the node bounds)
//! - Scaled so the longest side fits within `max_side` pixels
//! - Black strokes on white background (grayscale)

use base64::Engine;
use image::{GrayImage, Luma};

use super::freehand;
use super::freehand::types::StrokeOptions;
use super::types::CanvasBounds;

// ============================================================================
// Public API
// ============================================================================

/// Rasterize pen strokes into a base64-encoded PNG.
///
/// Strokes are `[x, y, pressure]` triples in absolute canvas coordinates.
/// The image is cropped to the tight bounding box, scaled so the longest
/// side equals `max_side`, and encoded as a grayscale PNG.
///
/// Returns `None` if `strokes` is empty or no stroke points exist.
pub(crate) fn rasterize_strokes_png(
    strokes: &[Vec<[f64; 3]>],
    _bounds: &CanvasBounds,
    max_side: u32,
    padding: u32,
    stroke_size: u32,
) -> Option<String> {
    if strokes.is_empty() {
        return None;
    }

    // Compute tight bounding box of all stroke points
    let bbox = stroke_bbox(strokes)?;
    let bbox_w = (bbox.max_x - bbox.min_x).max(1.0);
    let bbox_h = (bbox.max_y - bbox.min_y).max(1.0);

    // Scale so longest side = max_side (minus padding on both sides)
    let usable = max_side.saturating_sub(padding * 2).max(1) as f64;
    let longest = bbox_w.max(bbox_h);
    let scale = usable / longest;

    let img_w = (bbox_w * scale).ceil() as u32 + padding * 2;
    let img_h = (bbox_h * scale).ceil() as u32 + padding * 2;

    if img_w == 0 || img_h == 0 {
        return None;
    }

    let mut img = GrayImage::from_pixel(img_w, img_h, Luma([255u8]));

    // Detect if any real pressure data exists (not all ~0.5)
    let has_real_pressure = strokes.iter().any(|s| {
        s.iter().any(|p| (p[2] - 0.5).abs() > 0.01)
    });

    let options = StrokeOptions {
        size: stroke_size as f64,
        simulate_pressure: !has_real_pressure,
        ..StrokeOptions::default()
    };

    for stroke in strokes {
        if stroke.is_empty() {
            continue;
        }

        // Generate freehand outline polygon
        let outline = freehand::get_stroke(stroke, &options);

        if outline.len() < 3 {
            // Degenerate: just draw a dot
            if let Some(pt) = stroke.first() {
                let (px, py) = to_pixel(pt[0], pt[1], &bbox, scale, padding);
                fill_circle(&mut img, px, py, stroke_size);
            }
            continue;
        }

        // Smooth the outline with quadratic bezier subdivision (matches frontend's
        // quadraticCurveTo midpoint algorithm from outlineToPath.ts)
        let smoothed = subdivide_outline(&outline, 8);

        // Convert outline to pixel coordinates
        let pixel_polygon: Vec<(i32, i32)> = smoothed
            .iter()
            .map(|p| to_pixel(p[0], p[1], &bbox, scale, padding))
            .collect();

        // Fill the polygon using scanline (even-odd rule)
        scanline_fill(&mut img, &pixel_polygon, img_w, img_h);
    }

    // Encode to PNG
    let mut png_bytes: Vec<u8> = Vec::new();
    let encoder = image::codecs::png::PngEncoder::new(&mut png_bytes);
    image::ImageEncoder::write_image(
        encoder,
        img.as_raw(),
        img_w,
        img_h,
        image::ExtendedColorType::L8,
    )
    .ok()?;

    if std::env::var("DEBUG_RASTERIZE").is_ok() {
        if let Err(e) = std::fs::write("/tmp/debug_rasterize.png", &png_bytes) {
            eprintln!("Failed to write debug PNG: {}", e);
        } else {
            eprintln!("DEBUG: saved rasterized PNG to /tmp/debug_rasterize.png");
        }
    }

    Some(base64::engine::general_purpose::STANDARD.encode(&png_bytes))
}

// ============================================================================
// Bounding Box
// ============================================================================

struct StrokeBBox {
    min_x: f64,
    min_y: f64,
    max_x: f64,
    max_y: f64,
}

/// Compute tight bounding box of all stroke points.
fn stroke_bbox(strokes: &[Vec<[f64; 3]>]) -> Option<StrokeBBox> {
    let mut min_x = f64::MAX;
    let mut min_y = f64::MAX;
    let mut max_x = f64::MIN;
    let mut max_y = f64::MIN;
    let mut any = false;

    for stroke in strokes {
        for pt in stroke {
            any = true;
            min_x = min_x.min(pt[0]);
            min_y = min_y.min(pt[1]);
            max_x = max_x.max(pt[0]);
            max_y = max_y.max(pt[1]);
        }
    }

    if !any {
        return None;
    }

    Some(StrokeBBox {
        min_x,
        min_y,
        max_x,
        max_y,
    })
}

// ============================================================================
// Coordinate Mapping
// ============================================================================

/// Map an absolute canvas point to pixel coordinates.
fn to_pixel(x: f64, y: f64, bbox: &StrokeBBox, scale: f64, padding: u32) -> (i32, i32) {
    let px = ((x - bbox.min_x) * scale + padding as f64).round() as i32;
    let py = ((y - bbox.min_y) * scale + padding as f64).round() as i32;
    (px, py)
}

// ============================================================================
// Outline Smoothing
// ============================================================================

/// Subdivide an outline polygon using quadratic bezier midpoint interpolation.
///
/// Matches the frontend's `outlineToPath.ts` which uses `quadraticCurveTo`
/// with midpoints between consecutive outline points as an open path:
///
///   moveTo(outline[0])
///   for i in 1..n-1: quadraticCurveTo(outline[i], mid(outline[i], outline[i+1]))
///   lineTo(outline[n-1])
///   closePath()   // implicit straight line back to start
fn subdivide_outline(outline: &[[f64; 2]], segments_per_curve: usize) -> Vec<[f64; 2]> {
    if outline.len() < 3 {
        return outline.to_vec();
    }

    let n = outline.len();
    let mut result = Vec::with_capacity(n * segments_per_curve);

    // Start at outline[0] (the moveTo)
    let mut cursor = outline[0];
    result.push(cursor);

    // For i = 1..n-2: quadratic bezier from cursor through outline[i] to midpoint
    for i in 1..n - 1 {
        let ctrl = outline[i];
        let next = outline[i + 1];
        let end = [(ctrl[0] + next[0]) * 0.5, (ctrl[1] + next[1]) * 0.5];

        // Subdivide quadratic bezier: cursor -> ctrl -> end
        for s in 1..=segments_per_curve {
            let t = s as f64 / segments_per_curve as f64;
            let inv = 1.0 - t;
            let x = inv * inv * cursor[0] + 2.0 * inv * t * ctrl[0] + t * t * end[0];
            let y = inv * inv * cursor[1] + 2.0 * inv * t * ctrl[1] + t * t * end[1];
            result.push([x, y]);
        }

        cursor = end;
    }

    // lineTo last point (closePath back to start is implicit for scanline fill)
    result.push(outline[n - 1]);

    result
}

// ============================================================================
// Polygon Fill (scanline, even-odd rule)
// ============================================================================

/// Fill a closed polygon on the image using scanline rendering (non-zero winding rule).
///
/// Non-zero winding correctly fills self-intersecting polygons (like freehand
/// outlines with caps and sharp corner arcs) without punching holes.
fn scanline_fill(img: &mut GrayImage, polygon: &[(i32, i32)], img_w: u32, img_h: u32) {
    if polygon.len() < 3 {
        return;
    }

    // Find Y bounds
    let min_y = polygon.iter().map(|p| p.1).min().unwrap().max(0);
    let max_y = polygon
        .iter()
        .map(|p| p.1)
        .max()
        .unwrap()
        .min(img_h as i32 - 1);

    let n = polygon.len();

    for y in min_y..=max_y {
        // Collect X intersections with winding direction
        let mut crossings: Vec<(i32, i32)> = Vec::new(); // (x, direction)

        for i in 0..n {
            let (x0, y0) = polygon[i];
            let (x1, y1) = polygon[(i + 1) % n];

            // Skip horizontal edges
            if y0 == y1 {
                continue;
            }

            // Check if scanline intersects this edge
            let (lo, hi) = if y0 < y1 { (y0, y1) } else { (y1, y0) };
            if y < lo || y >= hi {
                continue;
            }

            // Compute X intersection
            let x = x0 as i64 + (y - y0) as i64 * (x1 - x0) as i64 / (y1 - y0) as i64;

            // Winding direction: +1 if edge goes up, -1 if edge goes down
            let dir = if y0 < y1 { 1 } else { -1 };
            crossings.push((x as i32, dir));
        }

        crossings.sort_unstable_by_key(|c| c.0);

        // Fill using non-zero winding rule: pixel is inside when winding != 0
        let mut winding: i32 = 0;
        let mut i = 0;
        while i < crossings.len() {
            let prev_winding = winding;
            winding += crossings[i].1;

            // Transition from outside (0) to inside (non-zero): start filling
            if prev_winding == 0 && winding != 0 {
                let xa = crossings[i].0.max(0);
                // Scan forward to find where winding returns to 0
                let mut j = i + 1;
                while j < crossings.len() {
                    winding += crossings[j].1;
                    if winding == 0 {
                        let xb = crossings[j].0.min(img_w as i32 - 1);
                        for x in xa..=xb {
                            img.put_pixel(x as u32, y as u32, Luma([0u8]));
                        }
                        i = j + 1;
                        break;
                    }
                    j += 1;
                }
                if winding != 0 {
                    // Didn't close — fill to last crossing
                    let xb = crossings.last().unwrap().0.min(img_w as i32 - 1);
                    for x in xa..=xb {
                        img.put_pixel(x as u32, y as u32, Luma([0u8]));
                    }
                    break;
                }
            } else {
                i += 1;
            }
        }
    }
}

// ============================================================================
// Drawing (fallback for degenerate strokes)
// ============================================================================

/// Fill a circle of `diameter` pixels centered at (cx, cy) with black.
fn fill_circle(img: &mut GrayImage, cx: i32, cy: i32, diameter: u32) {
    let r = diameter as i32 / 2;
    let (w, h) = (img.width() as i32, img.height() as i32);

    for dy in -r..=r {
        for dx in -r..=r {
            if dx * dx + dy * dy <= r * r {
                let px = cx + dx;
                let py = cy + dy;
                if px >= 0 && px < w && py >= 0 && py < h {
                    img.put_pixel(px as u32, py as u32, Luma([0u8]));
                }
            }
        }
    }
}

#[cfg(test)]
#[path = "rasterize_png_tests.rs"]
mod tests;
