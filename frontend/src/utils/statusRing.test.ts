import { describe, it, expect } from 'vitest'
import { resolveStatusRing } from './statusRing'
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

const resolve = (
  status: Parameters<typeof resolveStatusRing>[0]['status'],
  designStatus: Parameters<typeof resolveStatusRing>[0]['designStatus'] = null,
  animated = true,
) => resolveStatusRing({ status, designStatus, palette, animated })

describe('resolveStatusRing', () => {
  describe('run axis', () => {
    it('gives running a pulsing, glowing ring', () => {
      expect(resolve('running')).toEqual({
        color: '#running', dashed: false, dim: false, glow: true, pulse: true,
      })
    })

    it('gives finished a quiet ring — done is not an interruption', () => {
      const ring = resolve('success')
      expect(ring?.color).toBe('#finished')
      expect(ring?.glow).toBe(false)
      expect(ring?.pulse).toBe(false)
    })

    it('gives failed a glow but no pulse — persistent, not animated', () => {
      const ring = resolve('error')
      expect(ring?.color).toBe('#failed')
      expect(ring?.glow).toBe(true)
      expect(ring?.pulse).toBe(false)
    })

    it('draws skipped dashed and dims the node body', () => {
      const ring = resolve('skipped')
      expect(ring?.dashed).toBe(true)
      expect(ring?.dim).toBe(true)
    })

    it('returns null for idle so the plain border shows through', () => {
      expect(resolve('idle')).toBeNull()
    })
  })

  describe('design axis', () => {
    it('speaks only when the run axis is silent', () => {
      expect(resolve('idle', 'running')?.color).toBe('#designing')
      expect(resolve('idle', 'completed')?.color).toBe('#designed')
    })

    // A node that failed a run keeps reading as failed while it is redesigned.
    it('never overrides a run state', () => {
      expect(resolve('error', 'running')?.color).toBe('#failed')
      expect(resolve('success', 'completed')?.color).toBe('#finished')
    })

    it('pulses while designing', () => {
      expect(resolve('idle', 'running')?.pulse).toBe(true)
      expect(resolve('idle', 'completed')?.pulse).toBe(false)
    })

    it('returns null when both axes are idle', () => {
      expect(resolve('idle', null)).toBeNull()
      expect(resolve('idle', 'idle')).toBeNull()
    })
  })

  // Zoomed out is when the ring matters most and when there are most of them.
  it('drops motion when not animated, keeping the ring itself', () => {
    const ring = resolve('running', null, false)
    expect(ring?.color).toBe('#running')
    expect(ring?.pulse).toBe(false)
    expect(ring?.glow).toBe(true)
  })
})
