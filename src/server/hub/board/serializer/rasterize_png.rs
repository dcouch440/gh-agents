//! Stroke PNG rasterizer — converts freehand pen strokes into a small PNG
//! image (base64-encoded) for vision-capable LLM prompts.
//!
//! Companion to `rasterize.rs` (ASCII grid) and `encode.rs` (JSON coords).
//! Reuses the same Bresenham line algorithm but outputs pixels instead of
//! characters, producing significantly better recognition by vision models.
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

use super::types::CanvasBounds;

// ============================================================================
// Public API
// ============================================================================

/// Rasterize pen strokes into a base64-encoded PNG.
///
/// Strokes are given as absolute canvas coordinates. The image is cropped to
/// the tight bounding box of the strokes (not the node's `bounds`), scaled
/// so the longest side equals `max_side`, and encoded as a grayscale PNG.
///
/// Returns `None` if `strokes` is empty, `bounds` has zero area, or no
/// stroke points exist.
pub(crate) fn rasterize_strokes_png(
    strokes: &[Vec<[f64; 2]>],
    _bounds: &CanvasBounds,
    max_side: u32,
    padding: u32,
    stroke_width: u32,
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

    for stroke in strokes {
        if stroke.len() == 1 {
            let (px, py) = to_pixel(stroke[0][0], stroke[0][1], &bbox, scale, padding);
            fill_circle(&mut img, px, py, stroke_width);
            continue;
        }

        for pair in stroke.windows(2) {
            let (x0, y0) = to_pixel(pair[0][0], pair[0][1], &bbox, scale, padding);
            let (x1, y1) = to_pixel(pair[1][0], pair[1][1], &bbox, scale, padding);
            bresenham_thick(&mut img, x0, y0, x1, y1, stroke_width);
        }
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
fn stroke_bbox(strokes: &[Vec<[f64; 2]>]) -> Option<StrokeBBox> {
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
// Drawing
// ============================================================================

/// Draw a thick line from (x0,y0) to (x1,y1) using Bresenham's algorithm.
///
/// For each pixel on the line, fills a circle of `width` diameter to produce
/// even thickness.
fn bresenham_thick(img: &mut GrayImage, x0: i32, y0: i32, x1: i32, y1: i32, width: u32) {
    let mut cx = x0;
    let mut cy = y0;

    let dx = (x1 - x0).abs();
    let dy = -(y1 - y0).abs();
    let sx: i32 = if x0 < x1 { 1 } else { -1 };
    let sy: i32 = if y0 < y1 { 1 } else { -1 };
    let mut err = dx + dy;

    loop {
        fill_circle(img, cx, cy, width);

        if cx == x1 && cy == y1 {
            break;
        }

        let e2 = 2 * err;
        if e2 >= dy {
            err += dy;
            cx += sx;
        }
        if e2 <= dx {
            err += dx;
            cy += sy;
        }
    }
}

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
