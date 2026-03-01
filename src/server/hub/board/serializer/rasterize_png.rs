//! Stroke PNG rasterizer — converts freehand pen strokes into a small PNG
//! image (base64-encoded) for vision-capable LLM prompts.
//!
//! Uses the `freehand` module (Rust port of `perfect-freehand`) to generate
//! pressure-sensitive outline polygons, then fills them with `tiny-skia`.
//! This produces smooth, anti-aliased, variable-width strokes that match
//! the frontend rendering exactly (numerically verified).
//!
//! # Output
//!
//! Returns a base64-encoded PNG string ready for inclusion in an LLM image
//! content block. The image is:
//!
//! - Cropped to the tight bounding box of the outline points (not raw input)
//! - Scaled so the longest side fits within `max_side` pixels
//! - Black strokes on white background (grayscale)

use base64::Engine;
use image::{GrayImage, Luma};
use tiny_skia::{FillRule, Paint, PathBuilder, Pixmap, Transform};

use super::freehand;
use super::freehand::types::{StrokeOptions, Vec2};
use super::types::CanvasBounds;

/// Stroke size in canvas coordinates — must match the frontend's
/// `getStrokeOutline.ts` default `size: 5`.
const CANVAS_STROKE_SIZE: f64 = 5.0;

// ============================================================================
// Public API
// ============================================================================

/// Intermediate result from pass 1: either a full outline polygon
/// or a degenerate dot (fewer than 3 outline points).
enum StrokeOutline {
    Polygon(Vec<Vec2>),
    Dot { cx: f64, cy: f64, radius: f64 },
}

/// Rasterize pen strokes into a base64-encoded PNG.
///
/// Strokes are `[x, y, pressure]` triples in absolute canvas coordinates.
/// The freehand algorithm runs in canvas space (matching the frontend),
/// then outlines are scaled to pixel space for rendering.
///
/// Uses a two-pass approach:
/// 1. Generate all freehand outlines in canvas space
/// 2. Compute bbox from outline points (includes stroke width, caps, corners)
/// 3. Scale outlines to pixel space and render
///
/// Returns `None` if `strokes` is empty or no outline points exist.
pub(crate) fn rasterize_strokes_png(
    strokes: &[Vec<[f64; 3]>],
    _bounds: &CanvasBounds,
    max_side: u32,
    padding: u32,
) -> Option<String> {
    if strokes.is_empty() {
        return None;
    }

    // Detect if any real pressure data exists.
    // Match frontend exactly: JS uses strict !== 0.5 (no epsilon).
    let has_real_pressure = strokes.iter().any(|s| s.iter().any(|p| p[2] != 0.5));

    let options = StrokeOptions {
        size: CANVAS_STROKE_SIZE,
        simulate_pressure: !has_real_pressure,
        ..StrokeOptions::default()
    };

    let dot_radius = CANVAS_STROKE_SIZE * 0.5;

    // ── Pass 1: Generate all outlines in canvas space ──────────────
    let mut outlines: Vec<StrokeOutline> = Vec::with_capacity(strokes.len());

    for stroke in strokes {
        if stroke.is_empty() {
            continue;
        }

        let outline = freehand::get_stroke(stroke, &options);

        if outline.len() < 3 {
            if let Some(pt) = stroke.first() {
                outlines.push(StrokeOutline::Dot {
                    cx: pt[0],
                    cy: pt[1],
                    radius: dot_radius,
                });
            }
        } else {
            outlines.push(StrokeOutline::Polygon(outline));
        }
    }

    if outlines.is_empty() {
        return None;
    }

    // ── Compute bbox from OUTLINE points (not input points) ────────
    let bbox = outline_bbox(&outlines)?;
    let bbox_w = (bbox.max_x - bbox.min_x).max(1.0);
    let bbox_h = (bbox.max_y - bbox.min_y).max(1.0);

    let usable = max_side.saturating_sub(padding * 2).max(1) as f64;
    let longest = bbox_w.max(bbox_h);
    let scale = usable / longest;

    let img_w = (bbox_w * scale).ceil() as u32 + padding * 2;
    let img_h = (bbox_h * scale).ceil() as u32 + padding * 2;

    if img_w == 0 || img_h == 0 {
        return None;
    }

    // ── Pass 2: Scale outlines to pixel space and render ───────────
    let mut pixmap = Pixmap::new(img_w, img_h)?;
    pixmap.fill(tiny_skia::Color::WHITE);

    let mut paint = Paint::default();
    paint.set_color_rgba8(0, 0, 0, 255);
    paint.anti_alias = true;

    let pad = padding as f64;

    for outline in &outlines {
        match outline {
            StrokeOutline::Polygon(pts) => {
                let px_outline: Vec<[f64; 2]> = pts
                    .iter()
                    .map(|p| {
                        [
                            (p[0] - bbox.min_x) * scale + pad,
                            (p[1] - bbox.min_y) * scale + pad,
                        ]
                    })
                    .collect();

                if let Some(path) = build_outline_path(&px_outline) {
                    pixmap.fill_path(
                        &path,
                        &paint,
                        FillRule::Winding,
                        Transform::identity(),
                        None,
                    );
                }
            }
            StrokeOutline::Dot { cx, cy, radius } => {
                let px = ((cx - bbox.min_x) * scale + pad) as f32;
                let py = ((cy - bbox.min_y) * scale + pad) as f32;
                let r = (radius * scale) as f32;
                if let Some(path) = PathBuilder::from_circle(px, py, r) {
                    pixmap.fill_path(
                        &path,
                        &paint,
                        FillRule::Winding,
                        Transform::identity(),
                        None,
                    );
                }
            }
        }
    }

    // Convert RGBA pixmap to grayscale PNG
    let gray = rgba_to_grayscale(pixmap.data(), img_w, img_h);

    let mut png_bytes: Vec<u8> = Vec::new();
    let encoder = image::codecs::png::PngEncoder::new(&mut png_bytes);
    image::ImageEncoder::write_image(
        encoder,
        gray.as_raw(),
        img_w,
        img_h,
        image::ExtendedColorType::L8,
    )
    .ok()?;

    if std::env::var("DEBUG_RASTERIZE").is_ok() {
        use std::sync::atomic::{AtomicUsize, Ordering};
        static COUNTER: AtomicUsize = AtomicUsize::new(0);
        let n = COUNTER.fetch_add(1, Ordering::Relaxed);
        let path = format!("/tmp/debug_rasterize_{n}.png");
        if let Err(e) = std::fs::write(&path, &png_bytes) {
            eprintln!("Failed to write debug PNG: {e}");
        } else {
            eprintln!("DEBUG: saved rasterized PNG to {path}");
        }
        eprintln!(
            "DEBUG: input={} strokes, rendered={} outlines ({}x{} px, scale={:.2})",
            strokes.len(),
            outlines.len(),
            img_w,
            img_h,
            scale
        );
        for (i, stroke) in strokes.iter().enumerate() {
            eprintln!("DEBUG:   stroke[{i}]: {} points", stroke.len());
        }
    }

    Some(base64::engine::general_purpose::STANDARD.encode(&png_bytes))
}

