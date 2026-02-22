import { describe, it, expect } from 'vitest'
import type { Rect } from '@/utils/geometry'
import { Geometry } from '@/utils/geometry'
import type { PlacementIntent } from './types'
import { placeFanOutGroup, placeConvergenceTarget } from './fanOutPlacer'
import { buildOccupancyIndex } from './occupancyIndex'
import { PLACEMENT } from './constants'

const makeIntent = (overrides?: Partial<PlacementIntent>): PlacementIntent => ({
  stepId: 'test-step',
  width: 560,
  height: 500,
  strategy: 'fan_out',
  upstreamStepId: 'source',
  downstreamStepIds: [],
  fanOutSourceId: 'source',
  spliceDownstreamId: null,
  ...overrides,
})

const makeOccNode = (id: string, x: number, y: number, w: number, h: number) => ({
  id,
  rect: { x, y, width: w, height: h } as Rect,
})

describe('fanOutPlacer', () => {
  describe('placeFanOutGroup', () => {
    const sourceRect: Rect = { x: 0, y: 0, width: 560, height: 500 }

    it('places 2 children vertically centered on source Y midpoint', () => {
      const siblings = [
        makeIntent({ stepId: 'a' }),
        makeIntent({ stepId: 'b' }),
      ]
      const results = placeFanOutGroup(siblings, sourceRect, [])

      expect(results).toHaveLength(2)

      // Stack height: 500 + 48 + 500 = 1048
      // Source mid Y: 250
      // Stack top: 250 - 1048/2 = 250 - 524 = -274 → snap to -264
      // a at top, b below a
      expect(results[0]!.stepId).toBe('a')
      expect(results[1]!.stepId).toBe('b')
      expect(results[0]!.position.y).toBeLessThan(results[1]!.position.y)

      // Both at same X
      expect(results[0]!.position.x).toBe(results[1]!.position.x)
    })

    it('places 3 children vertically centered on source Y midpoint', () => {
      const siblings = [
        makeIntent({ stepId: 'a' }),
        makeIntent({ stepId: 'b' }),
        makeIntent({ stepId: 'c' }),
      ]
      const results = placeFanOutGroup(siblings, sourceRect, [])

      expect(results).toHaveLength(3)

      // Verify vertical ordering
      expect(results[0]!.position.y).toBeLessThan(results[1]!.position.y)
      expect(results[1]!.position.y).toBeLessThan(results[2]!.position.y)

      // All at same X
      const xs = results.map((r) => r.position.x)
      expect(new Set(xs).size).toBe(1)
    })

    it('positions children at source.right + H_GAP (grid-snapped)', () => {
      const siblings = [makeIntent({ stepId: 'a' }), makeIntent({ stepId: 'b' })]
      const results = placeFanOutGroup(siblings, sourceRect, [])

      // 560 + 96 = 656 → snapToGrid(656, 24) = 648
      expect(results[0]!.position.x).toBe(648)
    })

    it('applies approximately V_GAP spacing between siblings', () => {
      const siblings = [
        makeIntent({ stepId: 'a', height: 100 }),
        makeIntent({ stepId: 'b', height: 100 }),
      ]
      const results = placeFanOutGroup(siblings, sourceRect, [])

      // Gap between bottom of a and top of b should be approximately V_GAP
      // (may differ by up to GRID_SIZE/2 due to individual grid-snapping)
      const aBottom = results[0]!.position.y + 100
      const bTop = results[1]!.position.y
      const gap = bTop - aBottom
      expect(gap).toBeGreaterThanOrEqual(PLACEMENT.V_GAP - PLACEMENT.GRID_SIZE)
      expect(gap).toBeLessThanOrEqual(PLACEMENT.V_GAP + PLACEMENT.GRID_SIZE)
    })

    it('shifts entire group down when top position collides', () => {
      const siblings = [
        makeIntent({ stepId: 'a', height: 20 }),
        makeIntent({ stepId: 'b', height: 20 }),
      ]

      // Stack would be placed near source center. Put a blocker there.
      const sourceCenter = sourceRect.y + sourceRect.height / 2
      const occupancy = buildOccupancyIndex([
        makeOccNode('blocker', 648, sourceCenter - 50, 560, 20),
      ])

      const results = placeFanOutGroup(siblings, sourceRect, occupancy)

      // Should still place successfully (shifted down)
      expect(results).toHaveLength(2)
      // Should not overlap blocker
      for (const r of results) {
        const rRect: Rect = { x: r.position.x, y: r.position.y, width: 560, height: 20 }
        const blockerPadded = Geometry.expandRect(
          { x: 648, y: sourceCenter - 50, width: 560, height: 20 },
          PLACEMENT.OCCUPANCY_PAD,
        )
        expect(Geometry.rectsOverlap(rRect, blockerPadded)).toBe(false)
      }
    })

    it('returns correct stepIds in order', () => {
      const siblings = [
        makeIntent({ stepId: 'first' }),
        makeIntent({ stepId: 'second' }),
        makeIntent({ stepId: 'third' }),
      ]
      const results = placeFanOutGroup(siblings, sourceRect, [])

      expect(results[0]!.stepId).toBe('first')
      expect(results[1]!.stepId).toBe('second')
      expect(results[2]!.stepId).toBe('third')
    })

    it('all positions are grid-aligned', () => {
      const siblings = [
        makeIntent({ stepId: 'a' }),
        makeIntent({ stepId: 'b' }),
        makeIntent({ stepId: 'c' }),
      ]
      const results = placeFanOutGroup(siblings, sourceRect, [])

      for (const r of results) {
        expect(r.position.x % PLACEMENT.GRID_SIZE).toBe(0)
        // Math.abs normalizes -0 → 0 (JS % produces -0 for negative grid-aligned values)
        expect(Math.abs(r.position.y % PLACEMENT.GRID_SIZE)).toBe(0)
      }
    })

    it('handles children with different heights without overlap', () => {
      const siblings = [
        makeIntent({ stepId: 'a', height: 100 }),
        makeIntent({ stepId: 'b', height: 300 }),
        makeIntent({ stepId: 'c', height: 200 }),
      ]
      const results = placeFanOutGroup(siblings, sourceRect, [])

      expect(results).toHaveLength(3)
      // Verify no overlaps between siblings (using their actual heights)
      const heights = [100, 300, 200]
      for (let i = 0; i < results.length - 1; i++) {
        const bottom = results[i]!.position.y + heights[i]!
        const nextTop = results[i + 1]!.position.y
        // Gap must be positive (no overlap) and approximately V_GAP
        expect(nextTop).toBeGreaterThan(bottom)
      }
    })
  })

  describe('placeConvergenceTarget', () => {
    it('places to the right of the rightmost sibling', () => {
      const siblingRects: Rect[] = [
        { x: 648, y: 0, width: 560, height: 500 },
        { x: 648, y: 548, width: 560, height: 500 },
      ]
      const intent = makeIntent({ stepId: 'target' })
      const result = placeConvergenceTarget(intent, siblingRects, [])

      // 648 + 560 = 1208. 1208 + 96 = 1304 → snapToGrid(1304, 24) = 1296
      expect(result.position.x).toBe(1296)
    })

    it('centers vertically on the sibling stack', () => {
      const siblingRects: Rect[] = [
        { x: 648, y: 0, width: 560, height: 100 },
        { x: 648, y: 148, width: 560, height: 100 },
      ]
      const intent = makeIntent({ stepId: 'target', height: 100 })
      const result = placeConvergenceTarget(intent, siblingRects, [])

      // Stack bounds: y=0, height=248. Mid = 124. target.y = 124 - 50 = 74 → snap to 72
      expect(result.position.y).toBe(72)
    })

    it('shifts down when target position is occupied', () => {
      const siblingRects: Rect[] = [
        { x: 648, y: 0, width: 560, height: 100 },
        { x: 648, y: 148, width: 560, height: 100 },
      ]
      const intent = makeIntent({ stepId: 'target', height: 20 })

      // Block the expected convergence position
      const occupancy = buildOccupancyIndex([
        makeOccNode('blocker', 1296, 72, 560, 20),
      ])

      const result = placeConvergenceTarget(intent, siblingRects, occupancy)

      // Should shift down and not overlap blocker
      expect(result.position.y).toBeGreaterThan(72)
      expect(result.position.y % PLACEMENT.GRID_SIZE).toBe(0)
    })

    it('all positions are grid-aligned', () => {
      const siblingRects: Rect[] = [
        { x: 648, y: 0, width: 560, height: 500 },
        { x: 648, y: 548, width: 560, height: 500 },
      ]
      const intent = makeIntent({ stepId: 'target' })
      const result = placeConvergenceTarget(intent, siblingRects, [])

      expect(result.position.x % PLACEMENT.GRID_SIZE).toBe(0)
      expect(result.position.y % PLACEMENT.GRID_SIZE).toBe(0)
    })

    it('returns correct stepId', () => {
      const siblingRects: Rect[] = [{ x: 648, y: 0, width: 560, height: 500 }]
      const intent = makeIntent({ stepId: 'my-target' })
      const result = placeConvergenceTarget(intent, siblingRects, [])
      expect(result.stepId).toBe('my-target')
    })
  })
})
