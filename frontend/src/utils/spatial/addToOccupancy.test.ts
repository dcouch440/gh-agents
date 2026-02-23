import { describe, it, expect } from 'vitest'
import { addToOccupancy } from './addToOccupancy'

describe('addToOccupancy', () => {
  it('appends a new entry with padded rect', () => {
    const result = addToOccupancy([], 'a', { x: 10, y: 20, width: 30, height: 40 }, 5)
    expect(result).toHaveLength(1)
    expect(result[0]!.id).toBe('a')
    expect(result[0]!.rect).toEqual({ x: 10, y: 20, width: 30, height: 40 })
    expect(result[0]!.paddedRect).toEqual({ x: 5, y: 15, width: 40, height: 50 })
  })

  it('preserves existing entries', () => {
    const existing = [{
      id: 'x',
      rect: { x: 0, y: 0, width: 50, height: 50 },
      paddedRect: { x: -5, y: -5, width: 60, height: 60 },
    }]
    const result = addToOccupancy(existing, 'y', { x: 100, y: 100, width: 50, height: 50 }, 5)
    expect(result).toHaveLength(2)
    expect(result[0]).toBe(existing[0])
  })

  it('does not mutate the original array', () => {
    const original: readonly { id: string; rect: { x: number; y: number; width: number; height: number }; paddedRect: { x: number; y: number; width: number; height: number } }[] = []
    addToOccupancy(original, 'a', { x: 0, y: 0, width: 10, height: 10 }, 5)
    expect(original).toHaveLength(0)
  })
})
