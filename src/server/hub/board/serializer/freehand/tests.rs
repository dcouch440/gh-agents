//! Tests for the perfect-freehand Rust port.

use super::get_stroke;
use super::stroke_points::get_stroke_points;
use super::types::*;
use super::vec;

// ============================================================================
// Vec math
// ============================================================================

#[test]
fn vec_add_sub() {
    assert_eq!(vec::add([1.0, 2.0], [3.0, 4.0]), [4.0, 6.0]);
    assert_eq!(vec::sub([5.0, 3.0], [2.0, 1.0]), [3.0, 2.0]);
}

#[test]
fn vec_perpendicular() {
    assert_eq!(vec::per([1.0, 0.0]), [0.0, -1.0]);
    assert_eq!(vec::per([0.0, 1.0]), [1.0, 0.0]);
}

#[test]
fn vec_normalize_zero() {
    assert_eq!(vec::normalize([0.0, 0.0]), [0.0, 0.0]);
}

#[test]
fn vec_normalize_unit() {
    let n = vec::normalize([3.0, 4.0]);
    assert!((vec::len(n) - 1.0).abs() < 1e-10);
}

#[test]
fn vec_lerp_midpoint() {
    let mid = vec::lerp([0.0, 0.0], [10.0, 10.0], 0.5);
    assert!((mid[0] - 5.0).abs() < 1e-10);
    assert!((mid[1] - 5.0).abs() < 1e-10);
}

// ============================================================================
// Stroke points
// ============================================================================

#[test]
fn empty_input_produces_empty() {
    let opts = StrokeOptions::default();
    let result = get_stroke_points(&[], &opts);
    assert!(result.is_empty());
}

#[test]
fn single_point_duplicated() {
    let opts = StrokeOptions::default();
    let result = get_stroke_points(&[[100.0, 100.0, 0.5]], &opts);
    assert!(result.len() >= 2, "single point should be duplicated");
}

#[test]
fn two_points_subdivided() {
    let opts = StrokeOptions::default();
    let input = [[0.0, 0.0, 0.5], [100.0, 0.0, 0.5]];
    let result = get_stroke_points(&input, &opts);
    assert!(
        result.len() >= 3,
        "two points should be subdivided, got {}",
        result.len()
    );
}

#[test]
fn running_length_increases() {
    let opts = StrokeOptions {
        size: 1.0, // small size so we don't skip jitter
        ..StrokeOptions::default()
    };
    let input: Vec<[f64; 3]> = (0..20).map(|i| [i as f64 * 10.0, 0.0, 0.5]).collect();
    let result = get_stroke_points(&input, &opts);

    for pair in result.windows(2) {
        assert!(
            pair[1].running_length >= pair[0].running_length,
            "running_length should be non-decreasing"
        );
    }
}

// ============================================================================
// Full stroke outline
// ============================================================================

#[test]
fn empty_input_empty_outline() {
    let opts = StrokeOptions::default();
    let result = get_stroke(&[], &opts);
    assert!(result.is_empty());
}

#[test]
fn single_point_produces_dot() {
    let opts = StrokeOptions {
        last: true,
        ..StrokeOptions::default()
    };
    let result = get_stroke(&[[50.0, 50.0, 0.5]], &opts);
    assert!(!result.is_empty(), "single point should produce a dot");
    // All points should be roughly equidistant from center
    let center = [50.0, 50.0];
    let distances: Vec<f64> = result.iter().map(|p| vec::dist(*p, center)).collect();
    if distances.len() > 2 {
        let avg = distances.iter().sum::<f64>() / distances.len() as f64;
        for d in &distances {
            assert!(
                (d - avg).abs() < avg * 0.5,
                "dot points should be roughly circular"
            );
        }
    }
}

#[test]
fn straight_line_produces_outline() {
    let opts = StrokeOptions {
        size: 10.0,
        thinning: 0.0,
        simulate_pressure: false,
        ..StrokeOptions::default()
    };

    let input: Vec<[f64; 3]> = (0..30).map(|i| [i as f64 * 5.0, 0.0, 0.5]).collect();

    let outline = get_stroke(&input, &opts);
    assert!(
        outline.len() >= 4,
        "straight line should produce an outline polygon, got {} points",
        outline.len()
    );
}

