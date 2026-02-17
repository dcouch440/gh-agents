import { describe, it, expect } from 'vitest'
import { Position } from '@xyflow/react'
import { computeOrthogonalPath, computeOrthogonalLabel, assignParallelTracks } from './orthogonalPath'

// ============================================================================
// Helpers
// ============================================================================

/** Extract all points from an SVG path string. */
const extractPoints = (path: string): Array<{ x: number; y: number }> => {
  const points: Array<{ x: number; y: number }> = []
  const regex = /[ML]\s*([-\d.]+)\s+([-\d.]+)/g
  let match: RegExpExecArray | null = null
  while ((match = regex.exec(path)) !== null) {
    points.push({ x: parseFloat(match[1]!), y: parseFloat(match[2]!) })
  }
  return points
}

/** Check that all segments in a path are axis-aligned (no diagonals). */
const isOrthogonal = (path: string): boolean => {
  const points = extractPoints(path)
  for (let i = 1; i < points.length; i++) {
    const prev = points[i - 1]!
    const curr = points[i]!
    if (prev.x !== curr.x && prev.y !== curr.y) return false
  }
  return true
}

// ============================================================================
// computeOrthogonalPath — Horizontal-to-Horizontal
// ============================================================================

describe('computeOrthogonalPath', () => {
  describe('horizontal-to-horizontal (spine edges)', () => {
    it('produces a direct horizontal line when aligned', () => {
      const path = computeOrthogonalPath(0, 100, 200, 100, Position.Right, Position.Left)
      expect(path).toBe('M 0 100 L 200 100')
      expect(isOrthogonal(path)).toBe(true)
    })

    it('produces a 3-segment path when vertically offset', () => {
      const path = computeOrthogonalPath(0, 100, 200, 200, Position.Right, Position.Left)
      const points = extractPoints(path)
      expect(points).toHaveLength(4) // M + 3 L commands
      expect(isOrthogonal(path)).toBe(true)
      // Starts at source, ends at target
      expect(points[0]).toEqual({ x: 0, y: 100 })
      expect(points[points.length - 1]).toEqual({ x: 200, y: 200 })
    })

    it('handles backward routing (target behind source)', () => {
      const path = computeOrthogonalPath(200, 100, 0, 200, Position.Right, Position.Left)
      expect(isOrthogonal(path)).toBe(true)
      const points = extractPoints(path)
      expect(points[0]).toEqual({ x: 200, y: 100 })
      expect(points[points.length - 1]).toEqual({ x: 0, y: 200 })
    })

    it('handles both-right ports', () => {
      const path = computeOrthogonalPath(0, 100, 200, 200, Position.Right, Position.Right)
      expect(isOrthogonal(path)).toBe(true)
    })
  })

  // ============================================================================
  // Vertical-to-Vertical (tower edges)
  // ============================================================================

  describe('vertical-to-vertical (tower edges)', () => {
    it('produces a direct vertical line when aligned', () => {
      const path = computeOrthogonalPath(100, 0, 100, 200, Position.Bottom, Position.Top)
      expect(path).toBe('M 100 0 L 100 200')
    })

    it('produces a 3-segment path when horizontally offset', () => {
      const path = computeOrthogonalPath(100, 0, 200, 200, Position.Bottom, Position.Top)
      const points = extractPoints(path)
      expect(points).toHaveLength(4)
      expect(isOrthogonal(path)).toBe(true)
    })

    it('handles top-to-bottom routing (upward)', () => {
      const path = computeOrthogonalPath(100, 200, 100, 0, Position.Top, Position.Bottom)
      expect(path).toBe('M 100 200 L 100 0')
    })

    it('handles top-to-bottom with horizontal offset', () => {
      const path = computeOrthogonalPath(100, 200, 200, 0, Position.Top, Position.Bottom)
      expect(isOrthogonal(path)).toBe(true)
      const points = extractPoints(path)
      expect(points[0]).toEqual({ x: 100, y: 200 })
      expect(points[points.length - 1]).toEqual({ x: 200, y: 0 })
    })

    it('handles awkward vertical routing', () => {
      const path = computeOrthogonalPath(100, 0, 200, 200, Position.Top, Position.Top)
      expect(isOrthogonal(path)).toBe(true)
    })
  })

  // ============================================================================
  // Mixed (L-shaped paths)
  // ============================================================================

  describe('mixed orientation (L-shaped)', () => {
    it('routes horizontal source to vertical target', () => {
      const path = computeOrthogonalPath(0, 100, 200, 0, Position.Right, Position.Top)
      expect(isOrthogonal(path)).toBe(true)
      const points = extractPoints(path)
      expect(points).toHaveLength(3) // M + corner + end
      expect(points[0]).toEqual({ x: 0, y: 100 })
      expect(points[points.length - 1]).toEqual({ x: 200, y: 0 })
    })

    it('routes vertical source to horizontal target', () => {
      const path = computeOrthogonalPath(100, 0, 200, 100, Position.Bottom, Position.Left)
      expect(isOrthogonal(path)).toBe(true)
      const points = extractPoints(path)
      expect(points).toHaveLength(3)
      expect(points[0]).toEqual({ x: 100, y: 0 })
      expect(points[points.length - 1]).toEqual({ x: 200, y: 100 })
    })
  })

  // ============================================================================
  // Degenerate cases
  // ============================================================================

  describe('degenerate cases', () => {
    it('handles same point', () => {
      const path = computeOrthogonalPath(100, 100, 100, 100, Position.Right, Position.Left)
      expect(path).toBe('M 100 100')
    })

    it('handles very close points', () => {
      const path = computeOrthogonalPath(100, 100, 101, 101, Position.Right, Position.Left)
      expect(isOrthogonal(path)).toBe(true)
    })
  })

  // ============================================================================
  // All paths are orthogonal
  // ============================================================================

  describe('orthogonality invariant', () => {
    const positions = [Position.Top, Position.Right, Position.Bottom, Position.Left]
    const coords = [
      { sx: 0, sy: 0, tx: 200, ty: 200 },
      { sx: 200, sy: 0, tx: 0, ty: 200 },
      { sx: 0, sy: 200, tx: 200, ty: 0 },
      { sx: 100, sy: 100, tx: 100, ty: 300 },
      { sx: 100, sy: 100, tx: 300, ty: 100 },
    ]

    for (const c of coords) {
      for (const sp of positions) {
        for (const tp of positions) {
          it(`is orthogonal for (${c.sx},${c.sy})→(${c.tx},${c.ty}) ${sp}→${tp}`, () => {
            const path = computeOrthogonalPath(c.sx, c.sy, c.tx, c.ty, sp, tp)
            expect(isOrthogonal(path)).toBe(true)
          })
        }
      }
    }
  })
})

