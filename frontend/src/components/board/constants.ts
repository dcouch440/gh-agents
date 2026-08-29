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
  BOX_BORDER_RADIUS: 14,
  BOX_BORDER_WIDTH: 2,
  /**
   * Dash for a box that has not been designed yet, in *screen* px — the
   * renderer divides by zoom so the gaps stay readable when zoomed out.
   */
  BOX_UNDESIGNED_DASH: [7, 5] as readonly number[],

  // ── Arrow rendering ──────────────────────────────────────────────────────
  /** Gap between arrow endpoint and box edge (px). */
  ARROW_BINDING_GAP: 20,
  /** Snap-to-midpoint threshold when binding an arrow to a box side (px). */
  ARROW_SNAP_THRESHOLD: 20,
  /** Max stroke width at arrow tip (tapers from ARROW_STROKE_MIN). */
  ARROW_STROKE_WIDTH: 3.5,
  /** Min stroke width at arrow source. */
  ARROW_STROKE_MIN: 1.0,
  ARROW_HEAD_SIZE: 18,
  /** Arrowhead barb spread angle (~33°). */
  ARROW_HEAD_SPREAD: Math.PI / 5.5,
  /** Arrowhead barb line width. */
  ARROW_HEAD_STROKE: 3,
  /** Number of bezier sample points for tapered stroke. */
  ARROW_TAPER_SEGMENTS: 40,
  /** Max wobble offset in px for hand-drawn feel. */
  ARROW_WOBBLE: 1.2,

  // ── Edge hover ──────────────────────────────────────────────────────────
  /** Base distance from box edge to trigger arrow binding hover (px). */
  EDGE_HOVER_THRESHOLD: 16,
  /** Minimum edge hover threshold at high zoom. */
  EDGE_HOVER_MIN_THRESHOLD: 8,
  /** Maximum edge hover threshold at low zoom. */
  EDGE_HOVER_MAX_THRESHOLD: 32,

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

  // ── Anchor clamping ─────────────────────────────────────────────────────
  /** Min ratio along a side for arrow anchor (avoids corners). */
  ANCHOR_CLAMP_MIN: 0.1,
  /** Max ratio along a side for arrow anchor (avoids corners). */
  ANCHOR_CLAMP_MAX: 0.9,

  // ── Snap & alignment ────────────────────────────────────────────────────
  SNAP_MAGNETIC_THRESHOLD: 16,

  // ── Zoom ───────────────────────────────────────────────────────────────
  /** Zoom step multiplier for toolbar zoom in/out buttons. */
  ZOOM_BUTTON_STEP: 1.2,

  // ── History ──────────────────────────────────────────────────────────────
  HISTORY_MAX_DEPTH: 100,
} as const

/**
 * Status ring tuning, kept apart from BOARD because these govern an animation
 * budget rather than board geometry.
 */
const BOARD_RING = {
  /** Below this zoom, rings still draw but stop breathing. */
  ANIMATE_MIN_ZOOM: 0.45,
  /** One full breath, in ms. */
  PULSE_PERIOD_MS: 1800,
  /** Minimum ms between pulse repaints — well under display refresh on purpose. */
  PULSE_FRAME_MS: 50,
} as const

export { BOARD, BOARD_RING }
