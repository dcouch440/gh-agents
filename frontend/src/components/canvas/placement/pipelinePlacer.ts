import { Geometry } from '@/utils/geometry'
import type { Rect } from '@/utils/geometry'
import type { PlacementIntent, PlacementResult, OccupiedRect } from './types'
import { isOccupied, occupancyBounds } from './occupancyIndex'
import { PLACEMENT } from './constants'

// ============================================================================
// Pipeline Placer — Left-to-Right Chain Placement
// ============================================================================

/**
 * Place a node to the right of its upstream neighbor.
 *
 * Start at (upstream.right + H_GAP, upstream.y) — top-aligned.
 * If occupied, shift down by V_GAP and retry (max MAX_SCAN_ROWS attempts).
 * All positions snapped to grid.
 */
const placePipelineNode = (
  intent: PlacementIntent,
  upstreamRect: Rect,
  occupancy: readonly OccupiedRect[],
): PlacementResult => {
  const startX = Geometry.snapToGrid(
    upstreamRect.x + upstreamRect.width + PLACEMENT.H_GAP,
    PLACEMENT.GRID_SIZE,
  )
  const startY = Geometry.snapToGrid(upstreamRect.y, PLACEMENT.GRID_SIZE)

  for (let row = 0; row < PLACEMENT.MAX_SCAN_ROWS; row++) {
    const candidateY = startY + row * PLACEMENT.V_GAP
    const candidate: Rect = { x: startX, y: candidateY, width: intent.width, height: intent.height }

    if (!isOccupied(candidate, occupancy)) {
      return { stepId: intent.stepId, position: { x: startX, y: candidateY } }
    }
  }

  // Fallback: place below the last attempted position
  const fallbackY = Geometry.snapToGrid(
    startY + PLACEMENT.MAX_SCAN_ROWS * PLACEMENT.V_GAP,
    PLACEMENT.GRID_SIZE,
  )
  return { stepId: intent.stepId, position: { x: startX, y: fallbackY } }
}

/**
 * Place a root node (no upstream) on the canvas.
 *
 * Empty canvas: place at ORIGIN.
 * Otherwise: place to the right of existing content.
 * Shift down by V_GAP if occupied.
 */
const placeRootNode = (
  intent: PlacementIntent,
  occupancy: readonly OccupiedRect[],
): PlacementResult => {
  const bounds = occupancyBounds(occupancy)

  const startX = bounds !== null
    ? Geometry.snapToGrid(bounds.x + bounds.width + PLACEMENT.H_GAP, PLACEMENT.GRID_SIZE)
    : PLACEMENT.ORIGIN_X

  const startY = bounds !== null
    ? Geometry.snapToGrid(bounds.y, PLACEMENT.GRID_SIZE)
    : PLACEMENT.ORIGIN_Y

  for (let row = 0; row < PLACEMENT.MAX_SCAN_ROWS; row++) {
    const candidateY = startY + row * PLACEMENT.V_GAP
    const candidate: Rect = { x: startX, y: candidateY, width: intent.width, height: intent.height }

    if (!isOccupied(candidate, occupancy)) {
      return { stepId: intent.stepId, position: { x: startX, y: candidateY } }
    }
  }

  const fallbackY = Geometry.snapToGrid(
    startY + PLACEMENT.MAX_SCAN_ROWS * PLACEMENT.V_GAP,
    PLACEMENT.GRID_SIZE,
  )
  return { stepId: intent.stepId, position: { x: startX, y: fallbackY } }
}

export { placePipelineNode, placeRootNode }
