import { describe, it, expect } from 'vitest'
import { Position } from '@xyflow/react'
import { computeOrthogonalPath, computeOrthogonalLabel, assignParallelTracks, findObstaclesInPath, computeCorridorPath } from './orthogonalPath'
import type { NodeLike, ObstacleBounds } from './orthogonalPath'

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
    it('produces a direct vertical line when aligned (Bottom→Top)', () => {
      const path = computeOrthogonalPath(100, 0, 100, 200, Position.Bottom, Position.Top)
      expect(path).toBe('M 100 0 L 100 200')
    })

    it('routes horizontal near source for Bottom→Top offset', () => {
      // MIN_OFFSET = 24, so midY = sy + 24 = 0 + 24 = 24
      const path = computeOrthogonalPath(100, 0, 200, 200, Position.Bottom, Position.Top)
      const points = extractPoints(path)
      expect(points).toHaveLength(4)
      expect(isOrthogonal(path)).toBe(true)
      expect(points[0]).toEqual({ x: 100, y: 0 })
      expect(points[1]).toEqual({ x: 100, y: 24 })  // midY near source
      expect(points[2]).toEqual({ x: 200, y: 24 })
      expect(points[3]).toEqual({ x: 200, y: 200 })
    })

    it('produces a direct vertical line when aligned (Top→Bottom)', () => {
      const path = computeOrthogonalPath(100, 200, 100, 0, Position.Top, Position.Bottom)
      expect(path).toBe('M 100 200 L 100 0')
    })

    it('routes horizontal near target for Top→Bottom offset', () => {
      // MIN_OFFSET = 24, so midY = ty + 24 = 0 + 24 = 24
      const path = computeOrthogonalPath(100, 200, 200, 0, Position.Top, Position.Bottom)
      expect(isOrthogonal(path)).toBe(true)
      const points = extractPoints(path)
      expect(points).toHaveLength(4)
      expect(points[0]).toEqual({ x: 100, y: 200 })
      expect(points[1]).toEqual({ x: 100, y: 24 })  // midY near target
      expect(points[2]).toEqual({ x: 200, y: 24 })
      expect(points[3]).toEqual({ x: 200, y: 0 })
    })

    it('gap-aware: cross-tier edge avoids mid-tier collision', () => {
      // Simulates Designer A (tier 0, sy=-348) → Design Judge (tier 1, ty=-580+350=-230 for bottom)
      // midY should be ty + 24 = -556, in the gap above tier 0 docs
      const path = computeOrthogonalPath(150, -348, 300, -580, Position.Top, Position.Bottom)
      const points = extractPoints(path)
      expect(points).toHaveLength(4)
      expect(points[1]!.y).toBe(-580 + 24) // -556, in the tier gap
      expect(points[2]!.y).toBe(-580 + 24)
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

// ============================================================================
// findObstaclesInPath
// ============================================================================

describe('findObstaclesInPath', () => {
  const makeNode = (id: string, x: number, y: number, w: number, h: number): NodeLike => ({
    id,
    position: { x, y },
    measured: { width: w, height: h },
  })

  it('returns empty when no nodes intersect path', () => {
    const nodes = [makeNode('far', 500, 500, 100, 100)]
    const result = findObstaclesInPath(nodes, 0, 0, 100, 100, new Set())
    expect(result).toEqual([])
  })

  it('finds nodes overlapping the path area', () => {
    const nodes = [
      makeNode('blocker', 30, 30, 100, 100), // overlaps (0,0)→(100,100) area
      makeNode('far', 500, 500, 100, 100),    // outside
    ]
    const result = findObstaclesInPath(nodes, 0, 0, 100, 100, new Set())
    expect(result).toHaveLength(1)
    expect(result[0]).toEqual({ x: 30, y: 30, width: 100, height: 100 })
  })

  it('excludes source and target nodes', () => {
    const nodes = [
      makeNode('source', 0, 0, 100, 100),
      makeNode('target', 50, 50, 100, 100),
      makeNode('blocker', 30, 30, 50, 50),
    ]
    const result = findObstaclesInPath(nodes, 0, 0, 100, 100, new Set(['source', 'target']))
    expect(result).toHaveLength(1)
    expect(result[0]!.x).toBe(30)
  })

  it('respects padding parameter', () => {
    // Node at (105, 0) with width=50 is just outside the path area (0,0)→(100,100)
    // With padding=0, it should not be found. With padding=10, it should.
    const nodes = [makeNode('near', 105, 0, 50, 50)]
    const noPad = findObstaclesInPath(nodes, 0, 0, 100, 100, new Set(), 0)
    expect(noPad).toHaveLength(0)
    const withPad = findObstaclesInPath(nodes, 0, 0, 100, 100, new Set(), 10)
    expect(withPad).toHaveLength(1)
  })

  it('uses fallback dimensions when measured is missing', () => {
    const node: NodeLike = { id: 'bare', position: { x: 50, y: 50 } }
    const result = findObstaclesInPath([node], 0, 0, 100, 100, new Set())
    // Fallback: 200w × 100h — definitely overlaps
    expect(result).toHaveLength(1)
  })
})

// ============================================================================
// computeCorridorPath
// ============================================================================

describe('computeCorridorPath', () => {
  const obstacle: ObstacleBounds = { x: 100, y: -500, width: 200, height: 150 }

  it('routes LEFT when source is left of target', () => {
    // Source at x=150, target at x=200 → source left of target → LEFT corridor
    const path = computeCorridorPath(150, -300, 200, -700, [obstacle])
    const points = extractPoints(path)
    // Corridor should be at obstacle.x - margin = 100 - 24 = 76
    expect(points).toHaveLength(6) // 5 segments
    expect(isOrthogonal(path)).toBe(true)
    // All corridor points should have x = 76 (left of obstacle)
    const corridorX = points[2]!.x
    expect(corridorX).toBe(76)
    expect(points[3]!.x).toBe(76)
  })

  it('routes RIGHT when source is right of target', () => {
    // Source at x=250, target at x=200 → source right of target → RIGHT corridor
    const path = computeCorridorPath(250, -300, 200, -700, [obstacle])
    const points = extractPoints(path)
    expect(points).toHaveLength(6)
    expect(isOrthogonal(path)).toBe(true)
    // Corridor should be at obstacle.x + obstacle.width + margin = 100 + 200 + 24 = 324
    const corridorX = points[2]!.x
    expect(corridorX).toBe(324)
  })

  it('routes around when source and target are aligned vertically', () => {
    // Source at x=200, target at x=200, obstacle between them
    const path = computeCorridorPath(200, -300, 200, -700, [obstacle])
    const points = extractPoints(path)
    expect(points).toHaveLength(6)
    expect(isOrthogonal(path)).toBe(true)
    // Should route LEFT (sx <= tx) at corridorX = 76
    expect(points[2]!.x).toBe(76)
  })

  it('produces all orthogonal segments', () => {
    const path = computeCorridorPath(100, 0, 300, -500, [obstacle])
    expect(isOrthogonal(path)).toBe(true)
  })

  it('corridor is outside the obstacle bounding box', () => {
    const obs: ObstacleBounds[] = [
      { x: 50, y: -400, width: 300, height: 100 },
      { x: 100, y: -300, width: 200, height: 80 },
    ]
    const path = computeCorridorPath(150, -200, 200, -600, obs)
    const points = extractPoints(path)
    const corridorX = points[2]!.x
    // Left corridor: min(50, 100) - 24 = 26
    expect(corridorX).toBe(26)
    // Verify corridor is completely outside all obstacle X ranges
    for (const o of obs) {
      expect(corridorX).toBeLessThan(o.x)
    }
  })

  it('handles target below source (downward flow)', () => {
    // Source at y=0, target at y=500 (target below)
    const path = computeCorridorPath(150, 0, 250, 500, [{ x: 100, y: 100, width: 200, height: 100 }])
    const points = extractPoints(path)
    expect(points).toHaveLength(6)
    expect(isOrthogonal(path)).toBe(true)
    // Exit goes down: first segment should increase Y
    expect(points[1]!.y).toBeGreaterThan(points[0]!.y)
  })

  it('returns straight line for empty obstacles', () => {
    const path = computeCorridorPath(100, 0, 200, -500, [])
    expect(path).toBe('M 100 0 L 200 -500')
  })
})
