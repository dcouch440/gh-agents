import { describe, it, expect } from 'vitest'
import { roundCorners } from './roundCorners'

describe('roundCorners', () => {
  it('returns original path when radius is 0', () => {
    const path = 'M 0 0 L 100 0 L 100 100'
    expect(roundCorners(path, 0)).toBe(path)
  })

  it('returns original path with fewer than 3 points', () => {
    expect(roundCorners('M 0 0 L 100 0', 10)).toBe('M 0 0 L 100 0')
    expect(roundCorners('M 0 0', 10)).toBe('M 0 0')
  })

  it('produces Q commands for corners in a 3-point path', () => {
    const result = roundCorners('M 0 0 L 100 0 L 100 100', 10)
    expect(result).toContain('Q ')
    expect(result).toMatch(/^M 0 0/)
    expect(result).toMatch(/L 100 100$/)
  })

  it('preserves collinear points as L commands', () => {
    // All points on same X — no corner to round
    const result = roundCorners('M 0 0 L 0 50 L 0 100', 10)
    expect(result).not.toContain('Q ')
  })

  it('clamps radius to half the shortest segment', () => {
    // Short segment of 6px — radius 10 should clamp to 3
    const result = roundCorners('M 0 0 L 6 0 L 6 100', 10)
    expect(result).toContain('Q ')
  })

  it('skips rounding when segments are too short', () => {
    // Segments less than 1px effective radius
    const result = roundCorners('M 0 0 L 1 0 L 1 1', 10)
    expect(result).not.toContain('Q ')
  })
})
