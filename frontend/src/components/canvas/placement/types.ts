import type { Point, Rect } from '@/utils/geometry'

// ============================================================================
// Occupancy — Padded rectangles representing occupied canvas space
// ============================================================================

/** A rectangle occupied by an existing node, with padding applied for gap enforcement. */
type OccupiedRect = {
  readonly id: string
  readonly rect: Rect
  /** The rect expanded by OCCUPANCY_PAD on all sides — used for collision testing. */
  readonly paddedRect: Rect
}

// ============================================================================
// Placement Strategy — How to position an unplaced node
// ============================================================================

type PlacementStrategy = 'pipeline' | 'free_space'

// ============================================================================
// Placement Intent — Classification of an unplaced step
// ============================================================================

/** Classification result for a single unplaced step. */
type PlacementIntent = {
  readonly stepId: string
  readonly width: number
  readonly height: number
  readonly strategy: PlacementStrategy
  /**
   * For pipeline strategy: the step ID of the immediate upstream parent.
   * null means this is a root node in the chain (no placed upstream).
   */
  readonly upstreamStepId: string | null
  /** Step IDs downstream from this step (populated from edge adjacency). */
  readonly downstreamStepIds: readonly string[]
}

// ============================================================================
// Placement Result — Computed position for one step
// ============================================================================

/** The final computed position for one step. */
type PlacementResult = {
  readonly stepId: string
  readonly position: Point
}

export type { OccupiedRect, PlacementStrategy, PlacementIntent, PlacementResult }
