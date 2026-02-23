import type { Rect } from '@/utils/geometry'
import type { AlignmentGuide, LayoutRect, SnapResult } from './types'

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
  neighbors: readonly LayoutRect[],
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
    const d1 = Math.abs(dragRight - (neighbor.rect.x - attachGap))
    if (d1 < bestDx) { bestDx = d1; snappedX = neighbor.rect.x - attachGap - dragRect.width }

    const d2 = Math.abs(dragRect.x - (nRight + attachGap))
    if (d2 < bestDx) { bestDx = d2; snappedX = nRight + attachGap }

    const d3 = Math.abs(dragRect.x - neighbor.rect.x)
    if (d3 < bestDx) { bestDx = d3; snappedX = neighbor.rect.x }

    const d4 = Math.abs(dragRight - nRight)
    if (d4 < bestDx) { bestDx = d4; snappedX = nRight - dragRect.width }

    const d5 = Math.abs(dragCenterX - nCenterX)
    if (d5 < bestDx) { bestDx = d5; snappedX = nCenterX - dragRect.width / 2 }

    // Vertical snaps (y-axis)
    const d6 = Math.abs(dragBottom - (neighbor.rect.y - attachGap))
    if (d6 < bestDy) { bestDy = d6; snappedY = neighbor.rect.y - attachGap - dragRect.height }

    const d7 = Math.abs(dragRect.y - (nBottom + attachGap))
    if (d7 < bestDy) { bestDy = d7; snappedY = nBottom + attachGap }

    const d8 = Math.abs(dragRect.y - neighbor.rect.y)
    if (d8 < bestDy) { bestDy = d8; snappedY = neighbor.rect.y }

    const d9 = Math.abs(dragBottom - nBottom)
    if (d9 < bestDy) { bestDy = d9; snappedY = nBottom - dragRect.height }

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

export { computeMagneticSnap }