// ============================================================================
// Path Building
// ============================================================================

/// Build a tiny-skia path from a freehand outline polygon.
///
/// Matches the frontend's `outlineToPath.ts` algorithm:
///   moveTo(outline[0])
///   for i in 1..n-1: quadraticCurveTo(outline[i], mid(outline[i], outline[i+1]))
///   lineTo(outline[n-1])
///   closePath()
fn build_outline_path(outline: &[[f64; 2]]) -> Option<tiny_skia::Path> {
    let n = outline.len();
    if n < 3 {
        return None;
    }

    let mut pb = PathBuilder::new();
    pb.move_to(outline[0][0] as f32, outline[0][1] as f32);

    for i in 1..n - 1 {
        let cx = outline[i][0] as f32;
        let cy = outline[i][1] as f32;
        let mid_x = ((outline[i][0] + outline[i + 1][0]) * 0.5) as f32;
        let mid_y = ((outline[i][1] + outline[i + 1][1]) * 0.5) as f32;
        pb.quad_to(cx, cy, mid_x, mid_y);
    }

    pb.line_to(outline[n - 1][0] as f32, outline[n - 1][1] as f32);
    pb.close();
    pb.finish()
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

/// Compute tight bounding box from all outline points and dot extents.
fn outline_bbox(outlines: &[StrokeOutline]) -> Option<StrokeBBox> {
    let mut min_x = f64::MAX;
    let mut min_y = f64::MAX;
    let mut max_x = f64::MIN;
    let mut max_y = f64::MIN;
    let mut any = false;

    for outline in outlines {
        match outline {
            StrokeOutline::Polygon(pts) => {
                for p in pts {
                    any = true;
                    min_x = min_x.min(p[0]);
                    min_y = min_y.min(p[1]);
                    max_x = max_x.max(p[0]);
                    max_y = max_y.max(p[1]);
                }
            }
            StrokeOutline::Dot { cx, cy, radius } => {
                any = true;
                min_x = min_x.min(cx - radius);
                min_y = min_y.min(cy - radius);
                max_x = max_x.max(cx + radius);
                max_y = max_y.max(cy + radius);
            }
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
// Grayscale Conversion
// ============================================================================

/// Convert RGBA pixel data to grayscale.
///
/// Uses luminance: L = 0.299R + 0.587G + 0.114B, composited over white.
fn rgba_to_grayscale(rgba: &[u8], width: u32, height: u32) -> GrayImage {
    let mut gray = GrayImage::from_pixel(width, height, Luma([255u8]));

    for y in 0..height {
        for x in 0..width {
            let idx = ((y * width + x) * 4) as usize;
            let r = rgba[idx] as f32;
            let g = rgba[idx + 1] as f32;
            let b = rgba[idx + 2] as f32;
            let a = rgba[idx + 3] as f32 / 255.0;

            let lum = 0.299 * r + 0.587 * g + 0.114 * b;
            let val = (lum * a + 255.0 * (1.0 - a)).round() as u8;
            gray.put_pixel(x, y, Luma([val]));
        }
    }

    gray
}

#[cfg(test)]
#[path = "rasterize_png_tests.rs"]
mod tests;
