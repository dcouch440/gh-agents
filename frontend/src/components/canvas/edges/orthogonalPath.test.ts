import { describe, it, expect } from 'vitest'
import { Position } from '@xyflow/react'
import { computeOrthogonalPath, computeOrthogonalLabel, assignParallelTracks, findObstaclesInPath, computeCorridorPath } from './orthogonalPath'
import type { NodeLike, ObstacleBounds } from './orthogonalPath'

// ============================================================================
// Helpers
// ============================================================================

/** Extract all endpoint coordinates (M, L, and Q endpoints) from path. */
const extractPoints = (path: string): Array<{ x: number; y: number }> => {
  const points: Array<{ x: number; y: number }> = []
  const regex = /([MLQ])\s*([-\d.]+)\s+([-\d.]+)(?:\s+([-\d.]+)\s+([-\d.]+))?/g
  let match: RegExpExecArray | null = null
  while ((match = regex.exec(path)) !== null) {
    const cmd = match[1]!
    if (cmd === 'Q' && match[4] !== undefined && match[5] !== undefined) {
      points.push({ x: parseFloat(match[4]), y: parseFloat(match[5]) })
    } else {
      points.push({ x: parseFloat(match[2]!), y: parseFloat(match[3]!) })
    }
  }
  return points
}

/** Extract Q command control points — the original corner positions before rounding. */
const extractCorners = (path: string): Array<{ x: number; y: number }> => {
  const corners: Array<{ x: number; y: number }> = []
  const regex = /Q\s*([-\d.]+)\s+([-\d.]+)\s+[-\d.]+\s+[-\d.]+/g
  let match: RegExpExecArray | null = null
  while ((match = regex.exec(path)) !== null) {
    corners.push({ x: parseFloat(match[1]!), y: parseFloat(match[2]!) })
  }
  return corners
}

/** Check that all straight segments are axis-aligned. Q curves bridge axis transitions. */
const isOrthogonal = (path: string): boolean => {
  const segments: Array<{ x: number; y: number; isQ: boolean }> = []
  const regex = /([MLQ])\s*([-\d.]+)\s+([-\d.]+)(?:\s+([-\d.]+)\s+([-\d.]+))?/g
  let match: RegExpExecArray | null = null
  while ((match = regex.exec(path)) !== null) {
    const cmd = match[1]!
    if (cmd === 'Q' && match[4] !== undefined && match[5] !== undefined) {
      segments.push({ x: parseFloat(match[4]), y: parseFloat(match[5]), isQ: true })
    } else {
      segments.push({ x: parseFloat(match[2]!), y: parseFloat(match[3]!), isQ: false })
    }
  }
  for (let i = 1; i < segments.length; i++) {
    const prev = segments[i - 1]!
    const curr = segments[i]!
    // Q curves handle axis transitions — skip alignment check
    if (curr.isQ) continue
    if (prev.x !== curr.x && prev.y !== curr.y) return false
  }
  return true
}

