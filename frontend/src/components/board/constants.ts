// ============================================================================
// Board Constants — Sizing, Thresholds, and Rendering Configuration
// ============================================================================

const BOARD = {
  /** Grid spacing in px (matches existing CANVAS.GRID_SIZE). */
  GRID_SIZE: 24,

  // ── Box sizing ───────────────────────────────────────────────────────────
  MIN_BOX_WIDTH: 120,
  MIN_BOX_HEIGHT: 40,
  MAX_BOX_WIDTH: 400,
  DEFAULT_BOX_WIDTH: 200,
  DEFAULT_BOX_HEIGHT: 48,
  BOX_PADDING_X: 20,
  BOX_PADDING_Y: 12,
  BOX_BORDER_RADIUS: 6,
  BOX_BORDER_WIDTH: 1.5,

  // ── Arrow rendering ──────────────────────────────────────────────────────
  /** Gap between arrow endpoint and box edge (px). */
  ARROW_BINDING_GAP: 8,
  /** Snap-to-midpoint threshold when binding an arrow to a box side (px). */
  ARROW_SNAP_THRESHOLD: 20,
  ARROW_STROKE_WIDTH: 2,
  ARROW_HEAD_SIZE: 8,

  // ── Handles ──────────────────────────────────────────────────────────────
  HANDLE_SIZE: 8,
  HANDLE_HOVER_SIZE: 12,

  // ── Typography ───────────────────────────────────────────────────────────
  FONT_SIZE: 16,
  LINE_HEIGHT: 1.4,
  FONT_FAMILY: 'Virgil, Segoe Print, Bradley Hand, system-ui, sans-serif',

  // ── Viewport ─────────────────────────────────────────────────────────────
  MIN_ZOOM: 0.25,
  MAX_ZOOM: 2.0,
  ZOOM_SPEED: 0.001,

  // ── Snap & alignment ────────────────────────────────────────────────────
  SNAP_MAGNETIC_THRESHOLD: 16,

  // ── History ──────────────────────────────────────────────────────────────
  HISTORY_MAX_DEPTH: 100,
} as const

export { BOARD }
