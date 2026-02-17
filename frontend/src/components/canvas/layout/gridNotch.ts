// ============================================================================
// Grid Notch — Quantize dimensions to nearest grid multiple
// ============================================================================

/**
 * Round a dimension value to the nearest grid multiple,
 * clamped to a minimum value.
 *
 * Used during node resize to make blocks snap to clean grid-aligned sizes.
 */
const notchToGrid = (value: number, gridSize: number, min: number): number =>
  Math.max(min, Math.round(value / gridSize) * gridSize)

export { notchToGrid }
