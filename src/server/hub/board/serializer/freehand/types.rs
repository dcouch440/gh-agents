//! Type definitions for the perfect-freehand Rust port.

/// 2D point alias.
pub(crate) type Vec2 = [f64; 2];

/// Options for stroke generation, matching perfect-freehand's StrokeOptions.
#[derive(Clone)]
pub(crate) struct StrokeOptions {
    /// Base diameter of the stroke.
    pub size: f64,
    /// Effect of pressure on stroke width (0 = none, 1 = full).
    pub thinning: f64,
    /// Edge softening / minimum distance between outline points.
    pub smoothing: f64,
    /// Point interpolation factor (0 = no smoothing, 1 = max smoothing).
    pub streamline: f64,
    /// Whether to simulate pressure from stroke velocity.
    pub simulate_pressure: bool,
    /// Whether the stroke is finalized (complete).
    pub last: bool,
    /// Start cap/taper options.
    pub start: CapOptions,
    /// End cap/taper options.
    pub end: CapOptions,
    /// Easing function applied to pressure values.
    pub easing: fn(f64) -> f64,
}

/// Cap and taper options for stroke endpoints.
#[derive(Clone)]
pub(crate) struct CapOptions {
    /// Whether to draw a rounded cap (true) or flat cap (false).
    pub cap: bool,
    /// Taper configuration.
    pub taper: TaperValue,
    /// Easing function for the taper.
    pub easing: fn(f64) -> f64,
}

/// Taper configuration for stroke endpoints.
#[derive(Clone, Debug)]
pub(crate) enum TaperValue {
    /// No taper (false).
    Disabled,
    /// Auto taper: use max(size, running_length) (true).
    Auto,
    /// Fixed taper distance in pixels.
    Fixed(f64),
}

/// A processed stroke point (output of get_stroke_points).
#[derive(Clone, Debug)]
pub(crate) struct StrokePoint {
    pub point: Vec2,
    pub pressure: f64,
    pub distance: f64,
    pub vector: Vec2,
    pub running_length: f64,
}

impl Default for StrokeOptions {
    fn default() -> Self {
        Self {
            size: 16.0,
            thinning: 0.5,
            smoothing: 0.5,
            streamline: 0.5,
            simulate_pressure: true,
            last: false,
            start: CapOptions {
                cap: true,
                taper: TaperValue::Disabled,
                easing: |t| t * (2.0 - t), // ease-out quadratic
            },
            end: CapOptions {
                cap: true,
                taper: TaperValue::Disabled,
                easing: |t| {
                    let u = t - 1.0;
                    u * u * u + 1.0 // ease-out cubic
                },
            },
            easing: |t| t,
        }
    }
}
