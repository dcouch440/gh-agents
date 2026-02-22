import { Geometry } from '@/utils/geometry'
import type { Rect } from '@/utils/geometry'
import type { PlacementIntent, PlacementResult, PlacementShift, OccupiedRect, SpliceResult } from './types'
import { isOccupied } from './occupancyIndex'
import { PLACEMENT } from './constants'

// ============================================================================
// Splice Placer — Insert Between Two Placed Nodes
// ============================================================================

/**
 * Place a node between two placed nodes (splice / insert-between).
 *
 * Returns the new node's position and optionally a shift for the downstream node.
 *
 * CASE 1: Enough gap — place centered in the gap.
 * CASE 2: Not enough gap, downstream shiftable — shift downstream right, place in gap.
 * CASE 3: Not enough gap, downstream not shiftable — place above or below the edge line.
 */
const placeSpliceNode = (
  intent: PlacementIntent,
  upstreamRect: Rect,
  downstreamRect: Rect,
  downstreamIsShiftable: boolean,
  occupancy: readonly OccupiedRect[],
): SpliceResult => {
  const upstreamRight = upstreamRect.x + upstreamRect.width
  const gap = downstreamRect.x - upstreamRight
  const needed = PLACEMENT.H_GAP + intent.width + PLACEMENT.H_GAP

  if (gap >= needed) {
    return placeInGap(intent, upstreamRect, downstreamRect, occupancy)
  }

  if (downstreamIsShiftable) {
    return placeWithShift(intent, upstreamRect, downstreamRect, gap, needed, occupancy)
  }

  return placeOffEdgeLine(intent, upstreamRect, downstreamRect, occupancy)
}

/**
 * CASE 1: Enough gap — place to the right of upstream with H_GAP.
 */
const placeInGap = (
  intent: PlacementIntent,
  upstreamRect: Rect,
  downstreamRect: Rect,
  occupancy: readonly OccupiedRect[],
): SpliceResult => {
  const x = Geometry.snapToGrid(
    upstreamRect.x + upstreamRect.width + PLACEMENT.H_GAP,
    PLACEMENT.GRID_SIZE,
  )

  const edgeMidY = (upstreamRect.y + upstreamRect.height / 2 + downstreamRect.y + downstreamRect.height / 2) / 2
  const baseY = Geometry.snapToGrid(edgeMidY - intent.height / 2, PLACEMENT.GRID_SIZE)

  const placement = scanFreeY(intent, x, baseY, occupancy)
  return { placement, shift: null }
}

/**
 * CASE 2: Not enough gap, downstream is shiftable — shift it right and place in gap.
 */
const placeWithShift = (
  intent: PlacementIntent,
  upstreamRect: Rect,
  downstreamRect: Rect,
  gap: number,
  needed: number,
  occupancy: readonly OccupiedRect[],
): SpliceResult => {
  const rawShift = needed - gap
  const shiftDx = Math.max(
    Geometry.snapToGrid(rawShift, PLACEMENT.GRID_SIZE),
    PLACEMENT.GRID_SIZE,
  )

  const x = Geometry.snapToGrid(
    upstreamRect.x + upstreamRect.width + PLACEMENT.H_GAP,
    PLACEMENT.GRID_SIZE,
  )
  const baseY = Geometry.snapToGrid(upstreamRect.y, PLACEMENT.GRID_SIZE)

  // Check occupancy excluding the downstream node (it's moving)
  const placement = scanFreeY(intent, x, baseY, occupancy, intent.spliceDownstreamId ?? undefined)

  const shift: PlacementShift = {
    stepId: intent.spliceDownstreamId!,
    dx: shiftDx,
    dy: 0,
  }

  return { placement, shift }
}

/**
 * CASE 3: Not enough gap, downstream not shiftable — place above or below edge line.
 */
const placeOffEdgeLine = (
  intent: PlacementIntent,
  upstreamRect: Rect,
  downstreamRect: Rect,
  occupancy: readonly OccupiedRect[],
): SpliceResult => {
  const x = Geometry.snapToGrid(
    upstreamRect.x + upstreamRect.width + PLACEMENT.H_GAP,
    PLACEMENT.GRID_SIZE,
  )

  // Try above: above the higher of the two nodes
  const minTop = Math.min(upstreamRect.y, downstreamRect.y)
  const aboveY = Geometry.snapToGrid(minTop - intent.height - PLACEMENT.V_GAP, PLACEMENT.GRID_SIZE)
  const aboveCandidate: Rect = { x, y: aboveY, width: intent.width, height: intent.height }

  if (!isOccupied(aboveCandidate, occupancy)) {
    return {
      placement: { stepId: intent.stepId, position: { x, y: aboveY } },
      shift: null,
    }
  }

  // Try below: below the lower of the two nodes
  const maxBottom = Math.max(
    upstreamRect.y + upstreamRect.height,
    downstreamRect.y + downstreamRect.height,
  )
  const belowY = Geometry.snapToGrid(maxBottom + PLACEMENT.V_GAP, PLACEMENT.GRID_SIZE)
  const belowCandidate: Rect = { x, y: belowY, width: intent.width, height: intent.height }

  if (!isOccupied(belowCandidate, occupancy)) {
    return {
      placement: { stepId: intent.stepId, position: { x, y: belowY } },
      shift: null,
    }
  }

  // Fallback: scan down from belowY
  const placement = scanFreeY(intent, x, belowY + PLACEMENT.V_GAP, occupancy)
  return { placement, shift: null }
}

/**
 * Scan downward from baseY by V_GAP increments to find an unoccupied position.
 */
const scanFreeY = (
  intent: PlacementIntent,
  x: number,
  baseY: number,
  occupancy: readonly OccupiedRect[],
  excludeId?: string,
): PlacementResult => {
  for (let row = 0; row < PLACEMENT.MAX_SCAN_ROWS; row++) {
    const candidateY = baseY + row * PLACEMENT.V_GAP
    const candidate: Rect = { x, y: candidateY, width: intent.width, height: intent.height }

    if (!isOccupied(candidate, occupancy, excludeId)) {
      return { stepId: intent.stepId, position: { x, y: candidateY } }
    }
  }

  // Fallback
  const fallbackY = Geometry.snapToGrid(
    baseY + PLACEMENT.MAX_SCAN_ROWS * PLACEMENT.V_GAP,
    PLACEMENT.GRID_SIZE,
  )
  return { stepId: intent.stepId, position: { x, y: fallbackY } }
}

export { placeSpliceNode }
