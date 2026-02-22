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

type PlacementStrategy = 'pipeline' | 'fan_out' | 'splice' | 'free_space'

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
  /** Fan-out: the placed source node ID (for grouping siblings). null otherwise. */
  readonly fanOutSourceId: string | null
  /** Splice: the placed downstream node ID to potentially shift. null otherwise. */
  readonly spliceDownstreamId: string | null
}

// ============================================================================
// Placement Result — Computed position for one step
// ============================================================================

/** The final computed position for one step. */
type PlacementResult = {
  readonly stepId: string
  readonly position: Point
}

// ============================================================================
// Placement Shift — Position adjustment for an existing node (splice only)
// ============================================================================

/** A position delta for an existing (already-placed) node shifted by splice. */
type PlacementShift = {
  readonly stepId: string
  readonly dx: number
  readonly dy: number
}

// ============================================================================
// Placement Output — Complete result from the placement engine
// ============================================================================

/** Complete output from the placement engine. */
type PlacementOutput = {
  /** Positions for newly placed nodes. */
  readonly placements: readonly PlacementResult[]
  /** Position shifts for existing nodes (splice nudging). */
  readonly shifts: readonly PlacementShift[]
}

// ============================================================================
// Splice Result — Output from the splice placer
// ============================================================================

/** Result of placing a splice node, with an optional shift for the downstream node. */
type SpliceResult = {
  readonly placement: PlacementResult
  readonly shift: PlacementShift | null
}

export type { OccupiedRect, PlacementStrategy, PlacementIntent, PlacementResult, PlacementShift, PlacementOutput, SpliceResult }
