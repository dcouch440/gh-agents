//! Outline polygon generation from processed stroke points.
//!
//! Port of perfect-freehand's `getStrokeOutlinePoints` function.
//! Produces a closed polygon with variable width based on pressure,
//! tapered ends, and rounded or flat caps.
#![allow(dead_code)]

use std::f64::consts::PI;

use super::types::{StrokeOptions, StrokePoint, TaperValue, Vec2};
use super::vec;

/// Fixed PI with a tiny offset to avoid exact-boundary edge cases.
const FIXED_PI: f64 = PI + 0.0001;

/// Number of arc segments for rounded start caps.
const START_CAP_SEGMENTS: usize = 13;

/// Number of arc segments for rounded end caps.
const END_CAP_SEGMENTS: usize = 29;

/// Minimum stroke radius (prevents zero-width artifacts).
const MIN_RADIUS: f64 = 0.01;

/// Distance from end of stroke below which points are skipped (noise).
const END_NOISE_THRESHOLD: f64 = 3.0;

/// Corner arc step size (1/13 ≈ 0.0769), matching the JS constant.
const CORNER_STEP: f64 = 1.0 / 13.0;

/// Compute the effective stroke radius from pressure and thinning.
///
/// Maps `pressure` through `easing(0.5 - thinning * (0.5 - pressure))`
/// then multiplies by `size`.
fn stroke_radius(size: f64, thinning: f64, pressure: f64, easing: fn(f64) -> f64) -> f64 {
    size * easing(0.5 - thinning * (0.5 - pressure))
}

/// Resolve a taper value to a concrete pixel distance.
fn resolve_taper(taper: &TaperValue, size: f64, total_length: f64) -> f64 {
    match taper {
        TaperValue::Disabled => 0.0,
        TaperValue::Auto => size.max(total_length),
        TaperValue::Fixed(d) => *d,
    }
}

/// Average the first ≤10 points' pressures (simulating if needed).
fn average_start_pressure(points: &[StrokePoint], simulate: bool, size: f64) -> f64 {
    let limit = points.len().min(10);
    let mut pressure = points[0].pressure;

    for pt in &points[..limit] {
        let p = if simulate {
            super::stroke_points::simulate_pressure(pressure, pt.distance, size)
        } else {
            pt.pressure
        };
        pressure = (pressure + p) / 2.0;
    }

    pressure
}

/// Generate a dot (circle) outline for single-point strokes.
fn dot_outline(center: Vec2, radius: f64) -> Vec<Vec2> {
    // JS: T(e, b(h(u(e, c(e,[1,1])))), -n)
    // = project(center, normalize(per(center - (center + [1,1]))), -radius)
    // = project(center, normalize(per([-1,-1])), -radius)
    // per([-1,-1]) = [-1, 1], normalize = [-0.707, 0.707]
    let raw = vec::sub(center, vec::add(center, [1.0, 1.0])); // [-1, -1]
    let offset = vec::normalize(vec::per(raw)); // per then normalize
    let start = vec::project(center, offset, -radius);

    let step = 1.0 / START_CAP_SEGMENTS as f64;
    let mut outline = Vec::with_capacity(START_CAP_SEGMENTS);

    let mut t = step;
    while t <= 1.0 {
        outline.push(vec::rotate(start, center, FIXED_PI * 2.0 * t));
        t += step;
    }

    outline
}

/// Generate a rounded start cap (arc from right side to left side).
fn round_start_cap(center: Vec2, right: Vec2, segments: usize) -> Vec<Vec2> {
    let step = 1.0 / segments as f64;
    let mut cap = Vec::with_capacity(segments);

    let mut t = step;
    while t <= 1.0 {
        cap.push(vec::rotate(right, center, FIXED_PI * t));
        t += step;
    }

    cap
}

/// Generate a flat start cap (rectangle across the start).
/// JS: j(center, left, right) = { r=left-right; half=r*0.5; slight=r*0.51;
///   [center-half, center-slight, center+slight, center+half] }
fn flat_start_cap(center: Vec2, left: Vec2, right: Vec2) -> Vec<Vec2> {
    let diff = vec::sub(left, right);
    let half = vec::mul(diff, 0.5);
    let slightly_more = vec::mul(diff, 0.51);

    vec![
        vec::sub(center, half),
        vec::sub(center, slightly_more),
        vec::add(center, slightly_more),
        vec::add(center, half),
    ]
}

