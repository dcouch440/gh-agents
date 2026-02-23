import { describe, it, expect } from 'vitest'
import { snapToGridMin } from './snapToGridMin'

describe('snapToGridMin', () => {
  it('snaps to the nearest grid multiple', () => {
    expect(snapToGridMin(47, 24, 0)).toBe(48)
    expect(snapToGridMin(36, 24, 0)).toBe(48)
    expect(snapToGridMin(24, 24, 0)).toBe(24)
  })

  it('clamps to the minimum value', () => {
    expect(snapToGridMin(10, 24, 48)).toBe(48)
    expect(snapToGridMin(0, 24, 48)).toBe(48)
  })

  it('returns the grid-snapped value when above minimum', () => {
    expect(snapToGridMin(100, 24, 48)).toBe(96)
    expect(snapToGridMin(200, 24, 48)).toBe(192)
  })

  it('handles exact grid values', () => {
    expect(snapToGridMin(48, 24, 24)).toBe(48)
    expect(snapToGridMin(72, 24, 24)).toBe(72)
  })
})
