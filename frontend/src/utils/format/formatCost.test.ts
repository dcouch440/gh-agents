import { describe, it, expect } from 'vitest'
import { formatCost } from './formatCost'

describe('formatCost', () => {
  it('formats costs under $0.01 with 4 decimal places', () => {
    expect(formatCost(0.0001)).toBe('$0.0001')
    expect(formatCost(0.0099)).toBe('$0.0099')
  })

  it('formats costs $0.01+ with 2 decimal places', () => {
    expect(formatCost(0.01)).toBe('$0.01')
    expect(formatCost(1.23)).toBe('$1.23')
    expect(formatCost(100)).toBe('$100.00')
  })
})
