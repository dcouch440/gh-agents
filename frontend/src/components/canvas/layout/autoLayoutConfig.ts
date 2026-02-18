/**
 * Spacing constants for the tower layout algorithm.
 * All values are multiples of CANVAS.GRID_SIZE (24px) for clean grid alignment.
 */
export const TOWER_LAYOUT = {
  /** Vertical gap between agent tiers (4 grid cells). */
  TOWER_GAP: 96,

  /** Horizontal gap between agents in the same tier (2 grid cells). */
  TIER_AGENT_GAP: 48,

  /** Vertical gap above the protocol node to the first agent tier (4 grid cells). */
  TOWER_START_GAP: 96,
} as const
