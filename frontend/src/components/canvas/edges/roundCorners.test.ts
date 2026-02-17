import { describe, it, expect } from 'vitest'
import { roundCorners } from './roundCorners'

describe('roundCorners', () => {
  it('returns straight lines unchanged', () => {
    expect(roundCorners('M 0 0 L 100 0')).toBe('M 0 0 L 100 0')
  })

  it('returns single point unchanged', () => {
    expect(roundCorners('M 50 50')).toBe('M 50 50')
  })

  it('returns original path when radius is 0', () => {
    const path = 'M 0 0 L 50 0 L 50 100'
    expect(roundCorners(path, 0)).toBe(path)
  })

  it('returns original path when radius is negative', () => {
    const path = 'M 0 0 L 50 0 L 50 100'
    expect(roundCorners(path, -5)).toBe(path)
  })

  it('rounds a single corner (L-shape)', () => {
    const result = roundCorners('M 0 0 L 100 0 L 100 100', 8)
    expect(result).toContain('Q')
    expect(result).toMatch(/^M 0 0/)
    expect(result).toMatch(/L 100 100$/)
  })

  it('rounds two corners (S-shape)', () => {
    const result = roundCorners('M 0 0 L 50 0 L 50 100 L 100 100', 8)
    const qCount = (result.match(/Q/g) ?? []).length
    expect(qCount).toBe(2)
    expect(result).toMatch(/^M 0 0/)
    expect(result).toMatch(/L 100 100$/)
  })

  it('clamps radius to half shortest segment', () => {
    // Segments: 10px horizontal, 100px vertical — radius 8 should be clamped to 5 (10/2)
    const result = roundCorners('M 0 0 L 10 0 L 10 100', 8)
    expect(result).toContain('Q')
    // Approach point should be at x=5 (10 - 5 = 5), not x=2 (10 - 8)
    expect(result).toContain('L 5 0')
  })

  it('keeps sharp corner when both segments are too short', () => {
    // Both segments < 2px — radius clamped below 1px threshold
    const result = roundCorners('M 0 0 L 0.5 0 L 0.5 0.5', 8)
    expect(result).not.toContain('Q')
  })

  it('skips collinear points (all same X)', () => {
    const result = roundCorners('M 0 0 L 0 50 L 0 100', 8)
    expect(result).not.toContain('Q')
  })

  it('skips collinear points (all same Y)', () => {
    const result = roundCorners('M 0 0 L 50 0 L 100 0', 8)
    expect(result).not.toContain('Q')
  })

  it('preserves start and end points exactly', () => {
    const result = roundCorners('M 10 20 L 50 20 L 50 80 L 90 80', 8)
    expect(result).toMatch(/^M 10 20/)
    expect(result).toMatch(/L 90 80$/)
  })

  it('handles multiple corners in corridor path', () => {
    // 5-segment corridor path has 4 intermediate corners
    const path = 'M 100 0 L 100 24 L 76 24 L 76 -476 L 200 -476 L 200 -500'
    const result = roundCorners(path, 8)
    const qCount = (result.match(/Q/g) ?? []).length
    expect(qCount).toBe(4)
    expect(result).toMatch(/^M 100 0/)
    expect(result).toMatch(/L 200 -500$/)
  })

  it('uses default CORNER_RADIUS from PIPE constant', () => {
    // When no radius specified, should still produce rounded corners
    const result = roundCorners('M 0 0 L 100 0 L 100 100')
    expect(result).toContain('Q')
  })

  it('produces correct approach and depart points for right-angle bend', () => {
    // 90-degree bend at (100, 0) with radius 10
    // Approach: 10px before corner along incoming (horizontal) = (90, 0)
    // Depart: 10px after corner along outgoing (vertical) = (100, 10)
    const result = roundCorners('M 0 0 L 100 0 L 100 100', 10)
    expect(result).toBe('M 0 0 L 90 0 Q 100 0 100 10 L 100 100')
  })
})
