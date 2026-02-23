import type { Rect } from '@/utils/geometry'
import type { AlignmentGuide, SnapCandidate, SnapResult } from './types'

/**
 * Compute the snapped position for a dragged rect given snap candidates.
 * Picks the closest candidate on each axis independently.
 * Returns the adjusted position and the active guide lines for rendering.
 */
const computeSnap = (
  dragRect: Rect,
  candidates: readonly SnapCandidate[],
): SnapResult => {
  let snappedX = dragRect.x
  let snappedY = dragRect.y
  const activeGuides: AlignmentGuide[] = []

  let bestVertical: SnapCandidate | null = null
  let bestHorizontal: SnapCandidate | null = null

  const n = candidates.length
  for (let i = 0; i < n; i++) {
    const c = candidates[i]!
    if (c.guide.axis === 'vertical' && !bestVertical) {
      bestVertical = c
    } else if (c.guide.axis === 'horizontal' && !bestHorizontal) {
      bestHorizontal = c
    }
    if (bestVertical && bestHorizontal) break
  }

  if (bestVertical) {
    const pos = bestVertical.guide.position
    switch (bestVertical.snapEdge) {
      case 'start':  snappedX = pos; break
      case 'end':    snappedX = pos - dragRect.width; break
      case 'center': snappedX = pos - dragRect.width / 2; break
    }
    activeGuides.push(bestVertical.guide)
  }

  if (bestHorizontal) {
    const pos = bestHorizontal.guide.position
    switch (bestHorizontal.snapEdge) {
      case 'start':  snappedY = pos; break
      case 'end':    snappedY = pos - dragRect.height; break
      case 'center': snappedY = pos - dragRect.height / 2; break
    }
    activeGuides.push(bestHorizontal.guide)
  }

  return { snappedX, snappedY, activeGuides }
}

export { computeSnap }
