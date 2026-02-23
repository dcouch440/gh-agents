import { Geometry } from '@/utils/geometry'
import { lerpPoint } from '@/utils/spatial/lerpPoint'
import { parseWaypoints } from './parseWaypoints'

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
const roundCorners = (path: string, radius: number): string => {
  if (radius <= 0) return path

  const points = parseWaypoints(path)
  if (points.length < 3) return path

  const parts: string[] = [`M ${points[0]!.x} ${points[0]!.y}`]

  for (let i = 1; i < points.length - 1; i++) {
    const prev = points[i - 1]!
    const curr = points[i]!
    const next = points[i + 1]!

    const dBefore = Geometry.distanceBetweenPoints(prev, curr)
    const dAfter = Geometry.distanceBetweenPoints(curr, next)

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

    const approach = lerpPoint(curr, prev, r)
    const depart = lerpPoint(curr, next, r)

    parts.push(`L ${approach.x} ${approach.y}`)
    parts.push(`Q ${curr.x} ${curr.y} ${depart.x} ${depart.y}`)
  }

  const last = points[points.length - 1]!
  parts.push(`L ${last.x} ${last.y}`)

  return parts.join(' ')
}

export { roundCorners }
