import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest'
import { formatRelativeTime } from './formatRelativeTime'

describe('formatRelativeTime', () => {
  const NOW = new Date('2025-06-15T12:00:00Z').getTime()

  beforeEach(() => {
    vi.useFakeTimers()
    vi.setSystemTime(NOW)
  })

  afterEach(() => {
    vi.useRealTimers()
  })

  it('returns empty string for null', () => {
    expect(formatRelativeTime(null)).toBe('')
  })

  it('returns "just now" for timestamps less than 1 minute ago', () => {
    const ts = new Date(NOW - 30_000).toISOString()
    expect(formatRelativeTime(ts)).toBe('just now')
  })

  it('returns minutes ago for timestamps less than 1 hour ago', () => {
    const ts = new Date(NOW - 15 * 60_000).toISOString()
    expect(formatRelativeTime(ts)).toBe('15m ago')
  })

  it('returns hours ago for timestamps less than 24 hours ago', () => {
    const ts = new Date(NOW - 5 * 60 * 60_000).toISOString()
    expect(formatRelativeTime(ts)).toBe('5h ago')
  })

  it('returns days ago for timestamps less than 30 days ago', () => {
    const ts = new Date(NOW - 3 * 24 * 60 * 60_000).toISOString()
    expect(formatRelativeTime(ts)).toBe('3d ago')
  })

  it('returns locale date string for timestamps older than 30 days', () => {
    const ts = new Date(NOW - 60 * 24 * 60 * 60_000).toISOString()
    const result = formatRelativeTime(ts)
    // Falls back to toLocaleDateString
    expect(result).toBeTruthy()
    expect(result).not.toContain('ago')
  })

  it('handles boundary at exactly 1 minute', () => {
    const ts = new Date(NOW - 60_000).toISOString()
    expect(formatRelativeTime(ts)).toBe('1m ago')
  })

  it('handles boundary at exactly 1 hour', () => {
    const ts = new Date(NOW - 60 * 60_000).toISOString()
    expect(formatRelativeTime(ts)).toBe('1h ago')
  })

  it('handles boundary at exactly 1 day', () => {
    const ts = new Date(NOW - 24 * 60 * 60_000).toISOString()
    expect(formatRelativeTime(ts)).toBe('1d ago')
  })
})
