//! Visual test bench — generates test PNGs to /tmp for manual inspection.
//!
//! Run with: cargo test --lib -- freehand::visual_test --nocapture
//! Then open: open /tmp/freehand_test_*.png

use image::{GrayImage, Luma};

use super::types::{StrokeOptions, Vec2};
use super::{get_stroke, stroke_points::get_stroke_points};

/// Render a set of strokes (each as [x,y,pressure] triples) to a PNG file.
fn render_strokes_to_png(
    strokes: &[Vec<[f64; 3]>],
    path: &str,
    img_size: u32,
    options: &StrokeOptions,
) {
    let padding = 20u32;
    let mut img = GrayImage::from_pixel(img_size, img_size, Luma([255u8]));

    // Compute bbox
    let (mut min_x, mut min_y, mut max_x, mut max_y) =
        (f64::MAX, f64::MAX, f64::MIN, f64::MIN);
    for stroke in strokes {
        for p in stroke {
            min_x = min_x.min(p[0]);
            min_y = min_y.min(p[1]);
            max_x = max_x.max(p[0]);
            max_y = max_y.max(p[1]);
        }
    }

    let bbox_w = (max_x - min_x).max(1.0);
    let bbox_h = (max_y - min_y).max(1.0);
    let usable = (img_size - padding * 2) as f64;
    let scale = usable / bbox_w.max(bbox_h);

    for stroke in strokes {
        let outline = get_stroke(stroke, options);
        if outline.len() < 3 {
            continue;
        }

        // Subdivide the outline for smooth curves
        let smoothed = subdivide_outline(&outline);

        // Convert to pixel coords
        let pixels: Vec<(i32, i32)> = smoothed
            .iter()
            .map(|p| {
                let px = ((p[0] - min_x) * scale + padding as f64).round() as i32;
                let py = ((p[1] - min_y) * scale + padding as f64).round() as i32;
                (px, py)
            })
            .collect();

        // Scanline fill (non-zero winding)
        scanline_fill_nz(&mut img, &pixels);
    }

    img.save(path).expect("failed to save PNG");
    eprintln!("Saved: {}", path);
}

fn subdivide_outline(outline: &[Vec2]) -> Vec<Vec2> {
    if outline.len() < 3 {
        return outline.to_vec();
    }
    let n = outline.len();
    let segs = 8;
    let mut result = Vec::with_capacity(n * segs);

    // Start at outline[0] (matches frontend moveTo)
    let mut cursor = outline[0];
    result.push(cursor);

    // Quadratic bezier curves from point 1 to n-2
    for i in 1..n - 1 {
        let ctrl = outline[i];
        let next = outline[i + 1];
        let end = [(ctrl[0] + next[0]) * 0.5, (ctrl[1] + next[1]) * 0.5];

        for s in 1..=segs {
            let t = s as f64 / segs as f64;
            let inv = 1.0 - t;
            let x = inv * inv * cursor[0] + 2.0 * inv * t * ctrl[0] + t * t * end[0];
            let y = inv * inv * cursor[1] + 2.0 * inv * t * ctrl[1] + t * t * end[1];
            result.push([x, y]);
        }

        cursor = end;
    }

    // lineTo last point (scanline fill implicitly closes back to start)
    result.push(outline[n - 1]);

    result
}

