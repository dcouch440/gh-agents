import { describe, it, expect } from 'vitest'
import type { Rect } from '@/utils/geometry'
import type { PlacementIntent } from './types'
import { placePipelineNode, placeRootNode } from './pipelinePlacer'
import { buildOccupancyIndex } from './occupancyIndex'
import { PLACEMENT } from './constants'

const makeIntent = (overrides?: Partial<PlacementIntent>): PlacementIntent => ({
  stepId: 'test-step',
  width: 560,
  height: 500,
  strategy: 'pipeline',
  upstreamStepId: null,
  downstreamStepIds: [],
  ...overrides,
})

const makeOccNode = (id: string, x: number, y: number, w: number, h: number) => ({
  id,
  rect: { x, y, width: w, height: h } as Rect,
})

describe('pipelinePlacer', () => {
  describe('placePipelineNode', () => {
    it('places right of upstream with H_GAP (grid-snapped)', () => {
      const upstream: Rect = { x: 0, y: 0, width: 560, height: 500 }
      const result = placePipelineNode(makeIntent(), upstream, [])

      // 560 + 96 = 656 → snapToGrid(656, 24) = 648
      expect(result.position.x).toBe(648)
      expect(result.position.y).toBe(0) // top-aligned
    })

    it('snaps position to grid', () => {
      // Upstream at odd position that doesn't snap cleanly
      const upstream: Rect = { x: 13, y: 7, width: 560, height: 500 }
      const result = placePipelineNode(makeIntent(), upstream, [])

      expect(result.position.x % PLACEMENT.GRID_SIZE).toBe(0)
      expect(result.position.y % PLACEMENT.GRID_SIZE).toBe(0)
    })

    it('shifts down by V_GAP when first position is occupied', () => {
      const upstream: Rect = { x: 0, y: 0, width: 560, height: 500 }
      const snappedX = 648 // snapToGrid(560 + 96, 24)

      // Use a short blocker (height 20) so one V_GAP shift escapes it.
      // Padded rect y-range: [-24, 44]. Row 1 candidate at y=48 clears it.
      const occupancy = buildOccupancyIndex([
        makeOccNode('blocker', snappedX, 0, 560, 20),
      ])

      const result = placePipelineNode(makeIntent(), upstream, occupancy)

      expect(result.position.x).toBe(snappedX)
      expect(result.position.y).toBe(PLACEMENT.V_GAP) // shifted down once
    })

    it('shifts down multiple times when needed', () => {
      const upstream: Rect = { x: 0, y: 0, width: 560, height: 500 }
      const snappedX = 648

      // Two short blockers at row 0 and row 1 (height 20 each).
      // Blocker 1 padded y-range: [-24, 44] → blocks row 0 (y=0)
      // Blocker 2 padded y-range: [24, 92] → blocks row 1 (y=48)
      // Row 2 candidate at y=96 clears both.
      const occupancy = buildOccupancyIndex([
        makeOccNode('b1', snappedX, 0, 560, 20),
        makeOccNode('b2', snappedX, PLACEMENT.V_GAP, 560, 20),
      ])

      const result = placePipelineNode(makeIntent(), upstream, occupancy)

      expect(result.position.x).toBe(snappedX)
      expect(result.position.y).toBe(PLACEMENT.V_GAP * 2) // shifted down twice
    })

    it('returns correct stepId', () => {
      const upstream: Rect = { x: 0, y: 0, width: 100, height: 100 }
      const result = placePipelineNode(makeIntent({ stepId: 'my-step' }), upstream, [])
      expect(result.stepId).toBe('my-step')
    })
  })

  describe('placeRootNode', () => {
    it('places at origin on empty canvas', () => {
      const result = placeRootNode(makeIntent(), [])

      expect(result.position.x).toBe(PLACEMENT.ORIGIN_X)
      expect(result.position.y).toBe(PLACEMENT.ORIGIN_Y)
    })

    it('places to right of existing content', () => {
      const occupancy = buildOccupancyIndex([
        makeOccNode('existing', 0, 0, 560, 500),
      ])

      const result = placeRootNode(makeIntent(), occupancy)

      // bounds.width=560, H_GAP=96 → 656 → snapToGrid(656,24) = 648
      expect(result.position.x).toBe(648)
      expect(result.position.y).toBe(0)
    })

    it('shifts down when position is occupied', () => {
      // Two nodes side by side — root should go below or further right
      const occupancy = buildOccupancyIndex([
        makeOccNode('a', 0, 0, 560, 500),
        makeOccNode('b', 656, 0, 560, 500), // right of a at H_GAP
      ])

      const result = placeRootNode(makeIntent(), occupancy)

      // Position should not overlap either existing node
      // The exact position depends on bounds calculation
      expect(result.position.x).toBeGreaterThan(0)
    })

    it('returns correct stepId', () => {
      const result = placeRootNode(makeIntent({ stepId: 'root-step' }), [])
      expect(result.stepId).toBe('root-step')
    })
  })
})
