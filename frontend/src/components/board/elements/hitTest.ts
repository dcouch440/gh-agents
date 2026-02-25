// ============================================================================
// Hit Testing — Point-in-Element and Anchor Zone Detection
// ============================================================================

import { Geometry } from '@/utils/geometry'
import type { Point, Rect } from '@/utils/geometry'
import type { BoardElements } from './types'

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
  const idSet = new Set(ids)
  for (const [arrowId, arrow] of state.arrows) {
    if (idSet.has(arrow.sourceBoxId) && idSet.has(arrow.targetBoxId)) {
      ids.push(arrowId)
    }
  }

  return ids
}

/**
 * Returns a Set of all element IDs (boxes + arrows).
 */
const selectAllIds = (state: BoardElements): Set<string> => {
  const ids = new Set<string>()
  for (const id of state.boxes.keys()) ids.add(id)
  for (const id of state.arrows.keys()) ids.add(id)
  return ids
}

export { hitTest, hitTestBox, hitTestRect, selectAllIds }
