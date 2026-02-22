import { Geometry } from '@/utils/geometry'
import type { Rect } from '@/utils/geometry'
import type { PlacementIntent, PlacementResult, OccupiedRect } from './types'
import { isOccupied } from './occupancyIndex'
import { PLACEMENT } from './constants'

// ============================================================================
// Fan-Out Placer — Vertical Stack with Convergence Target
// ============================================================================

/**
 * Place a group of fan-out siblings in a vertical stack to the right of their source.
 *
 * Algorithm:
 * 1. X = source.right + H_GAP (grid-snapped).
 * 2. Total stack height = sum of sibling heights + V_GAP between each.
 * 3. Center the stack vertically on the source node's Y midpoint.
 * 4. If ANY candidate overlaps occupancy, shift entire group down by V_GAP.
 * 5. Grid-snap all positions.
 *
 * Note: Each child position is individually grid-snapped, so the gap between
 * siblings may differ from V_GAP by up to GRID_SIZE/2 pixels.
 */
const placeFanOutGroup = (
  siblings: readonly PlacementIntent[],
  sourceRect: Rect,
  occupancy: readonly OccupiedRect[],
): readonly PlacementResult[] => {
  const columnX = Geometry.snapToGrid(
    sourceRect.x + sourceRect.width + PLACEMENT.H_GAP,
    PLACEMENT.GRID_SIZE,
  )

  // Compute total stack height
  let totalHeight = 0
  for (let i = 0; i < siblings.length; i++) {
    totalHeight += siblings[i]!.height
    if (i < siblings.length - 1) totalHeight += PLACEMENT.V_GAP
  }

  const sourceMidY = sourceRect.y + sourceRect.height / 2
  const baseTopY = Geometry.snapToGrid(
    sourceMidY - totalHeight / 2,
    PLACEMENT.GRID_SIZE,
  )

  // Build candidate positions and check for collisions as a group
  for (let shift = 0; shift < PLACEMENT.MAX_SCAN_ROWS; shift++) {
    const shiftY = shift * PLACEMENT.V_GAP
    const positions = buildStackPositions(siblings, columnX, baseTopY + shiftY)

    // Check if ANY position overlaps occupancy
    let anyOverlap = false
    for (let i = 0; i < positions.length; i++) {
      const candidate: Rect = {
        x: positions[i]!.x,
        y: positions[i]!.y,
        width: siblings[i]!.width,
        height: siblings[i]!.height,
      }
      if (isOccupied(candidate, occupancy)) {
        anyOverlap = true
        break
      }
    }

    if (!anyOverlap) {
      return siblings.map((sib, idx) => ({
        stepId: sib.stepId,
        position: positions[idx]!,
      }))
    }
  }

  // Fallback: place below all scan attempts
  const fallbackY = Geometry.snapToGrid(
    baseTopY + PLACEMENT.MAX_SCAN_ROWS * PLACEMENT.V_GAP,
    PLACEMENT.GRID_SIZE,
  )
  const positions = buildStackPositions(siblings, columnX, fallbackY)
  return siblings.map((sib, idx) => ({
    stepId: sib.stepId,
    position: positions[idx]!,
  }))
}

/**
 * Build grid-snapped Y positions for a vertical stack.
 * Each position is individually snapped to the nearest grid line.
 */
const buildStackPositions = (
  siblings: readonly PlacementIntent[],
  x: number,
  startY: number,
): Array<{ x: number; y: number }> => {
  const positions: Array<{ x: number; y: number }> = []
  let currentY = startY

  for (let i = 0; i < siblings.length; i++) {
    const snappedY = gridSnap(currentY)
    positions.push({ x, y: snappedY })
    currentY = snappedY + siblings[i]!.height + PLACEMENT.V_GAP
  }

  return positions
}

/** Grid-snap with -0 normalization. */
const gridSnap = (value: number): number => {
  const result = Geometry.snapToGrid(value, PLACEMENT.GRID_SIZE)
  // Normalize IEEE 754 -0 to +0 (occurs when rounding small negative values)
  return result === 0 ? 0 : result
}

/**
 * Place a convergence target to the right of a fan-out stack.
 *
 * Algorithm:
 * 1. X = rightmost sibling edge + H_GAP (grid-snapped).
 * 2. Y = vertical center of the sibling stack (grid-snapped).
 * 3. If occupied, shift down by V_GAP.
 */
const placeConvergenceTarget = (
  intent: PlacementIntent,
  siblingRects: readonly Rect[],
  occupancy: readonly OccupiedRect[],
): PlacementResult => {
  // Find rightmost edge of all siblings
  let maxRight = 0
  for (let i = 0; i < siblingRects.length; i++) {
    const right = siblingRects[i]!.x + siblingRects[i]!.width
    if (right > maxRight) maxRight = right
  }

  const targetX = Geometry.snapToGrid(maxRight + PLACEMENT.H_GAP, PLACEMENT.GRID_SIZE)

  // Find vertical center of the sibling stack
  const stackBounds = Geometry.boundingBox(siblingRects)
  const stackMidY = stackBounds.y + stackBounds.height / 2
  const baseY = Geometry.snapToGrid(stackMidY - intent.height / 2, PLACEMENT.GRID_SIZE)

  for (let row = 0; row < PLACEMENT.MAX_SCAN_ROWS; row++) {
    const candidateY = baseY + row * PLACEMENT.V_GAP
    const candidate: Rect = { x: targetX, y: candidateY, width: intent.width, height: intent.height }

    if (!isOccupied(candidate, occupancy)) {
      return { stepId: intent.stepId, position: { x: targetX, y: candidateY } }
    }
  }

  // Fallback
  const fallbackY = Geometry.snapToGrid(
    baseY + PLACEMENT.MAX_SCAN_ROWS * PLACEMENT.V_GAP,
    PLACEMENT.GRID_SIZE,
  )
  return { stepId: intent.stepId, position: { x: targetX, y: fallbackY } }
}

export { placeFanOutGroup, placeConvergenceTarget }
