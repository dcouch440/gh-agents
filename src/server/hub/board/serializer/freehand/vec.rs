//! 2D vector math utilities for the freehand stroke algorithm.
#![allow(dead_code)]

use super::types::Vec2;

pub fn neg(a: Vec2) -> Vec2 {
    [-a[0], -a[1]]
}

pub fn add(a: Vec2, b: Vec2) -> Vec2 {
    [a[0] + b[0], a[1] + b[1]]
}

pub fn sub(a: Vec2, b: Vec2) -> Vec2 {
    [a[0] - b[0], a[1] - b[1]]
}

pub fn mul(a: Vec2, s: f64) -> Vec2 {
    [a[0] * s, a[1] * s]
}

pub fn div(a: Vec2, s: f64) -> Vec2 {
    [a[0] / s, a[1] / s]
}

/// Perpendicular vector (90-degree clockwise rotation).
pub fn per(a: Vec2) -> Vec2 {
    [a[1], -a[0]]
}

pub fn dot(a: Vec2, b: Vec2) -> f64 {
    a[0] * b[0] + a[1] * b[1]
}

pub fn eq(a: Vec2, b: Vec2) -> bool {
    a[0] == b[0] && a[1] == b[1]
}

pub fn len(a: Vec2) -> f64 {
    a[0].hypot(a[1])
}

pub fn len_sq(a: Vec2) -> f64 {
    a[0] * a[0] + a[1] * a[1]
}

pub fn dist(a: Vec2, b: Vec2) -> f64 {
    (a[1] - b[1]).hypot(a[0] - b[0])
}

pub fn dist_sq(a: Vec2, b: Vec2) -> f64 {
    let d = sub(a, b);
    len_sq(d)
}

pub fn normalize(a: Vec2) -> Vec2 {
    let l = len(a);
    if l == 0.0 {
        return [0.0, 0.0];
    }
    div(a, l)
}

/// Rotate point `a` around `center` by `angle` radians.
pub fn rotate(a: Vec2, center: Vec2, angle: f64) -> Vec2 {
    let sin = angle.sin();
    let cos = angle.cos();
    let dx = a[0] - center[0];
    let dy = a[1] - center[1];
    [
        dx * cos - dy * sin + center[0],
        dx * sin + dy * cos + center[1],
    ]
}

/// Linear interpolation between `a` and `b` by factor `t`.
pub fn lerp(a: Vec2, b: Vec2, t: f64) -> Vec2 {
    add(a, mul(sub(b, a), t))
}

/// Project point `a` along direction `dir` by distance `d`.
pub fn project(a: Vec2, dir: Vec2, d: f64) -> Vec2 {
    add(a, mul(dir, d))
}
