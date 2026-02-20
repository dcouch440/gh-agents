import { describe, it, expect } from 'vitest'
import { getNodeHighlightStyles } from './nodeHighlightStyles'
import { HighlightMode } from './canvasKinds'

const ACCENT = '#E57373'
const SCREEN_BORDER = '#d6cfc4'
const ACCENT_RING = 'rgba(90, 138, 110, 0.18)'

/** Shared defaults for every test call. */
const defaults = { screenBorder: SCREEN_BORDER, accentRing: ACCENT_RING } as const

describe('getNodeHighlightStyles', () => {
  describe('dark mode — borderColor', () => {
    it('returns accentColor when selected', () => {
      const { borderColor } = getNodeHighlightStyles({
        ...defaults,
        selected: true,
        accentColor: ACCENT,
        highlightMode: HighlightMode.NONE,
        themeMode: 'dark',
      })
      expect(borderColor).toBe(ACCENT)
    })

    it('returns accentColor for SELECT highlight', () => {
      const { borderColor } = getNodeHighlightStyles({
        ...defaults,
        selected: false,
        accentColor: ACCENT,
        highlightMode: HighlightMode.SELECT,
        themeMode: 'dark',
      })
      expect(borderColor).toBe(ACCENT)
    })

    it('returns accentColor+80 for HOVER highlight', () => {
      const { borderColor } = getNodeHighlightStyles({
        ...defaults,
        selected: false,
        accentColor: ACCENT,
        highlightMode: HighlightMode.HOVER,
        themeMode: 'dark',
      })
      expect(borderColor).toBe(`${ACCENT}80`)
    })

    it('returns soft accentColor+30 for NONE highlight', () => {
      const { borderColor } = getNodeHighlightStyles({
        ...defaults,
        selected: false,
        accentColor: ACCENT,
        highlightMode: HighlightMode.NONE,
        themeMode: 'dark',
      })
      expect(borderColor).toBe(`${ACCENT}30`)
    })

    it('selected takes precedence over highlightMode', () => {
      const { borderColor } = getNodeHighlightStyles({
        ...defaults,
        selected: true,
        accentColor: ACCENT,
        highlightMode: HighlightMode.SELECT,
        themeMode: 'dark',
      })
      expect(borderColor).toBe(ACCENT)
    })
  })

  describe('dark mode — step variant (default)', () => {
    it('uses accent glow ring when selected', () => {
      const { boxShadow } = getNodeHighlightStyles({
        ...defaults,
        selected: true,
        accentColor: ACCENT,
        highlightMode: HighlightMode.NONE,
        themeMode: 'dark',
      })
      expect(boxShadow).toBe(`0 0 0 2px ${ACCENT}, 0 0 20px ${ACCENT}40, 0 8px 32px ${ACCENT}30`)
    })

    it('uses SELECT shadow for SELECT highlight', () => {
      const { boxShadow } = getNodeHighlightStyles({
        ...defaults,
        selected: false,
        accentColor: ACCENT,
        highlightMode: HighlightMode.SELECT,
        themeMode: 'dark',
      })
      expect(boxShadow).toBe(`0 0 0 1px ${ACCENT}40, 0 8px 32px ${ACCENT}22`)
    })

    it('uses HOVER shadow for HOVER highlight', () => {
      const { boxShadow } = getNodeHighlightStyles({
        ...defaults,
        selected: false,
        accentColor: ACCENT,
        highlightMode: HighlightMode.HOVER,
        themeMode: 'dark',
      })
      expect(boxShadow).toBe(`0 0 0 1px ${ACCENT}20, 0 6px 24px ${ACCENT}14`)
    })

    it('uses simple dark default shadow for NONE', () => {
      const { boxShadow } = getNodeHighlightStyles({
        ...defaults,
        selected: false,
        accentColor: ACCENT,
        highlightMode: HighlightMode.NONE,
        themeMode: 'dark',
      })
      expect(boxShadow).toBe('0 4px 24px rgba(0, 0, 0, 0.4)')
    })
  })

  describe('dark mode — resizable variant', () => {
    it('uses accent glow ring when selected', () => {
      const { boxShadow } = getNodeHighlightStyles({
        ...defaults,
        selected: true,
        accentColor: ACCENT,
        highlightMode: HighlightMode.NONE,
        themeMode: 'dark',
        variant: 'resizable',
      })
      expect(boxShadow).toBe(`0 0 0 2px ${ACCENT}, 0 0 20px ${ACCENT}40, 0 8px 32px ${ACCENT}30`)
    })

    it('uses heavier dark default shadow', () => {
      const { boxShadow } = getNodeHighlightStyles({
        ...defaults,
        selected: false,
        accentColor: ACCENT,
        highlightMode: HighlightMode.NONE,
        themeMode: 'dark',
        variant: 'resizable',
      })
      expect(boxShadow).toBe('0 8px 32px rgba(0, 0, 0, 0.5), 0 2px 8px rgba(0, 0, 0, 0.3)')
    })
  })

  describe('light mode — flat design', () => {
    it('returns accentColor border + accent ring when selected', () => {
      const result = getNodeHighlightStyles({
        ...defaults,
        selected: true,
        accentColor: ACCENT,
        highlightMode: HighlightMode.NONE,
        themeMode: 'light',
      })
      expect(result.borderColor).toBe(ACCENT)
      expect(result.boxShadow).toBe(`0 0 0 2px ${ACCENT_RING}`)
    })

    it('returns accentColor+60 border for SELECT highlight, no shadow', () => {
      const result = getNodeHighlightStyles({
        ...defaults,
        selected: false,
        accentColor: ACCENT,
        highlightMode: HighlightMode.SELECT,
        themeMode: 'light',
      })
      expect(result.borderColor).toBe(`${ACCENT}60`)
      expect(result.boxShadow).toBe('none')
    })

    it('returns accentColor+40 border for HOVER highlight, no shadow', () => {
      const result = getNodeHighlightStyles({
        ...defaults,
        selected: false,
        accentColor: ACCENT,
        highlightMode: HighlightMode.HOVER,
        themeMode: 'light',
      })
      expect(result.borderColor).toBe(`${ACCENT}40`)
      expect(result.boxShadow).toBe('none')
    })

    it('returns screenBorder color and no shadow for NONE', () => {
      const result = getNodeHighlightStyles({
        ...defaults,
        selected: false,
        accentColor: ACCENT,
        highlightMode: HighlightMode.NONE,
        themeMode: 'light',
      })
      expect(result.borderColor).toBe(SCREEN_BORDER)
      expect(result.boxShadow).toBe('none')
    })

    it('ignores variant in light mode — both step and resizable have same output', () => {
      const step = getNodeHighlightStyles({
        ...defaults,
        selected: false,
        accentColor: ACCENT,
        highlightMode: HighlightMode.NONE,
        themeMode: 'light',
        variant: 'step',
      })
      const resizable = getNodeHighlightStyles({
        ...defaults,
        selected: false,
        accentColor: ACCENT,
        highlightMode: HighlightMode.NONE,
        themeMode: 'light',
        variant: 'resizable',
      })
      expect(step).toEqual(resizable)
    })
  })
})
