/**
 * Round a value to the nearest grid multiple, clamped to a minimum.
 *
 * Equivalent to `Math.max(min, snapToGrid(value, gridSize))`.
 * Useful for constraining resize dimensions to clean grid-aligned sizes.
 */
const snapToGridMin = (value: number, gridSize: number, min: number): number =>
  Math.max(min, Math.round(value / gridSize) * gridSize)

export { snapToGridMin }
