import { describe, it, expect } from 'vitest'
import { formatDuration, formatTokens, formatCost } from './formatters'

describe('formatDuration', () => {
  it('formats sub-second as milliseconds', () => {
    expect(formatDuration(500)).toBe('500ms')
    expect(formatDuration(0)).toBe('0ms')
    expect(formatDuration(999)).toBe('999ms')
  })

  it('formats seconds with one decimal', () => {
    expect(formatDuration(1000)).toBe('1.0s')
    expect(formatDuration(5500)).toBe('5.5s')
    expect(formatDuration(59999)).toBe('60.0s')
  })

  it('formats minutes with one decimal', () => {
    expect(formatDuration(60000)).toBe('1.0m')
    expect(formatDuration(90000)).toBe('1.5m')
    expect(formatDuration(600000)).toBe('10.0m')
  })
})

describe('formatTokens', () => {
  it('formats sub-thousand as raw number', () => {
    expect(formatTokens(0)).toBe('0')
    expect(formatTokens(500)).toBe('500')
    expect(formatTokens(999)).toBe('999')
  })

  it('formats thousands with k suffix', () => {
    expect(formatTokens(1000)).toBe('1.0k')
    expect(formatTokens(1500)).toBe('1.5k')
    expect(formatTokens(25000)).toBe('25.0k')
  })
})

describe('formatCost', () => {
  it('formats very small costs with 4 decimal places', () => {
    expect(formatCost(0.001)).toBe('$0.0010')
    expect(formatCost(0.0099)).toBe('$0.0099')
  })

  it('formats normal costs with 2 decimal places', () => {
    expect(formatCost(0.01)).toBe('$0.01')
    expect(formatCost(0.025)).toBe('$0.03')
    expect(formatCost(1.50)).toBe('$1.50')
  })
})
