import { describe, it, expect } from 'vitest'
import { lerpPoint } from './lerpPoint'

describe('lerpPoint', () => {
  it('returns a when distance is 0', () => {
    expect(lerpPoint({ x: 10, y: 20 }, { x: 50, y: 20 }, 0)).toEqual({ x: 10, y: 20 })
  })

  it('returns b when distance equals segment length', () => {
    const a = { x: 0, y: 0 }
    const b = { x: 40, y: 0 }
    expect(lerpPoint(a, b, 40)).toEqual({ x: 40, y: 0 })
  })

  it('returns midpoint when distance is half the segment', () => {
    const a = { x: 0, y: 0 }
    const b = { x: 100, y: 0 }
    expect(lerpPoint(a, b, 50)).toEqual({ x: 50, y: 0 })
  })

  it('works with diagonal segments', () => {
    const a = { x: 0, y: 0 }
    const b = { x: 3, y: 4 } // distance = 5
    const result = lerpPoint(a, b, 2.5) // halfway
    expect(result.x).toBeCloseTo(1.5)
    expect(result.y).toBeCloseTo(2)
  })

  it('returns copy of a when points are coincident', () => {
    const a = { x: 5, y: 5 }
    const result = lerpPoint(a, { x: 5, y: 5 }, 10)
    expect(result).toEqual({ x: 5, y: 5 })
    expect(result).not.toBe(a)
  })
})
