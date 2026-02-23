import type { Point } from '@/utils/geometry'

/** Compute the midpoint between two points. */
const midpoint = (a: Point, b: Point): Point => ({
  x: (a.x + b.x) / 2,
  y: (a.y + b.y) / 2,
})

export { midpoint }
