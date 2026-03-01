//! Stroke PNG rasterizer — converts freehand pen strokes into a small PNG
//! image (base64-encoded) for vision-capable LLM prompts.
//!
//! Renders strokes as simple polylines (connecting raw input points with
//! straight line segments). This preserves the exact geometry the user
//! drew — sharp corners, straight edges — which is critical for LLM
//! shape recognition. The freehand smoothing used in the frontend
//! rendering is intentionally skipped: it rounds corners and softens
//! edges, making shapes harder for an LLM to interpret.
//!
//! # Output
//!
//! Returns a base64-encoded PNG string ready for inclusion in an LLM image
//! content block. The image is:
//!
//! - Cropped to the tight bounding box of the strokes (plus stroke width)
//! - Scaled so the longest side fits within `max_side` pixels
//! - Black strokes on white background (grayscale)

use base64::Engine;
use image::{GrayImage, Luma};
use tiny_skia::{LineCap, LineJoin, Paint, PathBuilder, Pixmap, Stroke, Transform};

use super::types::CanvasBounds;

/// Stroke width in canvas coordinates — matches the frontend's
/// `getStrokeOutline.ts` default `size: 5`.
const CANVAS_STROKE_SIZE: f64 = 5.0;

// ============================================================================
// Public API
// ============================================================================

/// Rasterize pen strokes into a base64-encoded PNG.
///
/// Strokes are `[x, y, pressure]` triples in absolute canvas coordinates.
/// Renders as simple polylines connecting raw input points — no freehand
/// smoothing — to preserve sharp corners for LLM shape recognition.
///
/// The image is cropped to the tight bounding box of the strokes (expanded
/// by half the stroke width), scaled so the longest side equals `max_side`,
/// and encoded as a grayscale PNG.
///
/// Returns `None` if `strokes` is empty or no stroke points exist.
pub(crate) fn rasterize_strokes_png(
    strokes: &[Vec<[f64; 3]>],
    _bounds: &CanvasBounds,
    max_side: u32,
    padding: u32,
) -> Option<String> {
    if strokes.is_empty() {
        return None;
    }

    // Filter out empty strokes
    let non_empty: Vec<&Vec<[f64; 3]>> = strokes.iter().filter(|s| !s.is_empty()).collect();
    if non_empty.is_empty() {
        return None;
    }

    // Compute bbox from raw input points, expanded by half stroke width
    let half_stroke = CANVAS_STROKE_SIZE * 0.5;
    let bbox = stroke_bbox(&non_empty, half_stroke)?;
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

    // Render strokes as polylines
    let mut pixmap = Pixmap::new(img_w, img_h)?;
    pixmap.fill(tiny_skia::Color::WHITE);

    let mut paint = Paint::default();
    paint.set_color_rgba8(0, 0, 0, 255); // black
    paint.anti_alias = true;

    let mut stroke_style = Stroke::default();
    stroke_style.width = (CANVAS_STROKE_SIZE * scale) as f32;
    stroke_style.line_cap = LineCap::Round;
    stroke_style.line_join = LineJoin::Round;

    let pad = padding as f64;

    for stroke in &non_empty {
        if stroke.len() == 1 {
            // Single point — draw a filled circle
            let px = ((stroke[0][0] - bbox.min_x) * scale + pad) as f32;
            let py = ((stroke[0][1] - bbox.min_y) * scale + pad) as f32;
            let r = stroke_style.width * 0.5;
            if let Some(path) = PathBuilder::from_circle(px, py, r) {
                paint.anti_alias = true;
                pixmap.fill_path(
                    &path,
                    &paint,
                    tiny_skia::FillRule::Winding,
                    Transform::identity(),
                    None,
                );
            }
            continue;
        }

        let mut pb = PathBuilder::new();
        let x0 = ((stroke[0][0] - bbox.min_x) * scale + pad) as f32;
        let y0 = ((stroke[0][1] - bbox.min_y) * scale + pad) as f32;
        pb.move_to(x0, y0);

        for pt in &stroke[1..] {
            let px = ((pt[0] - bbox.min_x) * scale + pad) as f32;
            let py = ((pt[1] - bbox.min_y) * scale + pad) as f32;
            pb.line_to(px, py);
        }

        if let Some(path) = pb.finish() {
            pixmap.stroke_path(&path, &paint, &stroke_style, Transform::identity(), None);
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
            "DEBUG: input={} strokes, rendered={} polylines ({}x{} px, scale={:.2})",
            strokes.len(),
            non_empty.len(),
            img_w,
            img_h,
            scale
        );
        for (i, stroke) in strokes.iter().enumerate() {
            eprintln!("DEBUG:   stroke[{i}]: {} points", stroke.len());
        }
        // Dump stroke data as JSON for JS comparison
        let json_data: Vec<Vec<[f64; 3]>> = strokes.to_vec();
        if let Ok(json) = serde_json::to_string(&json_data) {
            let _ = std::fs::write("/tmp/debug_strokes.json", &json);
            eprintln!("DEBUG: saved stroke data to /tmp/debug_strokes.json");
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

/// Compute bounding box from raw input points, expanded by `expand` on all sides.
fn stroke_bbox(strokes: &[&Vec<[f64; 3]>], expand: f64) -> Option<StrokeBBox> {
    let mut min_x = f64::MAX;
    let mut min_y = f64::MAX;
    let mut max_x = f64::MIN;
    let mut max_y = f64::MIN;
    let mut any = false;

    for stroke in strokes {
        for p in stroke.iter() {
            any = true;
            min_x = min_x.min(p[0]);
            min_y = min_y.min(p[1]);
            max_x = max_x.max(p[0]);
            max_y = max_y.max(p[1]);
        }
    }

    if !any {
        return None;
    }

    Some(StrokeBBox {
        min_x: min_x - expand,
        min_y: min_y - expand,
        max_x: max_x + expand,
        max_y: max_y + expand,
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

            // Composite over white: result = src * alpha + white * (1 - alpha)
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
