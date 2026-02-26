// ============================================================================
// getStrokeOutline — Convert Raw Pen Points to Filled Outline via perfect-freehand
// ============================================================================

import getStroke from 'perfect-freehand'
import type { Point } from '@/utils/geometry'

type StrokeOptions = {
  readonly size?: number
  readonly thinning?: number
  readonly smoothing?: number
  readonly streamline?: number
  readonly simulatePressure?: boolean
}

const DEFAULTS: Required<StrokeOptions> = {
  size: 5,
  thinning: 0.5,
  smoothing: 0.5,
  streamline: 0.5,
  simulatePressure: true,
}

/**
 * Convert raw input points + pressures into outline points suitable for
 * filled polygon rendering. Uses `perfect-freehand` for variable-width
 * stroke generation with pressure sensitivity.
 *
 * Returns an empty array if fewer than 2 input points.
 */
const getStrokeOutline = (
  points: readonly Point[],
  pressures: readonly number[],
  options?: StrokeOptions,
): Point[] => {
  if (points.length < 2) return []

  const opts = { ...DEFAULTS, ...options }

  // Detect if any real pressure data exists (not all 0.5)
  let hasRealPressure = false
  for (let i = 0; i < pressures.length; i++) {
    if (pressures[i] !== 0.5) {
      hasRealPressure = true
      break
    }
  }

  // Build input array: [x, y, pressure]
  const input: number[][] = []
  for (let i = 0; i < points.length; i++) {
    const p = points[i]!
    input.push([p.x, p.y, pressures[i] ?? 0.5])
  }

  const outline = getStroke(input, {
    size: opts.size,
    thinning: opts.thinning,
    smoothing: opts.smoothing,
    streamline: opts.streamline,
    simulatePressure: !hasRealPressure && opts.simulatePressure,
  })

  // Convert number[][] to Point[]
  const result: Point[] = []
  for (let i = 0; i < outline.length; i++) {
    const pt = outline[i]!
    result.push({ x: pt[0], y: pt[1] })
  }

  return result
}

export { getStrokeOutline }
export type { StrokeOptions }
