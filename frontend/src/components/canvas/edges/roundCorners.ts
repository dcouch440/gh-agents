import { PIPE } from '../constants'

type Point = { x: number; y: number }

/** Parse an SVG path (M/L commands only) into waypoints. */
const parseWaypoints = (path: string): Point[] => {
  const points: Point[] = []
  const regex = /[ML]\s*([-\d.]+)\s+([-\d.]+)/g
  let match: RegExpExecArray | null = null
  while ((match = regex.exec(path)) !== null) {
    points.push({ x: parseFloat(match[1]!), y: parseFloat(match[2]!) })
  }
  return points
}

/** Distance between two points. */
const dist = (a: Point, b: Point): number =>
  Math.sqrt((b.x - a.x) ** 2 + (b.y - a.y) ** 2)

/** Point `distance` pixels from `a` toward `b`. */
const lerp = (a: Point, b: Point, distance: number): Point => {
  const d = dist(a, b)
  if (d === 0) return { ...a }
  const t = distance / d
  return { x: a.x + (b.x - a.x) * t, y: a.y + (b.y - a.y) * t }
}

/**
 * Replace sharp corners in an orthogonal SVG path with quadratic bezier curves.
 *
 * Takes a path string containing only M and L commands and returns a new path
 * where each intermediate corner is replaced with a smooth Q curve. The curve
 * starts `radius` px before the corner and ends `radius` px after, using the
 * original corner point as the bezier control point.
 *
 * The radius is clamped per-corner to not exceed half the length of either
 * adjacent segment, ensuring the curve never overshoots.
 */
const roundCorners = (path: string, radius: number = PIPE.CORNER_RADIUS): string => {
  if (radius <= 0) return path

  const points = parseWaypoints(path)
  if (points.length < 3) return path

  const parts: string[] = [`M ${points[0]!.x} ${points[0]!.y}`]

  for (let i = 1; i < points.length - 1; i++) {
    const prev = points[i - 1]!
    const curr = points[i]!
    const next = points[i + 1]!

    const dBefore = dist(prev, curr)
    const dAfter = dist(curr, next)

    // Clamp radius to half the shortest adjacent segment
    const r = Math.min(radius, dBefore / 2, dAfter / 2)

    if (r < 1) {
      // Segment too short to round — keep the sharp corner
      parts.push(`L ${curr.x} ${curr.y}`)
      continue
    }

    // Skip collinear points (same X or same Y for all three)
    const collinear =
      (prev.x === curr.x && curr.x === next.x) ||
      (prev.y === curr.y && curr.y === next.y)
    if (collinear) {
      parts.push(`L ${curr.x} ${curr.y}`)
      continue
    }

    const approach = lerp(curr, prev, r)
    const depart = lerp(curr, next, r)

    parts.push(`L ${approach.x} ${approach.y}`)
    parts.push(`Q ${curr.x} ${curr.y} ${depart.x} ${depart.y}`)
  }

  const last = points[points.length - 1]!
  parts.push(`L ${last.x} ${last.y}`)

  return parts.join(' ')
}

export { roundCorners, parseWaypoints }
export type { Point }