/** Count the number of rounded corners (Q commands) in a path. */
const countCorners = (path: string): number =>
  (path.match(/Q\s/g) ?? []).length

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

    it('produces a 3-segment path with rounded corners when vertically offset', () => {
      const path = computeOrthogonalPath(0, 100, 200, 200, Position.Right, Position.Left)
      expect(isOrthogonal(path)).toBe(true)
      expect(countCorners(path)).toBe(2)
      const points = extractPoints(path)
      expect(points[0]).toEqual({ x: 0, y: 100 })
      expect(points[points.length - 1]).toEqual({ x: 200, y: 200 })
      // Corners should be at the midX clamped to MIN_OFFSET from both handles
      const corners = extractCorners(path)
      expect(corners[0]!.x).toBe(100) // midX = (0+200)/2
      expect(corners[1]!.x).toBe(100)
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

    it('clamps midX to guarantee MIN_OFFSET from both handles', () => {
      // Close nodes: gap of 60px. midX would be 30, but MIN_OFFSET=24 from source clamps to 24
      const path = computeOrthogonalPath(0, 0, 60, 100, Position.Right, Position.Left)
      const corners = extractCorners(path)
      // midX = max(0+24, min(60-24, 30)) = max(24, min(36, 30)) = max(24, 30) = 30
      expect(corners[0]!.x).toBe(30)
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
      const path = computeOrthogonalPath(100, 0, 200, 200, Position.Bottom, Position.Top)
      expect(isOrthogonal(path)).toBe(true)
      expect(countCorners(path)).toBe(2)
      const points = extractPoints(path)
      expect(points[0]).toEqual({ x: 100, y: 0 })
      expect(points[points.length - 1]).toEqual({ x: 200, y: 200 })
      // Corner Y should be at sy + MIN_OFFSET = 24
      const corners = extractCorners(path)
      expect(corners[0]!.y).toBe(24)
      expect(corners[1]!.y).toBe(24)
    })

    it('produces a direct vertical line when aligned (Top→Bottom)', () => {
      const path = computeOrthogonalPath(100, 200, 100, 0, Position.Top, Position.Bottom)
      expect(path).toBe('M 100 200 L 100 0')
    })

    it('routes horizontal near target for Top→Bottom offset', () => {
      const path = computeOrthogonalPath(100, 200, 200, 0, Position.Top, Position.Bottom)
      expect(isOrthogonal(path)).toBe(true)
      expect(countCorners(path)).toBe(2)
      const points = extractPoints(path)
      expect(points[0]).toEqual({ x: 100, y: 200 })
      expect(points[points.length - 1]).toEqual({ x: 200, y: 0 })
      // Corner Y should be at ty + MIN_OFFSET = 0 + 24 = 24
      const corners = extractCorners(path)
      expect(corners[0]!.y).toBe(24)
      expect(corners[1]!.y).toBe(24)
    })

    it('gap-aware: cross-tier edge avoids mid-tier collision', () => {
      const path = computeOrthogonalPath(150, -348, 300, -580, Position.Top, Position.Bottom)
      expect(isOrthogonal(path)).toBe(true)
      // midY should be ty + 24 = -556
      const corners = extractCorners(path)
      expect(corners[0]!.y).toBe(-556)
      expect(corners[1]!.y).toBe(-556)
    })

    it('snaps to vertical line when X offset is within tolerance', () => {
      const path = computeOrthogonalPath(100, 0, 105, 200, Position.Bottom, Position.Top)
      expect(path).toBe('M 100 0 L 100 200')
    })

    it('does not snap when X offset exceeds tolerance', () => {
      const path = computeOrthogonalPath(100, 0, 110, 200, Position.Bottom, Position.Top)
      expect(countCorners(path)).toBeGreaterThan(0)
    })

    it('handles awkward vertical routing', () => {
      const path = computeOrthogonalPath(100, 0, 200, 200, Position.Top, Position.Top)
      expect(isOrthogonal(path)).toBe(true)
    })
  })

  // ============================================================================
  // Mixed (S-shaped paths with guaranteed stubs)
  // ============================================================================

  describe('mixed orientation (4-segment with entry stubs)', () => {
    it('routes horizontal source to vertical target (target ahead) with offset stubs', () => {
      // Right → Top: exits right, enters from above
      const path = computeOrthogonalPath(0, 100, 200, 0, Position.Right, Position.Top)
      expect(isOrthogonal(path)).toBe(true)
      expect(countCorners(path)).toBe(3) // 4-segment has 3 bends
      const points = extractPoints(path)
      expect(points[0]).toEqual({ x: 0, y: 100 })
      expect(points[points.length - 1]).toEqual({ x: 200, y: 0 })
      // Exit stub at sx + MIN_OFFSET = 24, entry stub at ty - MIN_OFFSET = -24
      const corners = extractCorners(path)
      expect(corners[0]!.x).toBe(24) // exit X
      expect(corners[2]!.y).toBe(-24) // entry Y (Top → approach from above)
    })

    it('routes vertical source to horizontal target (target ahead) with offset stubs', () => {
      // Bottom → Left: exits down, enters from left
      const path = computeOrthogonalPath(100, 0, 200, 100, Position.Bottom, Position.Left)
      expect(isOrthogonal(path)).toBe(true)
      expect(countCorners(path)).toBe(3)
      const points = extractPoints(path)
      expect(points[0]).toEqual({ x: 100, y: 0 })
      expect(points[points.length - 1]).toEqual({ x: 200, y: 100 })
      // Exit stub at sy + MIN_OFFSET = 24, entry stub at tx - MIN_OFFSET = 176
      const corners = extractCorners(path)
      expect(corners[0]!.y).toBe(24) // exit Y
      expect(corners[2]!.x).toBe(176) // entry X (Left → approach from left)
    })

    it('routes Bottom source to Left target when target is above (subway)', () => {
      // Bottom → Left: target is above, so "behind" — still gets entry stub
      const path = computeOrthogonalPath(280, 500, 632, 165, Position.Bottom, Position.Left)
      expect(isOrthogonal(path)).toBe(true)
      expect(countCorners(path)).toBe(3)
      const points = extractPoints(path)
      expect(points[0]).toEqual({ x: 280, y: 500 })
      expect(points[points.length - 1]).toEqual({ x: 632, y: 165 })
      // Exit at sy + MIN_OFFSET = 524, entry at tx - MIN_OFFSET = 608
      const corners = extractCorners(path)
      expect(corners[0]!.y).toBe(524)
      expect(corners[2]!.x).toBe(608) // horizontal entry stub
    })

    it('routes Right source to Top target when target is behind (detour)', () => {
      const path = computeOrthogonalPath(200, 100, 50, 300, Position.Right, Position.Top)
      expect(isOrthogonal(path)).toBe(true)
      expect(countCorners(path)).toBe(3)
      const points = extractPoints(path)
      expect(points[0]).toEqual({ x: 200, y: 100 })
      expect(points[points.length - 1]).toEqual({ x: 50, y: 300 })
      // Exit at sx + MIN_OFFSET = 224, entry at ty - MIN_OFFSET = 276
      const corners = extractCorners(path)
      expect(corners[0]!.x).toBe(224)
      expect(corners[2]!.y).toBe(276) // vertical entry stub (Top → from above)
    })

    it('routes Top source to Right target when target is below (subway)', () => {
      const path = computeOrthogonalPath(100, 0, 300, 200, Position.Top, Position.Right)
      expect(isOrthogonal(path)).toBe(true)
      expect(countCorners(path)).toBe(3)
      const points = extractPoints(path)
      expect(points[0]).toEqual({ x: 100, y: 0 })
      expect(points[points.length - 1]).toEqual({ x: 300, y: 200 })
      // Exit at sy - MIN_OFFSET = -24, entry at tx + MIN_OFFSET = 324 (Right → from right)
      const corners = extractCorners(path)
      expect(corners[0]!.y).toBe(-24)
      expect(corners[2]!.x).toBe(324)
    })

    it('routes Bottom to Left with horizontal entry not flush with target', () => {
      // Simulates Input → Protocol: Bottom handle exits down, Left handle enters horizontally
      const path = computeOrthogonalPath(150, 400, 500, 200, Position.Bottom, Position.Left)
      expect(isOrthogonal(path)).toBe(true)
      const points = extractPoints(path)
      expect(points[0]).toEqual({ x: 150, y: 400 })
      expect(points[points.length - 1]).toEqual({ x: 500, y: 200 })
      // Entry X should be tx - MIN_OFFSET = 476, NOT at tx=500 (flush with target)
      const corners = extractCorners(path)
      const entryCorner = corners[corners.length - 1]!
      expect(entryCorner.x).toBe(476) // 24px gap from target's left edge
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
      makeNode('blocker', 30, 30, 100, 100),
      makeNode('far', 500, 500, 100, 100),
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
    const nodes = [makeNode('near', 105, 0, 50, 50)]
    const noPad = findObstaclesInPath(nodes, 0, 0, 100, 100, new Set(), 0)
    expect(noPad).toHaveLength(0)
    const withPad = findObstaclesInPath(nodes, 0, 0, 100, 100, new Set(), 10)
    expect(withPad).toHaveLength(1)
  })

  it('uses fallback dimensions when measured is missing', () => {
    const node: NodeLike = { id: 'bare', position: { x: 50, y: 50 } }
    const result = findObstaclesInPath([node], 0, 0, 100, 100, new Set())
    expect(result).toHaveLength(1)
  })
})

