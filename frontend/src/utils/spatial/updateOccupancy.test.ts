import { describe, it, expect } from 'vitest'
import { updateOccupancy } from './updateOccupancy'
import type { OccupiedRect } from './types'

describe('updateOccupancy', () => {
  const existing: OccupiedRect[] = [
    { id: 'a', rect: { x: 0, y: 0, width: 50, height: 50 }, paddedRect: { x: -5, y: -5, width: 60, height: 60 } },
    { id: 'b', rect: { x: 100, y: 100, width: 50, height: 50 }, paddedRect: { x: 95, y: 95, width: 60, height: 60 } },
  ]

  it('updates the matching entry with new rect and padding', () => {
    const result = updateOccupancy(existing, 'a', { x: 10, y: 20, width: 50, height: 50 }, 5)
    expect(result[0]!.rect).toEqual({ x: 10, y: 20, width: 50, height: 50 })
    expect(result[0]!.paddedRect).toEqual({ x: 5, y: 15, width: 60, height: 60 })
    expect(result[1]).toBe(existing[1]) // other entry unchanged
  })

  it('returns original array when ID not found', () => {
    const result = updateOccupancy(existing, 'z', { x: 0, y: 0, width: 10, height: 10 }, 5)
    expect(result).toBe(existing)
  })

  it('does not mutate the original array', () => {
    const result = updateOccupancy(existing, 'a', { x: 50, y: 50, width: 50, height: 50 }, 5)
    expect(result).not.toBe(existing)
    expect(existing[0]!.rect.x).toBe(0) // unchanged
  })
})
