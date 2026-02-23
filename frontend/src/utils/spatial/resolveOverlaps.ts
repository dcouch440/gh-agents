import { Geometry } from '@/utils/geometry'
import type { Point, Rect, Side } from '@/utils/geometry'
import type { Overlap } from './types'

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
 * Resolve overlaps by computing new positions for pushed rectangles.
 * Handles cascading pushes (A pushes B, B pushes C) up to `maxDepth`.
 * All resolved positions are snapped to the grid directionally (away from
 * the push source) to guarantee full overlap clearance.
 *
 * Returns a map of id → new position for all rectangles that need to move.
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

      // Update the rect for cascade detection
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

        const dx = Geometry.rectCenter(otherRect).x - Geometry.rectCenter(newRect).x
        const dy = Geometry.rectCenter(otherRect).y - Geometry.rectCenter(newRect).y

        let pushDirection: Side
        let pushDistance: number
        if (intersection.width <= intersection.height) {
          pushDirection = dx >= 0 ? 'right' : 'left'
          pushDistance = intersection.width
        } else {
          pushDirection = dy >= 0 ? 'bottom' : 'top'
          pushDistance = intersection.height
        }

        cascadeOverlaps.push({
          nodeId: otherId,
          overlapRect: intersection,
          pushDirection,
          pushDistance,
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

export { resolveOverlaps }