/// Generate a rounded end cap.
fn round_end_cap(center: Vec2, direction: Vec2, radius: f64, segments: usize) -> Vec<Vec2> {
    let start = vec::project(center, direction, radius);
    let step = 1.0 / segments as f64;
    let mut cap = Vec::with_capacity(segments);

    let mut t = step;
    while t < 1.0 {
        cap.push(vec::rotate(start, center, FIXED_PI * 3.0 * t));
        t += step;
    }

    cap
}

/// Generate a flat end cap.
fn flat_end_cap(center: Vec2, direction: Vec2, radius: f64) -> Vec<Vec2> {
    vec![
        vec::add(center, vec::mul(direction, radius)),
        vec::add(center, vec::mul(direction, radius * 0.99)),
        vec::sub(center, vec::mul(direction, radius * 0.99)),
        vec::sub(center, vec::mul(direction, radius)),
    ]
}

/// Generate the outline polygon from processed stroke points.
///
/// Returns a closed polygon as a list of `Vec2` points suitable for
/// filled rendering. The polygon is assembled as:
/// `left_points + end_cap + reversed(right_points) + start_cap`
pub(crate) fn get_stroke_outline_points(
    points: &[StrokePoint],
    options: &StrokeOptions,
) -> Vec<Vec2> {
    if points.is_empty() || options.size <= 0.0 {
        return vec![];
    }

    let total_length = points.last().unwrap().running_length;

    let start_taper = resolve_taper(&options.start.taper, options.size, total_length);
    let end_taper = resolve_taper(&options.end.taper, options.size, total_length);

    let smoothing_threshold = (options.size * options.smoothing).powi(2);

    let mut left_points: Vec<Vec2> = Vec::new();
    let mut right_points: Vec<Vec2> = Vec::new();

    let mut prev_pressure = average_start_pressure(points, options.simulate_pressure, options.size);
    let mut radius = stroke_radius(
        options.size,
        options.thinning,
        points.last().unwrap().pressure,
        options.easing,
    );
    let mut first_radius: Option<f64> = None;

    let mut prev_vector = points[0].vector;
    // JS: G=e[0].point, K=G, q=G, J=K
    let mut prev_left = points[0].point;
    let mut prev_right = points[0].point;
    let mut _last_left = prev_left;
    let mut _last_right = prev_right;
    let mut is_sharp = false;

    let last_idx = points.len() - 1;

    for i in 0..points.len() {
        let pt = &points[i];
        let is_last = i == last_idx;

        // Skip noise near the end of the stroke
        if !is_last && total_length - pt.running_length < END_NOISE_THRESHOLD {
            continue;
        }

        // Compute radius from pressure
        let mut pressure = pt.pressure;
        if options.thinning != 0.0 {
            if options.simulate_pressure {
                pressure = super::stroke_points::simulate_pressure(
                    prev_pressure,
                    pt.distance,
                    options.size,
                );
            }
            radius = stroke_radius(options.size, options.thinning, pressure, options.easing);
        } else {
            radius = options.size / 2.0;
        }

        if first_radius.is_none() {
            first_radius = Some(radius);
        }

        // Apply start taper
        let start_taper_factor = if pt.running_length < start_taper {
            (options.start.easing)(pt.running_length / start_taper)
        } else {
            1.0
        };

        // Apply end taper
        let end_taper_factor = if total_length - pt.running_length < end_taper {
            (options.end.easing)((total_length - pt.running_length) / end_taper)
        } else {
            1.0
        };

        radius = MIN_RADIUS.max(radius * start_taper_factor.min(end_taper_factor));

        // Check for sharp corners
        let next_vector = if is_last {
            pt.vector
        } else {
            points[i + 1].vector
        };

        let dot_next = if is_last {
            1.0
        } else {
            vec::dot(pt.vector, next_vector)
        };

        let dot_prev = vec::dot(pt.vector, prev_vector);
        let is_sharp_prev = dot_prev < 0.0 && !is_sharp;
        let is_sharp_corner = dot_next < 0.0;

        // Sharp corner: emit arc segments on both sides
        // JS: for(let e=0; e<=1; e += 1/13) — starts at 0, inclusive
        if is_sharp_prev || is_sharp_corner {
            let perp = vec::mul(vec::per(prev_vector), radius);

            let mut t = 0.0;
            while t <= 1.0 {
                let left = vec::rotate(vec::sub(pt.point, perp), pt.point, FIXED_PI * t);
                _last_left = left;
                left_points.push(left);

                let right = vec::rotate(vec::add(pt.point, perp), pt.point, FIXED_PI * -t);
                _last_right = right;
                right_points.push(right);

                t += CORNER_STEP;
            }

            prev_left = _last_left;
            prev_right = _last_right;

            if is_sharp_corner {
                is_sharp = true;
            }
            continue;
        }

        is_sharp = false;

        // Last point: just add perpendicular offset
        if is_last {
            let perp = vec::mul(vec::per(pt.vector), radius);
            left_points.push(vec::sub(pt.point, perp));
            right_points.push(vec::add(pt.point, perp));
            continue;
        }

        // Normal point: use averaged direction between current and next
        // JS: te(E, k, h, A) = lerp(next_vec, cur_vec, dot)
        // then per(E), then scale by radius
        let offset_vec = vec::lerp(next_vector, pt.vector, dot_next);
        let perp = vec::mul(vec::per(offset_vec), radius);

        let left = vec::sub(pt.point, perp);
        _last_left = left;
        if i <= 1 || vec::dist_sq(prev_left, left) > smoothing_threshold {
            left_points.push(left);
            prev_left = left;
        }

        let right = vec::add(pt.point, perp);
        _last_right = right;
        if i <= 1 || vec::dist_sq(prev_right, right) > smoothing_threshold {
            right_points.push(right);
            prev_right = right;
        }

        prev_pressure = pressure;
        prev_vector = pt.vector;
    }

    // Assemble the outline polygon
    let first_point = [points[0].point[0], points[0].point[1]];
    let last_point = if points.len() > 1 {
        let lp = points[last_idx].point;
        [lp[0], lp[1]]
    } else {
        vec::add(points[0].point, [1.0, 1.0])
    };

    // Single point → dot
    if points.len() == 1 {
        if (start_taper == 0.0 && end_taper == 0.0) || options.last {
            return dot_outline(first_point, first_radius.unwrap_or(radius));
        }
    }

    // Build start cap
    // JS: I||L&&e.length===1||(S?Q.push(...A(X,B[0],13)):Q.push(...j(X,z[0],B[0])))
    // = if !(start_taper || (end_taper && len==1)) then add cap
    let start_cap = if start_taper > 0.0 || (end_taper > 0.0 && points.len() == 1) {
        vec![]
    } else if left_points.is_empty() || right_points.is_empty() {
        vec![]
    } else if options.start.cap {
        round_start_cap(first_point, right_points[0], START_CAP_SEGMENTS)
    } else {
        flat_start_cap(first_point, left_points[0], right_points[0])
    };

    // Build end cap
    // JS: let t=h(s(e[e.length-1].vector));  = per(neg(last_vector))
    // L||I&&e.length===1?$.push(Z):T?$.push(...M(Z,t,H,29)):$.push(...ne(Z,t,H))
    let last_vector = points[last_idx].vector;
    let end_direction = vec::per(vec::neg(last_vector));

    let end_cap = if end_taper > 0.0 || (start_taper > 0.0 && points.len() == 1) {
        vec![last_point]
    } else if options.end.cap {
        round_end_cap(last_point, end_direction, radius, END_CAP_SEGMENTS)
    } else {
        flat_end_cap(last_point, end_direction, radius)
    };

    // Assemble: left + end_cap + reversed(right) + start_cap
    let mut outline = left_points;
    outline.extend(end_cap);
    right_points.reverse();
    outline.extend(right_points);
    outline.extend(start_cap);

    outline
}
