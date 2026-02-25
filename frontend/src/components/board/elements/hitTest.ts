// ============================================================================
// Hit Testing — Point-in-Element and Anchor Zone Detection
// ============================================================================

import { Geometry } from '@/utils/geometry'
import type { Point, Rect, Side } from '@/utils/geometry'
import { BOARD } from '../constants'
import type { AnchorPoint, BoardElements, BoxElement } from './types'

/**
 * Returns the element ID under the given canvas point, or null.
 *
 * Checks boxes in reverse z-order (frontmost first), then arrows.
 * Arrow hit testing is skipped for now — arrows are selected by
 * clicking their connected box or via keyboard.
 */
const hitTest = (state: BoardElements, point: Point): string | null => {
  const boxId = hitTestBox(state, point)
  if (boxId !== null) return boxId
  return null
}

/**
 * Returns the box ID under the canvas point, or null.
 * Checks in reverse boxOrder (frontmost first).
 */
const hitTestBox = (state: BoardElements, point: Point): string | null => {
  for (let i = state.boxOrder.length - 1; i >= 0; i--) {
    const boxId = state.boxOrder[i]!
    const box = state.boxes.get(boxId)
    if (box !== undefined && Geometry.rectContainsPoint(box, point)) {
      return boxId
    }
  }
  return null
}

/**
 * Determine the best anchor point on a box for an arrow binding.
 *
 * Finds the nearest side and computes the ratio along it. If the
 * pointer is within `ARROW_SNAP_THRESHOLD` of the side midpoint,
 * snaps to ratio 0.5 (Excalidraw-style magnetic midpoint snapping).
 *
 * Ratio is clamped to [0.1, 0.9] to avoid corners.
 */
const hitTestBoxAnchor = (box: BoxElement, point: Point): AnchorPoint => {
  const side = Geometry.nearestSide(box, point)
  const midpoint = Geometry.pointAlongSide(box, side, 0.5)
  const distToMid = Geometry.distanceBetweenPoints(point, midpoint)

  if (distToMid <= BOARD.ARROW_SNAP_THRESHOLD) {
    return { side, ratio: 0.5 }
  }

  const ratio = computeRatioAlongSide(box, side, point)
  return { side, ratio: Geometry.clamp(ratio, 0.1, 0.9) }
}

/**
 * Returns the best anchor for an arrow arriving at a target box from
 * a given source point. Picks the side facing the source.
 */
const computeTargetAnchor = (box: BoxElement, sourcePoint: Point): AnchorPoint => {
  const center = Geometry.rectCenter(box)
  const dx = sourcePoint.x - center.x
  const dy = sourcePoint.y - center.y

  let side: Side
  if (Math.abs(dx) > Math.abs(dy)) {
    side = dx > 0 ? 'right' : 'left'
  } else {
    side = dy > 0 ? 'bottom' : 'top'
  }

  return { side, ratio: 0.5 }
}

/**
 * Returns all element IDs whose bounding boxes intersect a selection rectangle.
 */
const hitTestRect = (state: BoardElements, rect: Rect): string[] => {
  const ids: string[] = []

  for (const [boxId, box] of state.boxes) {
    if (Geometry.rectsOverlap(rect, box)) {
      ids.push(boxId)
    }
  }

  // Include arrows whose source and target are both in the selection
  for (const [arrowId, arrow] of state.arrows) {
    if (ids.includes(arrow.sourceBoxId) && ids.includes(arrow.targetBoxId)) {
      ids.push(arrowId)
    }
  }

  return ids
}

/**
 * Check if a point is near a box edge (within handle hover distance).
 * Returns the side if within range, null otherwise.
 */
const hitTestBoxEdge = (box: BoxElement, point: Point, threshold: number): Side | null => {
  const sides: readonly Side[] = ['top', 'right', 'bottom', 'left']
  for (let i = 0; i < sides.length; i++) {
    const side = sides[i]!
    const mid = Geometry.pointAlongSide(box, side, 0.5)
    if (Geometry.distanceBetweenPoints(point, mid) <= threshold) {
      return side
    }
  }
  return null
}

// ── Helpers ────────────────────────────────────────────────────────────────

const computeRatioAlongSide = (box: BoxElement, side: Side, point: Point): number => {
  switch (side) {
    case 'top':
    case 'bottom':
      return (point.x - box.x) / box.width
    case 'left':
    case 'right':
      return (point.y - box.y) / box.height
  }
}

export { computeTargetAnchor, hitTest, hitTestBox, hitTestBoxAnchor, hitTestBoxEdge, hitTestRect }