// ============================================================================
// computeCorridorPath
// ============================================================================

describe('computeCorridorPath', () => {
  const obstacle: ObstacleBounds = { x: 100, y: -500, width: 200, height: 150 }

  it('routes LEFT when source is left of target', () => {
    const path = computeCorridorPath(150, -300, 200, -700, [obstacle])
    expect(isOrthogonal(path)).toBe(true)
    expect(countCorners(path)).toBe(4) // 5-segment corridor has 4 corners
    // Corridor should be at obstacle.x - margin = 76
    const corners = extractCorners(path)
    const corridorCorners = corners.filter((c) => c.x === 76)
    expect(corridorCorners.length).toBeGreaterThanOrEqual(1)
  })

  it('routes RIGHT when source is right of target', () => {
    const path = computeCorridorPath(250, -300, 200, -700, [obstacle])
    expect(isOrthogonal(path)).toBe(true)
    expect(countCorners(path)).toBe(4)
    // Corridor should be at obstacle.x + obstacle.width + margin = 324
    const corners = extractCorners(path)
    const corridorCorners = corners.filter((c) => c.x === 324)
    expect(corridorCorners.length).toBeGreaterThanOrEqual(1)
  })

  it('routes around when source and target are aligned vertically', () => {
    const path = computeCorridorPath(200, -300, 200, -700, [obstacle])
    expect(isOrthogonal(path)).toBe(true)
    // Should route LEFT (sx <= tx) at corridorX = 76
    const corners = extractCorners(path)
    const corridorCorners = corners.filter((c) => c.x === 76)
    expect(corridorCorners.length).toBeGreaterThanOrEqual(1)
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
    // Left corridor: min(50, 100) - 24 = 26
    const corners = extractCorners(path)
    const corridorCorners = corners.filter((c) => c.x === 26)
    expect(corridorCorners.length).toBeGreaterThanOrEqual(1)
    // Verify corridor is completely outside all obstacle X ranges
    for (const c of corridorCorners) {
      for (const o of obs) {
        expect(c.x).toBeLessThan(o.x)
      }
    }
  })

  it('handles target below source (downward flow)', () => {
    const path = computeCorridorPath(150, 0, 250, 500, [{ x: 100, y: 100, width: 200, height: 100 }])
    expect(isOrthogonal(path)).toBe(true)
    expect(countCorners(path)).toBe(4)
    const points = extractPoints(path)
    expect(points[0]).toEqual({ x: 150, y: 0 })
    expect(points[points.length - 1]).toEqual({ x: 250, y: 500 })
  })

  it('returns straight line for empty obstacles', () => {
    const path = computeCorridorPath(100, 0, 200, -500, [])
    expect(path).toBe('M 100 0 L 200 -500')
  })
})
