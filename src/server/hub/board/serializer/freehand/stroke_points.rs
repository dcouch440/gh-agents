//! Input point processing: streamline smoothing + pressure simulation.
//!
//! Port of perfect-freehand's `getStrokePoints` function.

use super::types::{StrokeOptions, StrokePoint, Vec2};
use super::vec;

/// Default pressure for points without explicit pressure data.
const DEFAULT_PRESSURE: f64 = 0.5;

/// Pressure assigned to the very first point.
const DEFAULT_FIRST_PRESSURE: f64 = 0.25;

/// Unit offset used when duplicating a single point.
const UNIT_OFFSET: Vec2 = [1.0, 1.0];

/// Rate at which simulated pressure responds to velocity changes.
const RATE_OF_PRESSURE_CHANGE: f64 = 0.275;

/// Minimum streamline interpolation factor.
const MIN_STREAMLINE_T: f64 = 0.15;

/// Streamline interpolation range (added to MIN_STREAMLINE_T).
const STREAMLINE_T_RANGE: f64 = 0.85;

/// Returns true if a pressure value is present and non-negative.
fn is_valid_pressure(p: f64) -> bool {
    p >= 0.0
}

/// Simulate pressure based on velocity (distance between consecutive points).
///
/// Moves the current pressure toward the target `min(1, distance / size)`
/// by `RATE_OF_PRESSURE_CHANGE`, then averages with the current value.
pub(crate) fn simulate_pressure(prev_pressure: f64, distance: f64, size: f64) -> f64 {
    // Speed factor: how fast is the pen moving relative to stroke size?
    let sp = 1.0_f64.min(distance / size);
    // Rate of change: inverse of speed (slow = high pressure, fast = low)
    let rp = 1.0_f64.min(1.0 - sp);
    // Accelerate pressure toward target
    1.0_f64.min(prev_pressure + (rp - prev_pressure) * (sp * RATE_OF_PRESSURE_CHANGE))
}

/// Process raw input points into stroke points with smoothing and pressure.
///
/// Input points are `[x, y, pressure]` triples. If pressure is negative or
/// missing, defaults are applied (0.25 for first point, 0.5 for others).
///
/// The streamline parameter controls how much each point is interpolated
/// toward the previous point (`0` = no smoothing, `1` = maximum smoothing).
pub(crate) fn get_stroke_points(
    input: &[[f64; 3]],
    options: &StrokeOptions,
) -> Vec<StrokePoint> {
    if input.is_empty() {
        return vec![];
    }

    let streamline_t = MIN_STREAMLINE_T + (1.0 - options.streamline) * STREAMLINE_T_RANGE;

    // Normalize input: handle 1-point and 2-point edge cases
    let mut points: Vec<[f64; 3]> = input.to_vec();

    if points.len() == 2 {
        // Subdivide into 5 points via lerp.
        // JS lrp() only interpolates x/y (Vec2), so the subdivided points
        // have no pressure — they fall through to DEFAULT_PRESSURE (0.5).
        let a = points[0];
        let b = points[1];
        points = vec![a];
        for i in 1..5 {
            let t = i as f64 / 4.0;
            let pt = vec::lerp([a[0], a[1]], [b[0], b[1]], t);
            points.push([pt[0], pt[1], -1.0]);
        }
    }

    if points.len() == 1 {
        // Duplicate with a tiny offset
        let p = points[0];
        let offset = vec::add([p[0], p[1]], UNIT_OFFSET);
        points.push([offset[0], offset[1], p[2]]);
    }

    // Build first stroke point
    let first_pressure = if is_valid_pressure(points[0][2]) {
        points[0][2]
    } else {
        DEFAULT_FIRST_PRESSURE
    };

    let mut result = vec![StrokePoint {
        point: [points[0][0], points[0][1]],
        pressure: first_pressure,
        vector: UNIT_OFFSET,
        distance: 0.0,
        running_length: 0.0,
    }];

    let mut has_reached_min_length = false;
    let mut running_length = 0.0;
    let last_idx = points.len() - 1;

    for i in 1..points.len() {
        let is_last = options.last && i == last_idx;
        let prev = result.last().unwrap();

        // Interpolate toward previous point (streamline) unless last point
        let current = if is_last {
            [points[i][0], points[i][1]]
        } else {
            vec::lerp(prev.point, [points[i][0], points[i][1]], streamline_t)
        };

        // Skip if point hasn't moved
        if vec::eq(prev.point, current) {
            continue;
        }

        let distance = vec::dist(current, prev.point);
        running_length += distance;

        // Skip jitter points until we've traveled at least `size` distance
        if i < last_idx && !has_reached_min_length {
            if running_length < options.size {
                continue;
            }
            has_reached_min_length = true;
        }

        let pressure = if is_valid_pressure(points[i][2]) {
            points[i][2]
        } else {
            DEFAULT_PRESSURE
        };

        let direction = vec::normalize(vec::sub(prev.point, current));

        result.push(StrokePoint {
            point: current,
            pressure,
            vector: direction,
            distance,
            running_length,
        });
    }

    // Copy the second point's vector to the first (first has no meaningful direction)
    if result.len() > 1 {
        result[0].vector = result[1].vector;
    }

    result
}
