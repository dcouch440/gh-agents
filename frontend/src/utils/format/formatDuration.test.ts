import { describe, it, expect } from 'vitest'
import { formatDuration } from './formatDuration'

describe('formatDuration', () => {
  it('formats sub-second durations in milliseconds', () => {
    expect(formatDuration(0)).toBe('0ms')
    expect(formatDuration(150)).toBe('150ms')
    expect(formatDuration(999)).toBe('999ms')
  })

  it('formats durations under a minute in seconds', () => {
    expect(formatDuration(1000)).toBe('1.0s')
    expect(formatDuration(1500)).toBe('1.5s')
    expect(formatDuration(59_999)).toBe('60.0s')
  })

  it('formats durations over a minute in minutes', () => {
    expect(formatDuration(60_000)).toBe('1.0m')
    expect(formatDuration(90_000)).toBe('1.5m')
    expect(formatDuration(300_000)).toBe('5.0m')
  })
})
