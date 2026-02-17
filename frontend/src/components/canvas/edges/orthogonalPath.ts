import { Position } from '@xyflow/react'
import { roundCorners } from './roundCorners'

// ============================================================================
// Orthogonal Edge Path — Clean right-angle pipe routing
// ============================================================================

/**
 * Minimum offset (px) from the source/target node before the first bend.
 * Prevents pipes from hugging the node edge too tightly.
 */
const MIN_OFFSET = 24

/**
 * Snap threshold (px) — treat handle positions within this range as aligned.
 * Eliminates visible kinks from sub-pixel rounding or small resize differences.
 */
const SNAP_TOLERANCE = 8

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

  let path: string

  // Both horizontal ports (e.g., right → left): horizontal-first routing
  if (isSourceHorizontal && isTargetHorizontal) {
    path = routeHorizontalToHorizontal(sourceX, sourceY, targetX, targetY, sourcePosition, targetPosition)
  } else if (!isSourceHorizontal && !isTargetHorizontal) {
    // Both vertical ports (e.g., top → bottom): vertical-first routing
    path = routeVerticalToVertical(sourceX, sourceY, targetX, targetY, sourcePosition, targetPosition)
  } else {
    // Mixed: horizontal source → vertical target (or vice versa)
    path = routeMixed(sourceX, sourceY, targetX, targetY, sourcePosition, targetPosition)
  }

  return roundCorners(path)
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
    if (Math.abs(sy - ty) <= SNAP_TOLERANCE) {
      // Direct horizontal line (within snap tolerance)
      return `M ${sx} ${sy} L ${tx} ${sy}`
    }
    // 3-segment: horizontal → vertical → horizontal
    // Clamp midX to guarantee MIN_OFFSET from both handles
    const midX = Math.max(sx + MIN_OFFSET, Math.min(tx - MIN_OFFSET, (sx + tx) / 2))
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
    if (Math.abs(sx - tx) <= SNAP_TOLERANCE) {
      // Direct vertical line (within snap tolerance)
      return `M ${sx} ${sy} L ${sx} ${ty}`
    }
    // 3-segment: route horizontal near source to stay in the gap between tiers
    const midY = sy + MIN_OFFSET
    return `M ${sx} ${sy} L ${sx} ${midY} L ${tx} ${midY} L ${tx} ${ty}`
  }

  // Simple case: source top, target bottom, target above source
  if (sourcePos === Position.Top && targetPos === Position.Bottom && ty < sy) {
    if (Math.abs(sx - tx) <= SNAP_TOLERANCE) {
      return `M ${sx} ${sy} L ${sx} ${ty}`
    }
    // 3-segment: route horizontal near target to stay in the gap between tiers
    const midY = ty + MIN_OFFSET
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
 * Ensures the pipe exits in the source handle's direction and enters in
 * the target handle's direction, with MIN_OFFSET stubs at both ends.
 */
const routeMixed = (
  sx: number, sy: number,
  tx: number, ty: number,
  sourcePosition: Position,
  targetPosition: Position,
): string => {
  const isSourceHorizontal = sourcePosition === Position.Left || sourcePosition === Position.Right

  // Target entry direction: offset from target handle before the final straight entry
  const targetEntryDir = targetPosition === Position.Left ? -1
    : targetPosition === Position.Right ? 1
    : targetPosition === Position.Top ? -1
    : 1 // Bottom

  if (isSourceHorizontal) {
    // Source exits horizontally, target enters vertically (Top/Bottom)
    const goesRight = sourcePosition === Position.Right
    const targetIsAhead = goesRight ? tx >= sx : tx <= sx

    // Vertical entry stub: approach target from MIN_OFFSET above (Top) or below (Bottom)
    const entryY = ty + targetEntryDir * MIN_OFFSET

    if (targetIsAhead) {
      const exitX = sx + (goesRight ? 1 : -1) * MIN_OFFSET
      return `M ${sx} ${sy} L ${exitX} ${sy} L ${exitX} ${entryY} L ${tx} ${entryY} L ${tx} ${ty}`
    }
    const offsetX = sx + (goesRight ? 1 : -1) * MIN_OFFSET
    return `M ${sx} ${sy} L ${offsetX} ${sy} L ${offsetX} ${entryY} L ${tx} ${entryY} L ${tx} ${ty}`
  }

  // Source exits vertically, target enters horizontally (Left/Right)
  const goesDown = sourcePosition === Position.Bottom
  const targetIsAhead = goesDown ? ty >= sy : ty <= sy

  // Horizontal entry stub: approach target from MIN_OFFSET left (Left) or right (Right)
  const entryX = tx + targetEntryDir * MIN_OFFSET

  if (targetIsAhead) {
    const exitY = sy + (goesDown ? 1 : -1) * MIN_OFFSET
    return `M ${sx} ${sy} L ${sx} ${exitY} L ${entryX} ${exitY} L ${entryX} ${ty} L ${tx} ${ty}`
  }
  const offsetY = sy + (goesDown ? 1 : -1) * MIN_OFFSET
  return `M ${sx} ${sy} L ${sx} ${offsetY} L ${entryX} ${offsetY} L ${entryX} ${ty} L ${tx} ${ty}`
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

// ============================================================================
// Obstacle-Aware Corridor Routing
// ============================================================================

type ObstacleBounds = { x: number; y: number; width: number; height: number }

/** Minimal node shape used for obstacle detection. */
type NodeLike = {
  id: string
  position: { x: number; y: number }
  measured?: { width?: number; height?: number }
  width?: number
  height?: number
}

/**
 * Find nodes whose bounding boxes overlap the rectangular area between
 * source and target handle positions. Used to detect obstacles for
 * corridor routing.
 */
const findObstaclesInPath = (
  nodes: ReadonlyArray<NodeLike>,
  sx: number, sy: number,
  tx: number, ty: number,
  excludeIds: ReadonlySet<string>,
  padding: number = 8,
): ObstacleBounds[] => {
  const minX = Math.min(sx, tx) - padding
  const maxX = Math.max(sx, tx) + padding
  const minY = Math.min(sy, ty) - padding
  const maxY = Math.max(sy, ty) + padding

  const obstacles: ObstacleBounds[] = []
  for (const node of nodes) {
    if (excludeIds.has(node.id)) continue
    const nx = node.position.x
    const ny = node.position.y
    const nw = node.measured?.width ?? node.width ?? 200
    const nh = node.measured?.height ?? node.height ?? 100

    // AABB overlap check
    if (nx + nw > minX && nx < maxX && ny + nh > minY && ny < maxY) {
      obstacles.push({ x: nx, y: ny, width: nw, height: nh })
    }
  }
  return obstacles
}

/**
 * Compute a 5-segment corridor path that routes around obstacles.
 *
 * The path exits the source vertically, moves horizontally to a corridor
 * outside the obstacle bounding box, travels vertically through the corridor,
 * then re-enters horizontally to reach the target.
 *
 * Corridor side heuristic: source left of target → LEFT corridor,
 * source right of target → RIGHT corridor.
 */
const computeCorridorPath = (
  sx: number, sy: number,
  tx: number, ty: number,
  obstacles: readonly ObstacleBounds[],
  margin: number = MIN_OFFSET,
): string => {
  if (obstacles.length === 0) {
    // No obstacles — straight line or simple bend
    return `M ${sx} ${sy} L ${tx} ${ty}`
  }

  // Compute obstacle bounding box
  let obsMinX = Infinity
  let obsMaxX = -Infinity
  for (const obs of obstacles) {
    obsMinX = Math.min(obsMinX, obs.x)
    obsMaxX = Math.max(obsMaxX, obs.x + obs.width)
  }

  // Choose corridor side: LEFT if source is left of or equal to target, RIGHT otherwise
  const useLeft = sx <= tx
  const corridorX = useLeft ? obsMinX - margin : obsMaxX + margin

  // Determine vertical direction (target above or below source)
  const goingUp = ty < sy // target above source in screen coords

  // Exit/enter offsets maintain handle direction
  const exitY = goingUp ? sy - margin : sy + margin
  const enterY = goingUp ? ty + margin : ty - margin

  return roundCorners(`M ${sx} ${sy} L ${sx} ${exitY} L ${corridorX} ${exitY} L ${corridorX} ${enterY} L ${tx} ${enterY} L ${tx} ${ty}`)
}

export { computeOrthogonalPath, computeOrthogonalLabel, assignParallelTracks, findObstaclesInPath, computeCorridorPath }
export type { ObstacleBounds, NodeLike }
