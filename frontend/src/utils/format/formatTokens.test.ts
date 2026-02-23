import { describe, it, expect } from 'vitest'
import { formatTokens } from './formatTokens'

describe('formatTokens', () => {
  it('formats counts under 1000 as plain numbers', () => {
    expect(formatTokens(0)).toBe('0')
    expect(formatTokens(500)).toBe('500')
    expect(formatTokens(999)).toBe('999')
  })

  it('formats counts 1000+ with k suffix', () => {
    expect(formatTokens(1000)).toBe('1.0k')
    expect(formatTokens(1500)).toBe('1.5k')
    expect(formatTokens(10_000)).toBe('10.0k')
    expect(formatTokens(150_000)).toBe('150.0k')
  })
})