// ============================================================================
// computeOrthogonalLabel
// ============================================================================

describe('computeOrthogonalLabel', () => {
  it('returns midpoint of source and target', () => {
    const { labelX, labelY } = computeOrthogonalLabel(0, 0, 200, 100)
    expect(labelX).toBe(100)
    expect(labelY).toBe(50)
  })

  it('handles same point', () => {
    const { labelX, labelY } = computeOrthogonalLabel(50, 50, 50, 50)
    expect(labelX).toBe(50)
    expect(labelY).toBe(50)
  })
})

// ============================================================================
// assignParallelTracks
// ============================================================================

describe('assignParallelTracks', () => {
  it('returns empty for zero edges', () => {
    expect(assignParallelTracks(0, 8)).toEqual([])
  })

  it('returns [0] for single edge', () => {
    expect(assignParallelTracks(1, 8)).toEqual([0])
  })

  it('centers two edges symmetrically', () => {
    const offsets = assignParallelTracks(2, 8)
    expect(offsets).toEqual([-4, 4])
  })

  it('centers three edges symmetrically', () => {
    const offsets = assignParallelTracks(3, 8)
    expect(offsets).toEqual([-8, 0, 8])
  })

  it('centers four edges', () => {
    const offsets = assignParallelTracks(4, 10)
    expect(offsets).toEqual([-15, -5, 5, 15])
  })

  it('respects spacing parameter', () => {
    const offsets = assignParallelTracks(3, 12)
    expect(offsets).toEqual([-12, 0, 12])
  })
})