fn scanline_fill_nz(img: &mut GrayImage, polygon: &[(i32, i32)]) {
    if polygon.len() < 3 {
        return;
    }
    let (w, h) = (img.width() as i32, img.height() as i32);
    let min_y = polygon.iter().map(|p| p.1).min().unwrap().max(0);
    let max_y = polygon.iter().map(|p| p.1).max().unwrap().min(h - 1);
    let n = polygon.len();

    for y in min_y..=max_y {
        let mut crossings: Vec<(i32, i32)> = Vec::new();
        for i in 0..n {
            let (x0, y0) = polygon[i];
            let (x1, y1) = polygon[(i + 1) % n];
            if y0 == y1 {
                continue;
            }
            let (lo, hi) = if y0 < y1 { (y0, y1) } else { (y1, y0) };
            if y < lo || y >= hi {
                continue;
            }
            let x = x0 as i64 + (y - y0) as i64 * (x1 - x0) as i64 / (y1 - y0) as i64;
            let dir = if y0 < y1 { 1 } else { -1 };
            crossings.push((x as i32, dir));
        }
        crossings.sort_unstable_by_key(|c| c.0);

        let mut winding: i32 = 0;
        let mut i = 0;
        while i < crossings.len() {
            let prev_w = winding;
            winding += crossings[i].1;
            if prev_w == 0 && winding != 0 {
                let xa = crossings[i].0.max(0);
                let mut j = i + 1;
                while j < crossings.len() {
                    winding += crossings[j].1;
                    if winding == 0 {
                        let xb = crossings[j].0.min(w - 1);
                        for x in xa..=xb {
                            img.put_pixel(x as u32, y as u32, Luma([0u8]));
                        }
                        i = j + 1;
                        break;
                    }
                    j += 1;
                }
                if winding != 0 {
                    let xb = crossings.last().unwrap().0.min(w - 1);
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
// Comparison test — dumps Rust output + generates HTML to run JS side-by-side
// ============================================================================

#[test]
fn compare_with_js() {
    // Simple L-shaped stroke with uniform pressure
    let input: Vec<[f64; 3]> = vec![
        [0.0, 0.0, 0.5],
        [10.0, 0.0, 0.5],
        [20.0, 0.0, 0.5],
        [30.0, 0.0, 0.5],
        [40.0, 0.0, 0.5],
        [50.0, 0.0, 0.5],
        [60.0, 0.0, 0.5],
        [70.0, 0.0, 0.5],
        [80.0, 0.0, 0.5],
        [90.0, 0.0, 0.5],
        [100.0, 0.0, 0.5],
        [100.0, 10.0, 0.5],
        [100.0, 20.0, 0.5],
        [100.0, 30.0, 0.5],
        [100.0, 40.0, 0.5],
        [100.0, 50.0, 0.5],
        [100.0, 60.0, 0.5],
        [100.0, 70.0, 0.5],
        [100.0, 80.0, 0.5],
        [100.0, 90.0, 0.5],
        [100.0, 100.0, 0.5],
    ];

    let options = StrokeOptions {
        size: 16.0,
        ..StrokeOptions::default()
    };

    let points = get_stroke_points(&input, &options);
    let outline = get_stroke(&input, &options);

    eprintln!("\n=== RUST PORT OUTPUT ===");
    eprintln!("Input: {} points", input.len());
    eprintln!("Stroke points: {}", points.len());
    for (i, pt) in points.iter().enumerate() {
        eprintln!(
            "  [{:2}] ({:8.3}, {:8.3}) p={:.3} d={:.3} rl={:.3} v=({:.3},{:.3})",
            i, pt.point[0], pt.point[1], pt.pressure, pt.distance, pt.running_length,
            pt.vector[0], pt.vector[1]
        );
    }

    eprintln!("\nOutline: {} points", outline.len());
    eprintln!("Outline (first 20): {:?}", &outline[..outline.len().min(20)]);

    // Generate HTML comparison file
    let input_json: Vec<String> = input.iter().map(|p| format!("[{},{},{}]", p[0], p[1], p[2])).collect();
    let outline_json: Vec<String> = outline.iter().map(|p| format!("[{:.2},{:.2}]", p[0], p[1])).collect();

    let html = format!(r#"<!DOCTYPE html>
<html><body>
<h2>Left=JS (perfect-freehand), Right=Rust port</h2>
<canvas id="js" width="300" height="300" style="border:1px solid #ccc"></canvas>
<canvas id="rust" width="300" height="300" style="border:1px solid #ccc"></canvas>
<pre id="log"></pre>
<script type="module">
import getStroke from 'https://esm.sh/perfect-freehand@1.2.2';

const input = [{}];
const rustOutline = [{}];

// Run JS library
const jsOutline = getStroke(input, {{ size: 16, thinning: 0.5, smoothing: 0.5, streamline: 0.5, simulatePressure: true }});

document.getElementById('log').textContent =
  'JS outline: ' + jsOutline.length + ' points\n' +
  'Rust outline: ' + rustOutline.length + ' points\n' +
  'JS first 5: ' + JSON.stringify(jsOutline.slice(0, 5).map(p => [+p[0].toFixed(2), +p[1].toFixed(2)])) + '\n' +
  'Rust first 5: ' + JSON.stringify(rustOutline.slice(0, 5));

function draw(canvasId, outline, color) {{
  const ctx = document.getElementById(canvasId).getContext('2d');
  ctx.translate(50, 50);
  ctx.scale(1.5, 1.5);
  if (outline.length < 3) return;
  ctx.beginPath();
  ctx.moveTo(outline[0][0], outline[0][1]);
  for (let i = 1; i < outline.length - 1; i++) {{
    const curr = outline[i];
    const next = outline[i + 1];
    ctx.quadraticCurveTo(curr[0], curr[1], (curr[0]+next[0])/2, (curr[1]+next[1])/2);
  }}
  ctx.lineTo(outline[outline.length-1][0], outline[outline.length-1][1]);
  ctx.closePath();
  ctx.fillStyle = color;
  ctx.fill();
}}

draw('js', jsOutline, '#000');
draw('rust', rustOutline, '#000');
</script>
</body></html>"#, input_json.join(","), outline_json.join(","));

    std::fs::write("/tmp/freehand_compare.html", html).expect("write html");
    eprintln!("\nComparison HTML: open /tmp/freehand_compare.html");
}

// ============================================================================
// Visual tests
// ============================================================================

#[test]
fn visual_p_curve() {
    // A "P" shape: vertical line + curved top
    let options = StrokeOptions {
        size: 5.0,
        ..StrokeOptions::default()
    };

    // Draw a P: down-stroke, then a curve at the top
    let p_stroke: Vec<[f64; 3]> = vec![
        [50.0, 200.0, 0.5],
        [50.0, 180.0, 0.5],
        [50.0, 160.0, 0.5],
        [50.0, 140.0, 0.5],
        [50.0, 120.0, 0.5],
        [50.0, 100.0, 0.5],
        [50.0, 80.0, 0.5],
        [50.0, 60.0, 0.5],
        [50.0, 40.0, 0.5],
        [50.0, 20.0, 0.5],
    ];

    let p_curve: Vec<[f64; 3]> = vec![
        [50.0, 20.0, 0.5],
        [60.0, 20.0, 0.5],
        [70.0, 20.0, 0.5],
        [80.0, 22.0, 0.5],
        [90.0, 28.0, 0.5],
        [95.0, 35.0, 0.5],
        [98.0, 45.0, 0.5],
        [98.0, 55.0, 0.5],
        [95.0, 65.0, 0.5],
        [90.0, 72.0, 0.5],
        [80.0, 78.0, 0.5],
        [70.0, 80.0, 0.5],
        [60.0, 80.0, 0.5],
        [50.0, 80.0, 0.5],
    ];

    render_strokes_to_png(
        &[p_stroke, p_curve],
        "/tmp/freehand_test_P.png",
        400,
        &options,
    );
}

#[test]
fn visual_hello() {
    let options = StrokeOptions {
        size: 5.0,
        ..StrokeOptions::default()
    };

    // "H" - three strokes
    let h1: Vec<[f64; 3]> = (0..15).map(|i| [20.0, 20.0 + i as f64 * 8.0, 0.5]).collect();
    let h2: Vec<[f64; 3]> = (0..15).map(|i| [60.0, 20.0 + i as f64 * 8.0, 0.5]).collect();
    let h3: Vec<[f64; 3]> = (0..8).map(|i| [20.0 + i as f64 * 5.7, 70.0, 0.5]).collect();

    // "e" - a curve
    let e_pts: Vec<[f64; 3]> = vec![
        [100.0, 70.0, 0.5], [110.0, 70.0, 0.5], [115.0, 65.0, 0.5],
        [115.0, 55.0, 0.5], [110.0, 50.0, 0.5], [100.0, 50.0, 0.5],
        [95.0, 55.0, 0.5], [95.0, 70.0, 0.5], [100.0, 80.0, 0.5],
        [110.0, 85.0, 0.5],
    ];

    // "l" - vertical
    let l1: Vec<[f64; 3]> = (0..15).map(|i| [135.0, 20.0 + i as f64 * 8.0, 0.5]).collect();
    // Another "l"
    let l2: Vec<[f64; 3]> = (0..15).map(|i| [155.0, 20.0 + i as f64 * 8.0, 0.5]).collect();

    render_strokes_to_png(
        &[h1, h2, h3, e_pts, l1, l2],
        "/tmp/freehand_test_hello.png",
        500,
        &options,
    );
}

#[test]
fn visual_stroke_points_debug() {
    // Single smooth curve — dump outline points count for debugging
    let options = StrokeOptions {
        size: 5.0,
        ..StrokeOptions::default()
    };

    let curve: Vec<[f64; 3]> = (0..30)
        .map(|i| {
            let t = i as f64 / 29.0;
            let x = t * 200.0;
            let y = 100.0 + (t * std::f64::consts::PI * 2.0).sin() * 50.0;
            [x, y, 0.5]
        })
        .collect();

    let points = get_stroke_points(&curve, &options);
    let outline = get_stroke(&curve, &options);

    eprintln!(
        "Sine wave: {} input pts -> {} stroke pts -> {} outline pts",
        curve.len(),
        points.len(),
        outline.len(),
    );

    render_strokes_to_png(
        &[curve],
        "/tmp/freehand_test_sine.png",
        500,
        &options,
    );
}

#[test]
fn visual_real_strokes() {
    // Load real stroke data from the database export
    let json_path = "/tmp/real_strokes.json";
    let Ok(json_str) = std::fs::read_to_string(json_path) else {
        eprintln!("Skipping: {} not found (run the python extraction first)", json_path);
        return;
    };

    let strokes: Vec<Vec<[f64; 3]>> = serde_json::from_str(&json_str)
        .expect("parse real_strokes.json");

    eprintln!("Loaded {} strokes from {}", strokes.len(), json_path);

    // Run through the ACTUAL rasterize_strokes_png pipeline (same as production)
    let bounds = crate::server::hub::board::serializer::types::CanvasBounds {
        x: 0.0, y: 0.0, width: 1.0, height: 1.0, // unused by rasterizer
    };

    let result = crate::server::hub::board::serializer::rasterize_png::rasterize_strokes_png(
        &strokes, &bounds, 1536, 10,
    );

    if let Some(b64) = &result {
        // Decode and save to /tmp for visual inspection
        let bytes = base64::engine::general_purpose::STANDARD
            .decode(b64)
            .expect("decode base64");
        std::fs::write("/tmp/real_strokes_rust.png", &bytes).expect("write png");
        eprintln!("Saved: /tmp/real_strokes_rust.png");
    } else {
        eprintln!("rasterize_strokes_png returned None");
    }
}

use serde_json;
use base64::Engine;
