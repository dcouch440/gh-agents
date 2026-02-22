import { describe, it, expect } from 'vitest'
import type { Rect } from '@/utils/geometry'
import type { PlacementIntent } from './types'
import { findFreeSpace } from './freeSpaceFinder'
import { buildOccupancyIndex } from './occupancyIndex'
import { PLACEMENT } from './constants'

const makeIntent = (overrides?: Partial<PlacementIntent>): PlacementIntent => ({
  stepId: 'test-step',
  width: 560,
  height: 500,
  strategy: 'free_space',
  upstreamStepId: null,
  downstreamStepIds: [],
  ...overrides,
})

const makeOccNode = (id: string, x: number, y: number, w: number, h: number) => ({
  id,
  rect: { x, y, width: w, height: h } as Rect,
})

describe('freeSpaceFinder', () => {
  describe('findFreeSpace', () => {
    it('returns seed position when space is free', () => {
      const result = findFreeSpace(makeIntent(), { x: 100, y: 200 }, [])

      // Snapped to grid
      expect(result.position.x % PLACEMENT.GRID_SIZE).toBe(0)
      expect(result.position.y % PLACEMENT.GRID_SIZE).toBe(0)
      expect(result.position.x).toBe(96) // nearest grid snap of 100
      expect(result.position.y).toBe(192) // nearest grid snap of 200
    })

    it('shifts right when seed is occupied', () => {
      const occupancy = buildOccupancyIndex([
        makeOccNode('blocker', 96, 192, 560, 500),
      ])

      const result = findFreeSpace(makeIntent(), { x: 96, y: 192 }, occupancy)

      // Should find a position to the right of the blocker
      expect(result.position.x).toBeGreaterThan(96)
      expect(result.stepId).toBe('test-step')
    })

    it('wraps to next row when rightward scan is blocked', () => {
      // Create a wide wall of blockers at y=0 that spans MAX_SCAN_COLS
      const blockers = []
      for (let col = 0; col < PLACEMENT.MAX_SCAN_COLS; col++) {
        blockers.push(
          makeOccNode(`b${col}`, col * PLACEMENT.GRID_SIZE, 0, PLACEMENT.GRID_SIZE, 100),
        )
      }
      const occupancy = buildOccupancyIndex(blockers)

      const result = findFreeSpace(
        makeIntent({ width: 20, height: 20 }),
        { x: 0, y: 0 },
        occupancy,
      )

      // Should wrap to a lower row
      expect(result.position.y).toBeGreaterThanOrEqual(PLACEMENT.V_GAP)
    })

    it('snaps result to grid', () => {
      const result = findFreeSpace(makeIntent(), { x: 13, y: 7 }, [])

      expect(result.position.x % PLACEMENT.GRID_SIZE).toBe(0)
      expect(result.position.y % PLACEMENT.GRID_SIZE).toBe(0)
    })

    it('returns correct stepId', () => {
      const result = findFreeSpace(makeIntent({ stepId: 'my-orphan' }), { x: 0, y: 0 }, [])
      expect(result.stepId).toBe('my-orphan')
    })

    it('handles empty occupancy index', () => {
      const result = findFreeSpace(makeIntent(), { x: 0, y: 0 }, [])

      expect(result.position.x).toBe(0)
      expect(result.position.y).toBe(0)
    })
  })
})
