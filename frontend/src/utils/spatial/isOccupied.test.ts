import { describe, it, expect } from 'vitest'
import { isOccupied } from './isOccupied'
import type { OccupiedRect } from './types'

describe('isOccupied', () => {
  const occupancy: OccupiedRect[] = [
    {
      id: 'a',
      rect: { x: 100, y: 100, width: 50, height: 50 },
      paddedRect: { x: 90, y: 90, width: 70, height: 70 },
    },
  ]

  it('returns false for non-overlapping candidate', () => {
    expect(isOccupied({ x: 0, y: 0, width: 50, height: 50 }, occupancy)).toBe(false)
  })

  it('returns true for overlapping candidate (within padded area)', () => {
    expect(isOccupied({ x: 80, y: 80, width: 20, height: 20 }, occupancy)).toBe(true)
  })

  it('returns false for candidate that overlaps only unpadded rect', () => {
    // Candidate touches original rect but not padded rect
    // paddedRect starts at 90, so x=91 overlaps. But x=161 is past paddedRect (90+70=160)
    expect(isOccupied({ x: 161, y: 100, width: 50, height: 50 }, occupancy)).toBe(false)
  })

  it('excludes specified ID', () => {
    expect(isOccupied({ x: 95, y: 95, width: 10, height: 10 }, occupancy, 'a')).toBe(false)
  })

  it('returns false for empty occupancy', () => {
    expect(isOccupied({ x: 0, y: 0, width: 50, height: 50 }, [])).toBe(false)
  })
})
