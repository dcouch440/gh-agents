import { Geometry } from '@/utils/geometry'
import type { Point, Rect } from '@/utils/geometry'
import type { PlacementIntent, PlacementResult, OccupiedRect } from './types'
import { isOccupied, occupancyBounds } from './occupancyIndex'
import { PLACEMENT } from './constants'

// ============================================================================
// Free Space Finder — Row-Scan Fallback for Disconnected Nodes
// ============================================================================

/**
 * Find free space using a right-then-down scanning pattern.
 *
 * 1. Start at the seed point.
 * 2. Scan rightward in GRID_SIZE increments.
 * 3. After MAX_SCAN_COLS columns, wrap to the next row (seed.y + row * V_GAP).
 * 4. The first non-overlapping grid-aligned position wins.
 * 5. Absolute fallback: far-right of occupancy bounds.
 */
const findFreeSpace = (
  intent: PlacementIntent,
  seed: Point,
  occupancy: readonly OccupiedRect[],
): PlacementResult => {
  const snappedSeedX = Geometry.snapToGrid(seed.x, PLACEMENT.GRID_SIZE)
  const snappedSeedY = Geometry.snapToGrid(seed.y, PLACEMENT.GRID_SIZE)

  for (let row = 0; row < PLACEMENT.MAX_SCAN_ROWS; row++) {
    const candidateY = snappedSeedY + row * PLACEMENT.V_GAP

    for (let col = 0; col < PLACEMENT.MAX_SCAN_COLS; col++) {
      const candidateX = snappedSeedX + col * PLACEMENT.GRID_SIZE
      const candidate: Rect = {
        x: candidateX,
        y: candidateY,
        width: intent.width,
        height: intent.height,
      }

      if (!isOccupied(candidate, occupancy)) {
        return { stepId: intent.stepId, position: { x: candidateX, y: candidateY } }
      }
    }
  }

  // Absolute fallback: far right of existing content
  const bounds = occupancyBounds(occupancy)
  const fallbackX = bounds !== null
    ? Geometry.snapToGrid(bounds.x + bounds.width + PLACEMENT.H_GAP * 2, PLACEMENT.GRID_SIZE)
    : PLACEMENT.ORIGIN_X
  const fallbackY = snappedSeedY

  return { stepId: intent.stepId, position: { x: fallbackX, y: fallbackY } }
}

export { findFreeSpace }
