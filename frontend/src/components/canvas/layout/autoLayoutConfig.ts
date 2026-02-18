/**
 * Spacing and positioning constants for the spine+tower auto-layout algorithm.
 * All values are multiples of CANVAS.GRID_SIZE (24px) for clean grid alignment.
 */
export const AUTO_LAYOUT = {
  /** Horizontal gap between spine columns (3 grid cells). */
  SPINE_GAP: 72,

  /** Vertical gap between tower entries — agent+doc pairs (4 grid cells). */
  TOWER_GAP: 96,

  /** Vertical gap between agent node and its document above (2 grid cells). */
  DOC_GAP: 48,

  /** Horizontal gap between agents in the same tier (2 grid cells). */
  TIER_AGENT_GAP: 48,

  /** Vertical gap above the protocol node to the first tower entry (4 grid cells). */
  TOWER_START_GAP: 96,

  /** Y-coordinate of the spine row (anchor baseline). */
  SPINE_Y: 0,
} as const
