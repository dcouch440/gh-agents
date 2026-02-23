import { describe, it, expect } from 'vitest'
import { midpoint } from './midpoint'

describe('midpoint', () => {
  it('returns midpoint of two points on the x-axis', () => {
    expect(midpoint({ x: 0, y: 0 }, { x: 100, y: 0 })).toEqual({ x: 50, y: 0 })
  })

  it('returns midpoint of two points on the y-axis', () => {
    expect(midpoint({ x: 0, y: 0 }, { x: 0, y: 80 })).toEqual({ x: 0, y: 40 })
  })

  it('returns midpoint of diagonal points', () => {
    expect(midpoint({ x: 10, y: 20 }, { x: 30, y: 40 })).toEqual({ x: 20, y: 30 })
  })

  it('returns the same point when both inputs are identical', () => {
    expect(midpoint({ x: 5, y: 5 }, { x: 5, y: 5 })).toEqual({ x: 5, y: 5 })
  })

  it('handles negative coordinates', () => {
    expect(midpoint({ x: -10, y: -20 }, { x: 10, y: 20 })).toEqual({ x: 0, y: 0 })
  })
})
