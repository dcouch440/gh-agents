import { Position } from '@xyflow/react'

// ============================================================================
// Bezier Edge Path — Smooth cubic curves between nodes
// ============================================================================

/**
 * Minimum control point offset (px) from the handle.
 * Prevents flat curves when nodes are very close together.
 */
const MIN_OFFSET = 50

/**
 * Maximum control point offset as a fraction of the distance between handles.
 * Prevents extreme bulging when nodes are far apart.
 */
const MAX_RATIO = 0.4

/**
 * Compute the direction vector for a handle position.
 * Returns a unit vector pointing outward from the node face.
 */
const handleDirection = (position: Position): { dx: number; dy: number } => {
  switch (position) {
    case Position.Left:   return { dx: -1, dy: 0 }
    case Position.Right:  return { dx: 1, dy: 0 }
    case Position.Top:    return { dx: 0, dy: -1 }
    case Position.Bottom: return { dx: 0, dy: 1 }
  }
}

/**
 * Compute a cubic Bezier SVG path between source and target handle positions.
 *
 * Control points extend in the direction of each handle, with offset that
 * scales with the distance between nodes — clamped between MIN_OFFSET and
 * MAX_RATIO * distance. This produces natural S-curves that adapt to any
 * node arrangement.
 */
const computeBezierPath = (
  sourceX: number,
  sourceY: number,
  targetX: number,
  targetY: number,
  sourcePosition: Position,
  targetPosition: Position,
): string => {
  // Degenerate case
  if (sourceX === targetX && sourceY === targetY) {
    return `M ${sourceX} ${sourceY}`
  }

  const distance = Math.sqrt((targetX - sourceX) ** 2 + (targetY - sourceY) ** 2)
  const offset = Math.max(MIN_OFFSET, Math.min(distance * MAX_RATIO, distance))

  const srcDir = handleDirection(sourcePosition)
  const tgtDir = handleDirection(targetPosition)

  const cp1x = sourceX + srcDir.dx * offset
  const cp1y = sourceY + srcDir.dy * offset
  const cp2x = targetX + tgtDir.dx * offset
  const cp2y = targetY + tgtDir.dy * offset

  return `M ${sourceX} ${sourceY} C ${cp1x} ${cp1y} ${cp2x} ${cp2y} ${targetX} ${targetY}`
}

/**
 * Compute the label position for a Bezier edge.
 * Uses the midpoint of the cubic Bezier curve (t = 0.5).
 */
const computeBezierLabel = (
  sourceX: number,
  sourceY: number,
  targetX: number,
  targetY: number,
  sourcePosition: Position,
  targetPosition: Position,
): { labelX: number; labelY: number } => {
  const distance = Math.sqrt((targetX - sourceX) ** 2 + (targetY - sourceY) ** 2)
  const offset = Math.max(MIN_OFFSET, Math.min(distance * MAX_RATIO, distance))

  const srcDir = handleDirection(sourcePosition)
  const tgtDir = handleDirection(targetPosition)

  const cp1x = sourceX + srcDir.dx * offset
  const cp1y = sourceY + srcDir.dy * offset
  const cp2x = targetX + tgtDir.dx * offset
  const cp2y = targetY + tgtDir.dy * offset

  // Cubic Bezier at t=0.5: B(0.5) = 0.125*P0 + 0.375*CP1 + 0.375*CP2 + 0.125*P3
  const t = 0.5
  const mt = 1 - t
  const labelX = mt * mt * mt * sourceX + 3 * mt * mt * t * cp1x + 3 * mt * t * t * cp2x + t * t * t * targetX
  const labelY = mt * mt * mt * sourceY + 3 * mt * mt * t * cp1y + 3 * mt * t * t * cp2y + t * t * t * targetY

  return { labelX, labelY }
}

export { computeBezierPath, computeBezierLabel }
