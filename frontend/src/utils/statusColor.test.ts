import { describe, it, expect } from 'vitest'
import { statusColor, designStatusColor } from './statusColor'
import type { StatusPalette } from '@/theme'

const palette: StatusPalette = {
  pending: '#pending',
  running: '#running',
  finished: '#finished',
  failed: '#failed',
  paused: '#paused',
  skipped: '#skipped',
  designing: '#designing',
  designed: '#designed',
}

describe('statusColor', () => {
  it('maps the run states', () => {
    expect(statusColor('running', palette)).toBe('#running')
    expect(statusColor('paused', palette)).toBe('#paused')
    expect(statusColor('skipped', palette)).toBe('#skipped')
    expect(statusColor('pending', palette)).toBe('#pending')
  })

  // Three vocabularies describe the same two states; all of them must land on
  // the same color or the sidebar and the canvas drift apart.
  it('treats success/completed/complete as one finished state', () => {
    expect(statusColor('success', palette)).toBe('#finished')
    expect(statusColor('completed', palette)).toBe('#finished')
    expect(statusColor('complete', palette)).toBe('#finished')
  })

  it('treats error and failed as one failed state', () => {
    expect(statusColor('error', palette)).toBe('#failed')
    expect(statusColor('failed', palette)).toBe('#failed')
  })

  it('returns null for idle — no color is the right answer for nothing happening', () => {
    expect(statusColor('idle', palette)).toBeNull()
  })

  it('stays silent on an unrecognized status rather than guessing', () => {
    expect(statusColor('cancelled', palette)).toBeNull()
    expect(statusColor('', palette)).toBeNull()
  })
})

describe('designStatusColor', () => {
  it('maps the design axis to its own hues', () => {
    expect(designStatusColor('running', palette)).toBe('#designing')
    expect(designStatusColor('completed', palette)).toBe('#designed')
  })

  it('shares the failed color with the run axis', () => {
    expect(designStatusColor('failed', palette)).toBe('#failed')
  })

  it('returns null for idle', () => {
    expect(designStatusColor('idle', palette)).toBeNull()
  })
})
