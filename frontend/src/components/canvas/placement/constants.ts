/**
 * Spacing constants for the placement engine.
 * All values are multiples of 24px (CANVAS.GRID_SIZE) for clean grid alignment.
 */
export const PLACEMENT = {
  /** Horizontal gap between pipeline-placed nodes (4 grid cells). */
  H_GAP: 96,

  /** Vertical gap between collision-shifted rows (2 grid cells). */
  V_GAP: 48,

  /** Padding added around each occupied rect for gap enforcement (1 grid cell). */
  OCCUPANCY_PAD: 24,

  /** Grid size for snapping (mirrors CANVAS.GRID_SIZE). */
  GRID_SIZE: 24,

  /** Default origin X for the first placed node on an empty canvas. */
  ORIGIN_X: 0,

  /** Default origin Y for the first placed node on an empty canvas. */
  ORIGIN_Y: 0,

  /** Maximum vertical rows to scan before using the absolute fallback. */
  MAX_SCAN_ROWS: 50,

  /** Maximum rightward columns to scan per row before wrapping. */
  MAX_SCAN_COLS: 80,
} as const
