//! Rust port of the `perfect-freehand` stroke algorithm.
//!
//! Original: https://github.com/steveruizok/perfect-freehand (MIT).
//! Copyright (c) 2021 Stephen Ruiz Ltd — full licence text in NOTICE.
//!
//! Converts raw input points `[x, y, pressure]` into a closed outline
//! polygon with variable width, pressure sensitivity, and smooth caps.

mod stroke_outline;
pub(crate) mod stroke_points;
pub(crate) mod types;
pub(crate) mod vec;

#[cfg(test)]
mod tests;
#[cfg(test)]
mod visual_test;

use types::{StrokeOptions, Vec2};

/// Generate a stroke outline polygon from raw input points.
///
/// Input: slice of `[x, y, pressure]` triples.
/// Output: closed polygon as `Vec<Vec2>` suitable for filled rendering.
pub(crate) fn get_stroke(input: &[[f64; 3]], options: &StrokeOptions) -> Vec<Vec2> {
    let points = stroke_points::get_stroke_points(input, options);
    stroke_outline::get_stroke_outline_points(&points, options)
}
