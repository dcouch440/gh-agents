import { Collections } from '@/utils/collections'
import type { Rect } from '@/utils/geometry'
import type { AlignmentGuide, SnapCandidate, SnapEdge } from './types'

/** Determine which edge of the drag rect is closest to a guide position. */
const closestEdge = (start: number, end: number, center: number, guidePos: number): { edge: SnapEdge; distance: number } => {
  const dStart = Math.abs(start - guidePos)
  const dEnd = Math.abs(end - guidePos)
  const dCenter = Math.abs(center - guidePos)
  const min = Math.min(dStart, dEnd, dCenter)

  if (min === dStart) return { edge: 'start', distance: dStart }
  if (min === dEnd) return { edge: 'end', distance: dEnd }
  return { edge: 'center', distance: dCenter }
}

/**
 * Find all guides within `threshold` distance of the dragged rect's
 * corresponding edges/center. Returns candidates sorted by distance (ascending).
 *
 * For vertical guides: checks against drag rect's left, right, and center-x.
 * For horizontal guides: checks against drag rect's top, bottom, and center-y.
 */
const findSnapCandidates = (
  dragRect: Rect,
  guides: readonly AlignmentGuide[],
  threshold: number,
): readonly SnapCandidate[] => {
  const candidates: SnapCandidate[] = []
  const n = guides.length

  const dragLeft = dragRect.x
  const dragRight = dragRect.x + dragRect.width
  const dragCenterX = dragRect.x + dragRect.width / 2
  const dragTop = dragRect.y
  const dragBottom = dragRect.y + dragRect.height
  const dragCenterY = dragRect.y + dragRect.height / 2

  for (let i = 0; i < n; i++) {
    const guide = guides[i]!

    const result = guide.axis === 'vertical'
      ? closestEdge(dragLeft, dragRight, dragCenterX, guide.position)
      : closestEdge(dragTop, dragBottom, dragCenterY, guide.position)

    if (result.distance <= threshold) {
      candidates.push({ guide, distance: result.distance, snapEdge: result.edge })
    }
  }

  // Sort by distance ascending
  return Collections.sortedCopy(candidates, (a, b) => a.distance - b.distance)
}

export { findSnapCandidates }
