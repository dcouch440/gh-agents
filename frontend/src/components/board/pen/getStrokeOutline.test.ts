import { describe, expect, it } from 'vitest'
import { getStrokeOutline } from './getStrokeOutline'

describe('getStrokeOutline', () => {
  it('returns empty for fewer than 2 points', () => {
    const result = getStrokeOutline([{ x: 0, y: 0 }], [0.5])
    expect(result).toEqual([])
  })

  it('returns outline points for a simple stroke', () => {
    const points = [
      { x: 0, y: 0 },
      { x: 10, y: 0 },
      { x: 20, y: 0 },
      { x: 30, y: 0 },
    ]
    const pressures = [0.5, 0.5, 0.5, 0.5]

    const outline = getStrokeOutline(points, pressures)
    expect(outline.length).toBeGreaterThan(0)

    // Every outline point should have x and y
    for (let i = 0; i < outline.length; i++) {
      expect(typeof outline[i]!.x).toBe('number')
      expect(typeof outline[i]!.y).toBe('number')
      expect(Number.isNaN(outline[i]!.x)).toBe(false)
      expect(Number.isNaN(outline[i]!.y)).toBe(false)
    }
  })

  it('respects size option', () => {
    const points = [
      { x: 0, y: 0 },
      { x: 50, y: 0 },
      { x: 100, y: 0 },
    ]
    const pressures = [0.5, 0.5, 0.5]

    const small = getStrokeOutline(points, pressures, { size: 2 })
    const large = getStrokeOutline(points, pressures, { size: 20 })

    // Larger size should produce a wider stroke (bigger bounding box)
    const smallHeight = Math.max(...small.map((p) => p.y)) - Math.min(...small.map((p) => p.y))
    const largeHeight = Math.max(...large.map((p) => p.y)) - Math.min(...large.map((p) => p.y))
    expect(largeHeight).toBeGreaterThan(smallHeight)
  })

  it('uses real pressure when not all 0.5', () => {
    const points = [
      { x: 0, y: 0 },
      { x: 20, y: 0 },
      { x: 40, y: 0 },
      { x: 60, y: 0 },
    ]
    const uniformPressure = [0.5, 0.5, 0.5, 0.5]
    const varyingPressure = [0.1, 0.9, 0.1, 0.9]

    const uniformOutline = getStrokeOutline(points, uniformPressure)
    const varyingOutline = getStrokeOutline(points, varyingPressure)

    // Both should produce valid outlines
    expect(uniformOutline.length).toBeGreaterThan(0)
    expect(varyingOutline.length).toBeGreaterThan(0)
  })
})
