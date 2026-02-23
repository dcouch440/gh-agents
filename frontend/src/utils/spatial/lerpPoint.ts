import { Geometry } from '@/utils/geometry'
import type { Point } from '@/utils/geometry'

/**
 * Compute a point `distance` pixels from `a` toward `b`.
 *
 * Returns a copy of `a` if the two points are coincident (zero-length segment).
 * Uses absolute pixel distance, not a normalized 0–1 parameter.
 */
const lerpPoint = (a: Point, b: Point, distance: number): Point => {
  const d = Geometry.distanceBetweenPoints(a, b)
  if (d === 0) return { x: a.x, y: a.y }
  const t = distance / d
  return { x: a.x + (b.x - a.x) * t, y: a.y + (b.y - a.y) * t }
}

export { lerpPoint }
