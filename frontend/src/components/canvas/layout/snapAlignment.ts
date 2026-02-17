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

// ============================================================================
// Magnetic Snap — Snap to neighbor edges with magnetic pull
// ============================================================================

/**
 * Compute magnetic snap: when a dragged rect is within `magneticThreshold`
 * of any neighbor's edge, snap to align edges (with an optional `attachGap`
 * between them).
 *
 * This is a higher-priority snap that produces clean edge-to-edge alignment
 * between adjacent blocks. Falls back to regular grid snap when no neighbor
 * is within range.
 */
const computeMagneticSnap = (
  dragRect: Rect,
  neighbors: readonly LayoutNode[],
  gridSize: number,
  magneticThreshold: number,
  attachGap: number = 0,
): SnapResult => {
  let snappedX = dragRect.x
  let snappedY = dragRect.y
  const activeGuides: AlignmentGuide[] = []

  let bestDx = magneticThreshold + 1
  let bestDy = magneticThreshold + 1

  const dragRight = dragRect.x + dragRect.width
  const dragBottom = dragRect.y + dragRect.height
  const dragCenterX = dragRect.x + dragRect.width / 2
  const dragCenterY = dragRect.y + dragRect.height / 2

  const n = neighbors.length
  for (let i = 0; i < n; i++) {
    const neighbor = neighbors[i]!
    const nRight = neighbor.rect.x + neighbor.rect.width
    const nBottom = neighbor.rect.y + neighbor.rect.height
    const nCenterX = neighbor.rect.x + neighbor.rect.width / 2
    const nCenterY = neighbor.rect.y + neighbor.rect.height / 2

    // Horizontal snaps (x-axis)
    // Right edge of drag → left edge of neighbor (with gap)
    const d1 = Math.abs(dragRight - (neighbor.rect.x - attachGap))
    if (d1 < bestDx) { bestDx = d1; snappedX = neighbor.rect.x - attachGap - dragRect.width }

    // Left edge of drag → right edge of neighbor (with gap)
    const d2 = Math.abs(dragRect.x - (nRight + attachGap))
    if (d2 < bestDx) { bestDx = d2; snappedX = nRight + attachGap }

    // Left-to-left alignment
    const d3 = Math.abs(dragRect.x - neighbor.rect.x)
    if (d3 < bestDx) { bestDx = d3; snappedX = neighbor.rect.x }

    // Right-to-right alignment
    const d4 = Math.abs(dragRight - nRight)
    if (d4 < bestDx) { bestDx = d4; snappedX = nRight - dragRect.width }

    // Center-to-center X
    const d5 = Math.abs(dragCenterX - nCenterX)
    if (d5 < bestDx) { bestDx = d5; snappedX = nCenterX - dragRect.width / 2 }

    // Vertical snaps (y-axis)
    // Bottom of drag → top of neighbor (with gap)
    const d6 = Math.abs(dragBottom - (neighbor.rect.y - attachGap))
    if (d6 < bestDy) { bestDy = d6; snappedY = neighbor.rect.y - attachGap - dragRect.height }

    // Top of drag → bottom of neighbor (with gap)
    const d7 = Math.abs(dragRect.y - (nBottom + attachGap))
    if (d7 < bestDy) { bestDy = d7; snappedY = nBottom + attachGap }

    // Top-to-top alignment
    const d8 = Math.abs(dragRect.y - neighbor.rect.y)
    if (d8 < bestDy) { bestDy = d8; snappedY = neighbor.rect.y }

    // Bottom-to-bottom alignment
    const d9 = Math.abs(dragBottom - nBottom)
    if (d9 < bestDy) { bestDy = d9; snappedY = nBottom - dragRect.height }

    // Center-to-center Y
    const d10 = Math.abs(dragCenterY - nCenterY)
    if (d10 < bestDy) { bestDy = d10; snappedY = nCenterY - dragRect.height / 2 }
  }

  // Fall back to grid snap if no neighbor was within threshold
  if (bestDx > magneticThreshold) {
    snappedX = Math.round(dragRect.x / gridSize) * gridSize
  } else {
    activeGuides.push({ axis: 'vertical', position: snappedX, anchorNodeId: '' })
  }

  if (bestDy > magneticThreshold) {
    snappedY = Math.round(dragRect.y / gridSize) * gridSize
  } else {
    activeGuides.push({ axis: 'horizontal', position: snappedY, anchorNodeId: '' })
  }

  return { snappedX, snappedY, activeGuides }
}

export { buildAlignmentGuides, findSnapCandidates, computeSnap, computeMagneticSnap }
