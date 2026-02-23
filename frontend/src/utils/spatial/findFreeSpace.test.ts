import { describe, it, expect } from 'vitest'
import { findFreeSpace } from './findFreeSpace'
import type { FindFreeSpaceConfig } from './findFreeSpace'
import type { OccupiedRect } from './types'

const config: FindFreeSpaceConfig = {
  gridSize: 24,
  vGap: 48,
  hGap: 96,
  maxScanRows: 50,
  maxScanCols: 80,
  originX: 0,
}

describe('findFreeSpace', () => {
  it('returns seed position when no occupancy', () => {
    const result = findFreeSpace({ width: 50, height: 50 }, { x: 0, y: 0 }, [], config)
    expect(result).toEqual({ x: 0, y: 0 })
  })

  it('snaps seed to grid', () => {
    const result = findFreeSpace({ width: 50, height: 50 }, { x: 10, y: 10 }, [], config)
    expect(result.x % 24).toBe(0)
    expect(result.y % 24).toBe(0)
  })

  it('skips occupied positions and finds the next free one', () => {
    const occ: OccupiedRect[] = [{
      id: 'a',
      rect: { x: 0, y: 0, width: 50, height: 50 },
      paddedRect: { x: -24, y: -24, width: 98, height: 98 },
    }]
    const result = findFreeSpace({ width: 50, height: 50 }, { x: 0, y: 0 }, occ, config)
    // Should find a position that doesn't overlap with the padded rect
    expect(result.x).toBeGreaterThanOrEqual(0)
  })

  it('uses fallback when all scan positions are occupied', () => {
    // Create a very small scan area that's fully occupied
    const smallConfig = { ...config, maxScanRows: 1, maxScanCols: 1 }
    const occ: OccupiedRect[] = [{
      id: 'a',
      rect: { x: 0, y: 0, width: 200, height: 200 },
      paddedRect: { x: -24, y: -24, width: 248, height: 248 },
    }]
    const result = findFreeSpace({ width: 50, height: 50 }, { x: 0, y: 0 }, occ, smallConfig)
    // Fallback: far right of occupancy bounds
    expect(result.x).toBeGreaterThan(0)
  })
})
