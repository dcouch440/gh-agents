import { describe, it, expect } from 'vitest'
import { notchToGrid } from './gridNotch'

describe('notchToGrid', () => {
  it('rounds to nearest grid multiple', () => {
    expect(notchToGrid(100, 24, 0)).toBe(96)    // 100/24=4.17 → 4 → 96
    expect(notchToGrid(108, 24, 0)).toBe(120)   // 108/24=4.5  → 5 → 120
    expect(notchToGrid(112, 24, 0)).toBe(120)   // 112/24=4.67 → 5 → 120
    expect(notchToGrid(120, 24, 0)).toBe(120)   // exact
    expect(notchToGrid(85, 24, 0)).toBe(96)     // 85/24=3.54  → 4 → 96
  })

  it('clamps to minimum value', () => {
    expect(notchToGrid(10, 24, 360)).toBe(360)
    expect(notchToGrid(0, 24, 360)).toBe(360)
    expect(notchToGrid(300, 24, 360)).toBe(360)  // rounded (288) < min
  })

  it('handles exact grid multiples', () => {
    expect(notchToGrid(48, 24, 0)).toBe(48)
    expect(notchToGrid(72, 24, 0)).toBe(72)
    expect(notchToGrid(240, 24, 0)).toBe(240)
  })

  it('handles different grid sizes', () => {
    expect(notchToGrid(15, 10, 0)).toBe(20)   // rounds to nearest 10
    expect(notchToGrid(14, 10, 0)).toBe(10)
    expect(notchToGrid(100, 50, 0)).toBe(100)
  })

  it('handles min equal to zero', () => {
    expect(notchToGrid(5, 24, 0)).toBe(0)
  })

  it('handles value exactly at min', () => {
    expect(notchToGrid(360, 24, 360)).toBe(360)
  })
})
