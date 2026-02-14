import { describe, it, expect } from 'vitest'
import { resolveScaleNotch, resolveScaleFactor } from './scaleNotch'

describe('resolveScaleNotch', () => {
  it('returns XS for minimum node size', () => {
    expect(resolveScaleNotch(360, 300)).toBe('XS')
  })

  it('returns M for default node size', () => {
    expect(resolveScaleNotch(560, 500)).toBe('M')
  })

  it('returns XXL for maximum node size', () => {
    expect(resolveScaleNotch(1800, 1600)).toBe('XXL')
  })

  it('caps at height notch when wide but short', () => {
    expect(resolveScaleNotch(1800, 300)).toBe('XS')
  })

  it('caps at width notch when narrow but tall', () => {
    expect(resolveScaleNotch(360, 1600)).toBe('XS')
  })

  it('returns S when height constrains below width', () => {
    expect(resolveScaleNotch(800, 400)).toBe('S')
  })

  it('returns L when both dimensions are moderately large', () => {
    expect(resolveScaleNotch(900, 900)).toBe('L')
  })

  it('returns XL when both dimensions are large', () => {
    expect(resolveScaleNotch(1200, 1200)).toBe('XL')
  })
})

describe('resolveScaleFactor', () => {
  it('returns 1.0 for default size', () => {
    expect(resolveScaleFactor(560, 500)).toBe(1.0)
  })

  it('returns 0.85 for wide+short (height constrains)', () => {
    expect(resolveScaleFactor(1800, 300)).toBe(0.85)
  })

  it('returns 1.4 for max size', () => {
    expect(resolveScaleFactor(1800, 1600)).toBe(1.4)
  })

  it('returns 0.85 for minimum size', () => {
    expect(resolveScaleFactor(360, 300)).toBe(0.85)
  })

  it('returns 0.95 when height constrains a medium-width node', () => {
    expect(resolveScaleFactor(800, 400)).toBe(0.95)
  })
})
