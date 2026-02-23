import { describe, it, expect } from 'vitest'
import { occupancyBounds } from './occupancyBounds'
import type { OccupiedRect } from './types'

describe('occupancyBounds', () => {
  it('returns null for empty occupancy', () => {
    expect(occupancyBounds([])).toBeNull()
  })

  it('returns the rect for a single entry', () => {
    const occ: OccupiedRect[] = [
      { id: 'a', rect: { x: 10, y: 20, width: 30, height: 40 }, paddedRect: { x: 5, y: 15, width: 40, height: 50 } },
    ]
    expect(occupancyBounds(occ)).toEqual({ x: 10, y: 20, width: 30, height: 40 })
  })

  it('returns bounding box of multiple rects (non-padded)', () => {
    const occ: OccupiedRect[] = [
      { id: 'a', rect: { x: 0, y: 0, width: 50, height: 50 }, paddedRect: { x: -5, y: -5, width: 60, height: 60 } },
      { id: 'b', rect: { x: 100, y: 100, width: 50, height: 50 }, paddedRect: { x: 95, y: 95, width: 60, height: 60 } },
    ]
    expect(occupancyBounds(occ)).toEqual({ x: 0, y: 0, width: 150, height: 150 })
  })
})
