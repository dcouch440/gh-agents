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
  color: '#3fb950', glow: false, pulse: false, dim: false, ...over,
})

describe('resolveBoxStroke', () => {
  // Status replaces the outline rather than adding a ring beside it.
  it('paints the box outline with the status color', () => {
    expect(resolveBoxStroke(ring({ color: '#3fb950' }), false, theme, 1))
      .toMatchObject({ color: '#3fb950' })
  })

  it('outranks selection, which still has its resize handles', () => {
    expect(resolveBoxStroke(ring({ color: '#3fb950' }), true, theme, 1).color).toBe('#3fb950')
  })

  it('draws a solid outline for every status, dash being reserved', () => {
    for (const r of [ring(), ring({ dim: true }), ring({ glow: true })]) {
      expect(resolveBoxStroke(r, false, theme, 1).dash).toBeUndefined()
    }
  })

  it('thickens a status outline so it reads as deliberate', () => {
    expect(resolveBoxStroke(ring(), false, theme, 1).width)
      .toBeGreaterThan(resolveBoxStroke(null, false, theme, 1).width)
  })

  describe('undesigned', () => {
    it('dashes a box with no status, rather than leaving a plain outline', () => {
      const stroke = resolveBoxStroke(null, false, theme, 1)
      expect(stroke.color).toBe(theme.strokeColor)
      expect(stroke.dash).toEqual([7, 5])
    })

    // Dash says "not designed", color says "selected" — two questions, two channels.
    it('keeps the dash while the accent marks selection', () => {
      const stroke = resolveBoxStroke(null, true, theme, 1)
      expect(stroke.color).toBe(theme.accentColor)
      expect(stroke.dash).toEqual([7, 5])
    })

    // The canvas draws under ctx.scale(zoom), so a world-space dash would shrink
    // to nothing exactly when someone is scanning the board for unbuilt nodes.
    it('scales the dash against zoom so the gaps survive zooming out', () => {
      expect(resolveBoxStroke(null, false, theme, 0.25).dash).toEqual([28, 20])
      expect(resolveBoxStroke(null, false, theme, 2).dash).toEqual([3.5, 2.5])
    })

    it('does not divide by a zero zoom', () => {
      expect(resolveBoxStroke(null, false, theme, 0).dash).toEqual([7, 5])
    })
  })
})

describe('renderBoard — undesigned dash', () => {
  // resolveBoxStroke returning a dash proves nothing on its own: it is handed to
  // rough.js, which decides whether it survives to the context.
  it('sets a line dash on a box with no status', () => {
    expect(paint(new Map()).calls.some((c) => c.startsWith('setLineDash(7,5'))).toBe(true)
  })

  it('sets no line dash once the box has a status', () => {
    expect(paint(new Map([[BOX_ID, ring()]]))
      .calls.some((c) => c.startsWith('setLineDash(7,5'))).toBe(false)
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
