import { describe, it, expect } from 'vitest'
import { brightenHex, computePipeOpacities } from './pipeEdgeUtils'
import { PIPE } from './constants'

describe('brightenHex', () => {
  it('returns white when factor is 1', () => {
    expect(brightenHex('#000000', 1)).toBe('#ffffff')
    expect(brightenHex('#3b82f6', 1)).toBe('#ffffff')
  })

  it('returns the same color when factor is 0', () => {
    expect(brightenHex('#3b82f6', 0)).toBe('#3b82f6')
    expect(brightenHex('#000000', 0)).toBe('#000000')
  })

  it('returns midpoint gray for black at factor 0.5', () => {
    expect(brightenHex('#000000', 0.5)).toBe('#808080')
  })

  it('brightens a blue color toward white', () => {
    const result = brightenHex('#3b82f6', 0.4)
    // #3b -> 59 + (255-59)*0.4 = 59 + 78.4 = 137.4 -> 137 -> 0x89
    // #82 -> 130 + (255-130)*0.4 = 130 + 50 = 180 -> 0xb4
    // #f6 -> 246 + (255-246)*0.4 = 246 + 3.6 = 249.6 -> 250 -> 0xfa
    expect(result).toBe('#89b4fa')
  })

  it('handles already-white color', () => {
    expect(brightenHex('#ffffff', 0.5)).toBe('#ffffff')
  })

  it('handles hex without hash', () => {
    expect(brightenHex('000000', 1)).toBe('#ffffff')
  })
})

describe('computePipeOpacities', () => {
  it('returns selected opacities when selected', () => {
    const result = computePipeOpacities(true, true)
    expect(result.glow).toBe(PIPE.GLOW_OPACITY_SELECTED)
    expect(result.body).toBe(PIPE.BODY_OPACITY_SELECTED)
    expect(result.core).toBe(PIPE.CORE_OPACITY_SELECTED)
    expect(result.particle).toBe(PIPE.PARTICLE_OPACITY_SELECTED)
  })

  it('returns selected opacities even when not protocol', () => {
    const result = computePipeOpacities(false, true)
    expect(result.glow).toBe(PIPE.GLOW_OPACITY_SELECTED)
    expect(result.body).toBe(PIPE.BODY_OPACITY_SELECTED)
  })

  it('returns protocol opacities for protocol edges', () => {
    const result = computePipeOpacities(true, false)
    expect(result.glow).toBe(PIPE.GLOW_OPACITY)
    expect(result.body).toBe(PIPE.BODY_OPACITY)
    expect(result.core).toBe(PIPE.CORE_OPACITY)
    expect(result.particle).toBe(PIPE.PARTICLE_OPACITY)
  })

  it('returns dim opacities for non-protocol, non-selected edges', () => {
    const result = computePipeOpacities(false, false)
    expect(result.glow).toBe(0)
    expect(result.body).toBe(PIPE.BODY_OPACITY_DIM)
    expect(result.core).toBe(PIPE.CORE_OPACITY_DIM)
    expect(result.particle).toBe(0)
  })
})