#[test]
fn pressure_affects_width() {
    let opts = StrokeOptions {
        size: 20.0,
        thinning: 0.5,
        simulate_pressure: false,
        ..StrokeOptions::default()
    };

    // Light pressure stroke
    let light: Vec<[f64; 3]> = (0..30).map(|i| [i as f64 * 5.0, 0.0, 0.1]).collect();

    // Heavy pressure stroke
    let heavy: Vec<[f64; 3]> = (0..30).map(|i| [i as f64 * 5.0, 0.0, 0.9]).collect();

    let light_outline = get_stroke(&light, &opts);
    let heavy_outline = get_stroke(&heavy, &opts);

    // Measure approximate widths (max Y spread)
    let light_height = y_spread(&light_outline);
    let heavy_height = y_spread(&heavy_outline);

    assert!(
        heavy_height > light_height,
        "heavy pressure ({}) should produce wider stroke than light ({})",
        heavy_height,
        light_height
    );
}

/// Measure the Y-axis spread of an outline (proxy for stroke width on a horizontal line).
fn y_spread(outline: &[Vec2]) -> f64 {
    if outline.is_empty() {
        return 0.0;
    }
    let min_y = outline.iter().map(|p| p[1]).fold(f64::MAX, f64::min);
    let max_y = outline.iter().map(|p| p[1]).fold(f64::MIN, f64::max);
    max_y - min_y
}

#[test]
fn sharp_corner_handled() {
    let opts = StrokeOptions {
        size: 5.0,
        ..StrokeOptions::default()
    };

    // L-shaped path: right then down
    let mut input: Vec<[f64; 3]> = Vec::new();
    for i in 0..15 {
        input.push([i as f64 * 10.0, 0.0, 0.5]);
    }
    for i in 1..15 {
        input.push([140.0, i as f64 * 10.0, 0.5]);
    }

    let outline = get_stroke(&input, &opts);
    assert!(
        outline.len() >= 10,
        "L-shape should produce a substantial outline"
    );
}

#[test]
fn compare_outline_with_js_reference() {
    // Same stroke from debug_strokes.json, run through JS perfect-freehand v1.2.3
    // with options: size=5, thinning=0.5, smoothing=0.5, streamline=0.5, simulatePressure=true
    let stroke: Vec<[f64; 3]> = vec![
        [213.0, 171.0, 0.5],
        [210.0, 284.0, 0.5],
        [198.0, 314.0, 0.5],
        [190.0, 362.0, 0.5],
        [197.0, 481.0, 0.5],
        [189.0, 513.0, 0.5],
        [194.0, 575.0, 0.5],
        [186.0, 620.0, 0.5],
    ];

    let options = StrokeOptions {
        size: 5.0,
        simulate_pressure: true,
        ..StrokeOptions::default()
    };

    let outline = get_stroke(&stroke, &options);

    // JS produced 58 outline points
    assert_eq!(
        outline.len(),
        58,
        "Rust should produce same number of outline points as JS (58), got {}",
        outline.len()
    );

    // Compare first few points with JS output (tolerance 0.001)
    let js_first_points = [
        [214.693241, 171.044953],
        [212.846010, 236.018947],
        [205.104154, 281.088164],
        [197.197598, 327.738235],
        [197.861631, 415.755028],
        [193.517818, 471.777534],
        [194.541943, 531.064511],
        [190.364244, 582.317036],
    ];

    for (i, js_pt) in js_first_points.iter().enumerate() {
        let rust_pt = &outline[i];
        let dx = (rust_pt[0] - js_pt[0]).abs();
        let dy = (rust_pt[1] - js_pt[1]).abs();
        assert!(
            dx < 0.01 && dy < 0.01,
            "Point {i}: Rust=[{:.6}, {:.6}] vs JS=[{:.6}, {:.6}] (dx={dx:.6}, dy={dy:.6})",
            rust_pt[0],
            rust_pt[1],
            js_pt[0],
            js_pt[1]
        );
    }
}
