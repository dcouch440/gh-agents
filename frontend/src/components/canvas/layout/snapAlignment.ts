import type { Rect } from '@/utils/geometry'
import type { AlignmentGuide, LayoutNode, SnapCandidate, SnapEdge, SnapResult } from './types'

// ============================================================================
// Snap Alignment — Guide Generation and Snap Computation
// ============================================================================

/**
 * Build alignment guides from a set of nodes. Each node emits 6 guides:
 * left edge, right edge, center-x (vertical axis),
 * top edge, bottom edge, center-y (horizontal axis).
 *
 * Nodes in `excludeIds` are skipped (typically the node being dragged).
 */
const buildAlignmentGuides = (
  nodes: readonly LayoutNode[],
  excludeIds: ReadonlySet<string>,
): readonly AlignmentGuide[] => {
  const guides: AlignmentGuide[] = []
  const n = nodes.length

  for (let i = 0; i < n; i++) {
    const node = nodes[i]!
    if (excludeIds.has(node.id)) continue

    const { x, y, width, height } = node.rect

    // Vertical guides (x-axis values)
    guides.push({ axis: 'vertical', position: x, anchorNodeId: node.id })
    guides.push({ axis: 'vertical', position: x + width, anchorNodeId: node.id })
    guides.push({ axis: 'vertical', position: x + width / 2, anchorNodeId: node.id })

    // Horizontal guides (y-axis values)
    guides.push({ axis: 'horizontal', position: y, anchorNodeId: node.id })
    guides.push({ axis: 'horizontal', position: y + height, anchorNodeId: node.id })
    guides.push({ axis: 'horizontal', position: y + height / 2, anchorNodeId: node.id })
  }

  return guides
}

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
  candidates.sort((a, b) => a.distance - b.distance)

  return candidates
}

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

export { buildAlignmentGuides, findSnapCandidates, computeSnap }
