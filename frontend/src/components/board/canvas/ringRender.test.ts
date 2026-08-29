import { describe, it, expect, vi } from 'vitest'
import { renderBoard, resolveBoxStroke } from './renderer'
import type { DrawTheme } from './renderer'
import { addBox, createBox, emptyBoard } from '../elements'
import type { StatusRing } from '@/utils/statusRing'

/**
 * These assert that a ring reaches the canvas at all.
 *
 * jsdom has no 2D context, so the board's painter is normally stubbed out in
 * tests and nothing verifies that a status actually results in pixels. A
 * recording context closes that gap: it runs the real `renderBoard` and counts
 * what it asked the canvas to do.
 */

const theme = {
  canvasBg: '#000', gridDotColor: '#111', connectorColor: '#222',
  strokeColor: '#888', accentColor: '#444', surfaceBg: '#333', textColor: '#fff',
} as DrawTheme

const CANVAS_METHODS = [
  'save', 'restore', 'clearRect', 'setTransform', 'translate', 'scale', 'beginPath',
  'roundRect', 'fill', 'stroke', 'arc', 'moveTo', 'lineTo', 'closePath', 'fillText',
  'measureText', 'setLineDash', 'rect', 'quadraticCurveTo', 'bezierCurveTo', 'ellipse',
  'fillRect', 'clip', 'drawImage', 'createLinearGradient',
] as const

const recordingCanvas = () => {
  const calls: string[] = []
  const ctx: Record<string, unknown> = {}
  for (const name of CANVAS_METHODS) {
    ctx[name] = vi.fn((...args: unknown[]) => {
      calls.push(`${name}(${args.join(',')})`)
      return name === 'measureText' ? { width: 10 } : undefined
    })
  }
  return {
    canvas: { getContext: () => ctx, width: 100, height: 100 } as unknown as HTMLCanvasElement,
    calls,
    strokeCount: () => calls.filter((c) => c.startsWith('stroke(')).length,
  }
}

const BOX_ID = 'el-1'
const board = addBox(emptyBoard(), { ...createBox(10, 10, 'Scanner'), id: BOX_ID })
const selection = { selectedIds: new Set<string>(), marquee: null }
const viewport = { panX: 0, panY: 0, zoom: 1 }

const paint = (rings: ReadonlyMap<string, StatusRing>) => {
  const rec = recordingCanvas()
  renderBoard(
    rec.canvas, 100, 100, board, selection, null, viewport,
    null, null, null, null, theme, rings, 1,
  )
  return rec
}

const ring = (over: Partial<StatusRing> = {}): StatusRing => ({
  color: '#3fb950', dashed: false, glow: false, pulse: false, dim: false, ...over,
})

describe('resolveBoxStroke', () => {
  // Status replaces the outline rather than adding a ring beside it.
  it('paints the box outline with the status color', () => {
    expect(resolveBoxStroke(ring({ color: '#3fb950' }), false, theme))
      .toMatchObject({ color: '#3fb950' })
  })

  it('outranks selection, which still has its resize handles', () => {
    expect(resolveBoxStroke(ring({ color: '#3fb950' }), true, theme).color).toBe('#3fb950')
  })

  it('falls back to the accent when selected with no status', () => {
    expect(resolveBoxStroke(null, true, theme).color).toBe(theme.accentColor)
  })

  it('falls back to the plain stroke when idle and unselected', () => {
    const stroke = resolveBoxStroke(null, false, theme)
    expect(stroke.color).toBe(theme.strokeColor)
    expect(stroke.dash).toBeUndefined()
  })

  it('thickens a status outline so it reads as deliberate', () => {
    expect(resolveBoxStroke(ring(), false, theme).width)
      .toBeGreaterThan(resolveBoxStroke(null, false, theme).width)
  })

  it('dashes a skipped step', () => {
    expect(resolveBoxStroke(ring({ dashed: true }), false, theme).dash).toEqual([7, 5])
  })
})

describe('renderBoard — status glow', () => {
  // The glow is the only part drawn outside the box: padded by 3 from a box at
  // 10,10, so roundRect(7,7…) appearing proves the pre-pass ran on real geometry.
  it('strokes a halo outside the box for a glowing state', () => {
    expect(paint(new Map([[BOX_ID, ring({ glow: true })]]))
      .calls.some((c) => c.startsWith('roundRect(7,7'))).toBe(true)
  })

  it('draws no halo for a non-glowing state', () => {
    expect(paint(new Map([[BOX_ID, ring()]]))
      .calls.some((c) => c.startsWith('roundRect(7,7'))).toBe(false)
  })

  it('draws no halo when nothing has a status', () => {
    expect(paint(new Map()).calls.some((c) => c.startsWith('roundRect(7,7'))).toBe(false)
  })

  it('ignores a ring whose box is not on the board', () => {
    expect(paint(new Map([['ghost-element', ring({ glow: true })]]))
      .calls.some((c) => c.startsWith('roundRect(7,7'))).toBe(false)
  })
})
