import { Geometry } from '@/utils/geometry'
import type { Rect, Side } from '@/utils/geometry'
import type { LayoutRect, Overlap } from './types'

/**
 * Compute push direction and distance to resolve an overlap.
 * Pushes along the axis with smaller intersection (easier to resolve),
 * away from the moved rect toward the other rect.
 */
const computePushVector = (
  movedRect: Rect,
  otherRect: Rect,
  intersection: Rect,
): { pushDirection: Side; pushDistance: number } => {
  const dx = Geometry.rectCenter(otherRect).x - Geometry.rectCenter(movedRect).x
  const dy = Geometry.rectCenter(otherRect).y - Geometry.rectCenter(movedRect).y

  if (intersection.width <= intersection.height) {
    return {
      pushDirection: dx >= 0 ? 'right' : 'left',
      pushDistance: intersection.width,
    }
  }
  return {
    pushDirection: dy >= 0 ? 'bottom' : 'top',
    pushDistance: intersection.height,
  }
}

/**
 * Detect all rectangles that overlap with a moved/resized rect.
 * For each overlap, computes the push direction (away from the moved rect's
 * center) and the minimum push distance to resolve the overlap.
 */
const detectOverlaps = (
  movedRect: Rect,
  movedId: string,
  others: readonly LayoutRect[],
): readonly Overlap[] => {
  const overlaps: Overlap[] = []
  const n = others.length

  for (let i = 0; i < n; i++) {
    const other = others[i]!
    if (other.id === movedId) continue

    const intersection = Geometry.rectsIntersection(movedRect, other.rect)
    if (!intersection) continue

    const { pushDirection, pushDistance } = computePushVector(movedRect, other.rect, intersection)

    overlaps.push({
      nodeId: other.id,
      overlapRect: intersection,
      pushDirection,
      pushDistance,
    })
  }

  return overlaps
}

export { detectOverlaps }
