import { describe, it, expect } from 'vitest'
import type { Rect } from '@/utils/geometry'
import type { PlacementIntent } from './types'
import { placeSpliceNode } from './splicePlacer'
import { buildOccupancyIndex } from './occupancyIndex'
import { PLACEMENT } from './constants'

const makeIntent = (overrides?: Partial<PlacementIntent>): PlacementIntent => ({
  stepId: 'new-node',
  width: 560,
  height: 500,
  strategy: 'splice',
  upstreamStepId: 'from',
  downstreamStepIds: ['to'],
  fanOutSourceId: null,
  spliceDownstreamId: 'to',
  ...overrides,
})

const makeOccNode = (id: string, x: number, y: number, w: number, h: number) => ({
  id,
  rect: { x, y, width: w, height: h } as Rect,
})

describe('splicePlacer', () => {
  describe('placeSpliceNode', () => {
    it('places in gap when gap is sufficient (no shift)', () => {
      // upstream at x=0, width=560. downstream at x=1500.
      // gap = 1500 - 560 = 940. needed = 96 + 560 + 96 = 752. 940 >= 752 → fits
      const upstream: Rect = { x: 0, y: 0, width: 560, height: 500 }
      const downstream: Rect = { x: 1500, y: 0, width: 560, height: 500 }

      const result = placeSpliceNode(makeIntent(), upstream, downstream, false, [])

      expect(result.shift).toBeNull()
      // x = snapToGrid(0 + 560 + 96, 24) = 648
      expect(result.placement.position.x).toBe(648)
    })

    it('aligns Y to edge midpoint when gap is sufficient', () => {
      const upstream: Rect = { x: 0, y: 0, width: 560, height: 500 }
      const downstream: Rect = { x: 1500, y: 0, width: 560, height: 500 }
      const intent = makeIntent({ height: 100 })

      const result = placeSpliceNode(intent, upstream, downstream, false, [])

      // Edge mid Y = (0 + 250 + 0 + 250) / 2 = 250. Placement Y = 250 - 50 = 200 → snap to 192 or 204
      expect(result.placement.position.y % PLACEMENT.GRID_SIZE).toBe(0)
    })

    it('shifts downstream right when gap insufficient and downstream is shiftable', () => {
      // upstream at x=0, width=560. downstream at x=660 (tight gap).
      // gap = 660 - 560 = 100. needed = 752. 100 < 752 → shift
      const upstream: Rect = { x: 0, y: 0, width: 560, height: 500 }
      const downstream: Rect = { x: 660, y: 0, width: 560, height: 500 }

      const result = placeSpliceNode(makeIntent(), upstream, downstream, true, [])

      expect(result.shift).not.toBeNull()
      expect(result.shift!.stepId).toBe('to')
      expect(result.shift!.dx).toBeGreaterThan(0)
      expect(result.shift!.dx % PLACEMENT.GRID_SIZE).toBe(0) // grid-aligned
      expect(result.shift!.dy).toBe(0)
    })

    it('returns correct shift amount', () => {
      const upstream: Rect = { x: 0, y: 0, width: 560, height: 500 }
      const downstream: Rect = { x: 660, y: 0, width: 560, height: 500 }

      const result = placeSpliceNode(makeIntent(), upstream, downstream, true, [])

      // gap = 100. needed = 752. raw shift = 652. snap(652, 24) = 648
      expect(result.shift!.dx).toBe(648)
    })

    it('places above edge line when gap insufficient and downstream not shiftable', () => {
      const upstream: Rect = { x: 0, y: 200, width: 560, height: 500 }
      const downstream: Rect = { x: 660, y: 200, width: 560, height: 500 }
      const intent = makeIntent({ height: 100 })

      const result = placeSpliceNode(intent, upstream, downstream, false, [])

      // Should place above the nodes (y < upstream.y)
      expect(result.shift).toBeNull()
      expect(result.placement.position.y).toBeLessThan(200)
    })

    it('places below edge line when above is occupied and downstream not shiftable', () => {
      const upstream: Rect = { x: 0, y: 200, width: 560, height: 100 }
      const downstream: Rect = { x: 660, y: 200, width: 560, height: 100 }
      const intent = makeIntent({ height: 50 })

      // Block the position above
      const aboveY = 200 - 50 - PLACEMENT.V_GAP // where it would try above
      const occupancy = buildOccupancyIndex([
        makeOccNode('blocker-above', 648, aboveY - 50, 700, 200),
      ])

      const result = placeSpliceNode(intent, upstream, downstream, false, occupancy)

      expect(result.shift).toBeNull()
      // Should place below the nodes (y >= max bottom)
      const maxBottom = Math.max(200 + 100, 200 + 100)
      expect(result.placement.position.y).toBeGreaterThanOrEqual(maxBottom)
    })

    it('returns null shift when gap is sufficient', () => {
      const upstream: Rect = { x: 0, y: 0, width: 100, height: 100 }
      const downstream: Rect = { x: 2000, y: 0, width: 100, height: 100 }

      const result = placeSpliceNode(makeIntent(), upstream, downstream, true, [])
      expect(result.shift).toBeNull()
    })

    it('returns null shift when downstream is not shiftable', () => {
      const upstream: Rect = { x: 0, y: 0, width: 560, height: 500 }
      const downstream: Rect = { x: 660, y: 0, width: 560, height: 500 }

      const result = placeSpliceNode(makeIntent(), upstream, downstream, false, [])
      expect(result.shift).toBeNull()
    })

    it('all positions are grid-aligned', () => {
      const upstream: Rect = { x: 0, y: 0, width: 560, height: 500 }
      const downstream: Rect = { x: 1500, y: 0, width: 560, height: 500 }

      const result = placeSpliceNode(makeIntent(), upstream, downstream, false, [])

      expect(result.placement.position.x % PLACEMENT.GRID_SIZE).toBe(0)
      expect(result.placement.position.y % PLACEMENT.GRID_SIZE).toBe(0)
    })

    it('returns correct stepId', () => {
      const upstream: Rect = { x: 0, y: 0, width: 100, height: 100 }
      const downstream: Rect = { x: 2000, y: 0, width: 100, height: 100 }

      const result = placeSpliceNode(makeIntent({ stepId: 'spliced' }), upstream, downstream, false, [])
      expect(result.placement.stepId).toBe('spliced')
    })
  })
})
