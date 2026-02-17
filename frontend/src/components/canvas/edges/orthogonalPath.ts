import { Position } from '@xyflow/react'

// ============================================================================
// Orthogonal Edge Path — Clean right-angle pipe routing
// ============================================================================

/**
 * Minimum offset (px) from the source/target node before the first bend.
 * Prevents pipes from hugging the node edge too tightly.
 */
const MIN_OFFSET = 24

/**
 * Compute an SVG path string for an orthogonal (right-angle) edge between
 * source and target positions. Produces clean Manhattan-style routing with
 * at most one intermediate bend pair.
 *
 * The path adapts its routing strategy based on the source and target handle
 * positions (which side of the node the port is on).
 */
const computeOrthogonalPath = (
  sourceX: number,
  sourceY: number,
  targetX: number,
  targetY: number,
  sourcePosition: Position,
  targetPosition: Position,
): string => {
  // Degenerate case: source and target at the same point
  if (sourceX === targetX && sourceY === targetY) {
    return `M ${sourceX} ${sourceY}`
  }

  const isSourceHorizontal = sourcePosition === Position.Left || sourcePosition === Position.Right
  const isTargetHorizontal = targetPosition === Position.Left || targetPosition === Position.Right

  // Both horizontal ports (e.g., right → left): horizontal-first routing
  if (isSourceHorizontal && isTargetHorizontal) {
    return routeHorizontalToHorizontal(sourceX, sourceY, targetX, targetY, sourcePosition, targetPosition)
  }

  // Both vertical ports (e.g., top → bottom): vertical-first routing
  if (!isSourceHorizontal && !isTargetHorizontal) {
    return routeVerticalToVertical(sourceX, sourceY, targetX, targetY, sourcePosition, targetPosition)
  }

  // Mixed: horizontal source → vertical target (or vice versa): L-shaped
  return routeMixed(sourceX, sourceY, targetX, targetY, isSourceHorizontal)
}

/**
 * Route between two horizontal ports (left/right).
 * Typical for spine edges (protocol → protocol).
 */
const routeHorizontalToHorizontal = (
  sx: number, sy: number,
  tx: number, ty: number,
  sourcePos: Position, targetPos: Position,
): string => {
  const sourceDir = sourcePos === Position.Right ? 1 : -1
  const targetDir = targetPos === Position.Left ? -1 : 1

  // Simple case: source right of left, target left of right, flowing left-to-right
  if (sourcePos === Position.Right && targetPos === Position.Left && tx > sx) {
    if (sy === ty) {
      // Direct horizontal line
      return `M ${sx} ${sy} L ${tx} ${ty}`
    }
    // 3-segment: horizontal → vertical → horizontal
    const midX = (sx + tx) / 2
    return `M ${sx} ${sy} L ${midX} ${sy} L ${midX} ${ty} L ${tx} ${ty}`
  }

  // Awkward case: need to route around (e.g., both facing right, or target is behind source)
  const offsetX1 = sx + sourceDir * MIN_OFFSET
  const offsetX2 = tx + targetDir * MIN_OFFSET
  const midY = (sy + ty) / 2

  return `M ${sx} ${sy} L ${offsetX1} ${sy} L ${offsetX1} ${midY} L ${offsetX2} ${midY} L ${offsetX2} ${ty} L ${tx} ${ty}`
}

/**
 * Route between two vertical ports (top/bottom).
 * Typical for tower edges (protocol ↔ agent).
 */
const routeVerticalToVertical = (
  sx: number, sy: number,
  tx: number, ty: number,
  sourcePos: Position, targetPos: Position,
): string => {
  const sourceDir = sourcePos === Position.Bottom ? 1 : -1
  const targetDir = targetPos === Position.Top ? -1 : 1

  // Simple case: source bottom, target top, target below source
  if (sourcePos === Position.Bottom && targetPos === Position.Top && ty > sy) {
    if (sx === tx) {
      // Direct vertical line
      return `M ${sx} ${sy} L ${tx} ${ty}`
    }
    // 3-segment: vertical → horizontal → vertical
    const midY = (sy + ty) / 2
    return `M ${sx} ${sy} L ${sx} ${midY} L ${tx} ${midY} L ${tx} ${ty}`
  }

  // Simple case: source top, target bottom, target above source
  if (sourcePos === Position.Top && targetPos === Position.Bottom && ty < sy) {
    if (sx === tx) {
      return `M ${sx} ${sy} L ${tx} ${ty}`
    }
    const midY = (sy + ty) / 2
    return `M ${sx} ${sy} L ${sx} ${midY} L ${tx} ${midY} L ${tx} ${ty}`
  }

  // Awkward case
  const offsetY1 = sy + sourceDir * MIN_OFFSET
  const offsetY2 = ty + targetDir * MIN_OFFSET
  const midX = (sx + tx) / 2

  return `M ${sx} ${sy} L ${sx} ${offsetY1} L ${midX} ${offsetY1} L ${midX} ${offsetY2} L ${tx} ${offsetY2} L ${tx} ${ty}`
}

/**
 * Route between a horizontal port and a vertical port (or vice versa).
 * Produces an L-shaped path — one horizontal segment + one vertical segment.
 */
const routeMixed = (
  sx: number, sy: number,
  tx: number, ty: number,
  isSourceHorizontal: boolean,
): string => {
  if (isSourceHorizontal) {
    // Source exits horizontally, target enters vertically → corner at (tx, sy)
    return `M ${sx} ${sy} L ${tx} ${sy} L ${tx} ${ty}`
  }
  // Source exits vertically, target enters horizontally → corner at (sx, ty)
  return `M ${sx} ${sy} L ${sx} ${ty} L ${tx} ${ty}`
}

// ============================================================================
// Edge Label Position — midpoint of the path
// ============================================================================

/**
 * Compute the label position for an orthogonal edge.
 * Returns the midpoint between source and target (good enough for label placement).
 */
const computeOrthogonalLabel = (
  sourceX: number,
  sourceY: number,
  targetX: number,
  targetY: number,
): { labelX: number; labelY: number } => ({
  labelX: (sourceX + targetX) / 2,
  labelY: (sourceY + targetY) / 2,
})

// ============================================================================
// Parallel Track Assignment
// ============================================================================

/**
 * Compute perpendicular offsets for parallel pipes sharing a corridor.
 * Centers the group around 0 with `spacing` between each pipe.
 *
 * Example: 3 pipes with spacing 8 → offsets [-8, 0, 8]
 */
const assignParallelTracks = (
  edgeCount: number,
  spacing: number,
): number[] => {
  if (edgeCount <= 0) return []
  if (edgeCount === 1) return [0]

  const totalWidth = (edgeCount - 1) * spacing
  const offsets: number[] = []
  for (let i = 0; i < edgeCount; i++) {
    offsets.push(-totalWidth / 2 + i * spacing)
  }
  return offsets
}

export { computeOrthogonalPath, computeOrthogonalLabel, assignParallelTracks }
