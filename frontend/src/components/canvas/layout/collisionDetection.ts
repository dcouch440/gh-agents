import { Geometry } from '@/utils/geometry'
import type { Point, Rect, Side } from '@/utils/geometry'
import type { LayoutNode, Overlap } from './types'

// ============================================================================
// Collision Detection — Overlap Detection and Resolution
// ============================================================================

/**
 * Compute push direction and distance to resolve an overlap.
 * Pushes along the axis with smaller intersection (easier to resolve),
 * away from `movedCenter` toward `otherCenter`.
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
 * Snap a pushed position to the grid directionally — always snapping AWAY
 * from the source of the push to guarantee full overlap clearance.
 */
const snapDirectional = (value: number, pushDirection: Side, gridSize: number): number => {
  switch (pushDirection) {
    case 'right':
    case 'bottom':
      return Math.ceil(value / gridSize) * gridSize
    case 'left':
    case 'top':
      return Math.floor(value / gridSize) * gridSize
  }
}

/**
 * Detect all nodes that overlap with a moved/resized rect.
 * For each overlap, computes the push direction (away from the moved rect's
 * center) and the minimum push distance to resolve the overlap.
 */
const detectOverlaps = (
  movedRect: Rect,
  movedNodeId: string,
  others: readonly LayoutNode[],
): readonly Overlap[] => {
  const overlaps: Overlap[] = []
  const n = others.length

  for (let i = 0; i < n; i++) {
    const other = others[i]!
    if (other.id === movedNodeId) continue

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

/**
 * Resolve overlaps by computing new positions for pushed nodes.
 * Handles cascading pushes (A pushes B, B pushes C) up to `maxDepth`.
 * All resolved positions are snapped to the grid directionally (away from
 * the push source) to guarantee full overlap clearance.
 *
 * Returns a map of nodeId → new position for all nodes that need to move.
 */
const resolveOverlaps = (
  overlaps: readonly Overlap[],
  allNodes: ReadonlyMap<string, Rect>,
  gridSize: number,
  maxDepth: number = 3,
): ReadonlyMap<string, Point> => {
  const resolved = new Map<string, Point>()

  const resolveRecursive = (
    currentOverlaps: readonly Overlap[],
    currentNodes: ReadonlyMap<string, Rect>,
    depth: number,
  ): void => {
    if (depth > maxDepth || currentOverlaps.length === 0) return

    const updatedNodes = new Map(currentNodes)
    const cascadeOverlaps: Overlap[] = []

    const n = currentOverlaps.length
    for (let i = 0; i < n; i++) {
      const overlap = currentOverlaps[i]!
      const nodeRect = updatedNodes.get(overlap.nodeId)
      if (!nodeRect) continue

      let newX = nodeRect.x
      let newY = nodeRect.y

      switch (overlap.pushDirection) {
        case 'right':  newX = nodeRect.x + overlap.pushDistance; break
        case 'left':   newX = nodeRect.x - overlap.pushDistance; break
        case 'bottom': newY = nodeRect.y + overlap.pushDistance; break
        case 'top':    newY = nodeRect.y - overlap.pushDistance; break
      }

      // Snap directionally — always away from the push source
      const snappedX = (overlap.pushDirection === 'left' || overlap.pushDirection === 'right')
        ? snapDirectional(newX, overlap.pushDirection, gridSize)
        : Geometry.snapToGrid(newX, gridSize)
      const snappedY = (overlap.pushDirection === 'top' || overlap.pushDirection === 'bottom')
        ? snapDirectional(newY, overlap.pushDirection, gridSize)
        : Geometry.snapToGrid(newY, gridSize)

      resolved.set(overlap.nodeId, { x: snappedX, y: snappedY })

      // Update the node's rect for cascade detection
      const newRect: Rect = {
        x: snappedX,
        y: snappedY,
        width: nodeRect.width,
        height: nodeRect.height,
      }
      updatedNodes.set(overlap.nodeId, newRect)

      // Check for new overlaps caused by the push
      for (const [otherId, otherRect] of updatedNodes) {
        if (otherId === overlap.nodeId) continue
        if (resolved.has(otherId)) continue

        const intersection = Geometry.rectsIntersection(newRect, otherRect)
        if (!intersection) continue

        const push = computePushVector(newRect, otherRect, intersection)

        cascadeOverlaps.push({
          nodeId: otherId,
          overlapRect: intersection,
          pushDirection: push.pushDirection,
          pushDistance: push.pushDistance,
        })
      }
    }

    if (cascadeOverlaps.length > 0) {
      resolveRecursive(cascadeOverlaps, updatedNodes, depth + 1)
    }
  }

  resolveRecursive(overlaps, allNodes, 1)

  return resolved
}

export { detectOverlaps, resolveOverlaps }
