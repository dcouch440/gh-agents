import type { Point } from '@/utils/geometry'

/**
 * Parse an SVG path string (M/L commands only) into an array of waypoints.
 *
 * Extracts all `M x y` and `L x y` commands from the path string.
 * Other SVG commands (C, Q, A, etc.) are ignored.
 */
const parseWaypoints = (path: string): Point[] => {
  const points: Point[] = []
  const regex = /[ML]\s*([-\d.]+)\s+([-\d.]+)/g
  let match: RegExpExecArray | null = null
  while ((match = regex.exec(path)) !== null) {
    points.push({ x: parseFloat(match[1]!), y: parseFloat(match[2]!) })
  }
  return points
}

export { parseWaypoints }
