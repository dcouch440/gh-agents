import { describe, it, expect } from 'vitest'
import { getNodeHighlightStyles } from './nodeHighlightStyles'
import { HighlightMode } from './canvasKinds'

const ACCENT = '#E57373'

describe('getNodeHighlightStyles', () => {
  describe('borderColor', () => {
    it('returns accentColor when selected', () => {
      const { borderColor } = getNodeHighlightStyles({
        selected: true,
        accentColor: ACCENT,
        highlightMode: HighlightMode.NONE,
        themeMode: 'dark',
      })
      expect(borderColor).toBe(ACCENT)
    })

    it('returns accentColor for SELECT highlight', () => {
      const { borderColor } = getNodeHighlightStyles({
        selected: false,
        accentColor: ACCENT,
        highlightMode: HighlightMode.SELECT,
        themeMode: 'dark',
      })
      expect(borderColor).toBe(ACCENT)
    })

    it('returns accentColor+80 for HOVER highlight', () => {
      const { borderColor } = getNodeHighlightStyles({
        selected: false,
        accentColor: ACCENT,
        highlightMode: HighlightMode.HOVER,
        themeMode: 'dark',
      })
      expect(borderColor).toBe(`${ACCENT}80`)
    })

    it('returns soft accentColor+30 for NONE highlight', () => {
      const { borderColor } = getNodeHighlightStyles({
        selected: false,
        accentColor: ACCENT,
        highlightMode: HighlightMode.NONE,
        themeMode: 'dark',
      })
      expect(borderColor).toBe(`${ACCENT}30`)
    })

    it('selected takes precedence over highlightMode', () => {
      const { borderColor } = getNodeHighlightStyles({
        selected: true,
        accentColor: ACCENT,
        highlightMode: HighlightMode.SELECT,
        themeMode: 'dark',
      })
      expect(borderColor).toBe(ACCENT)
    })
  })

  describe('step variant (default)', () => {
    it('uses accent glow ring when selected in dark mode', () => {
      const { boxShadow } = getNodeHighlightStyles({
        selected: true,
        accentColor: ACCENT,
        highlightMode: HighlightMode.NONE,
        themeMode: 'dark',
      })
      expect(boxShadow).toBe(`0 0 0 2px ${ACCENT}, 0 0 20px ${ACCENT}40, 0 8px 32px ${ACCENT}30`)
    })

    it('uses accent glow ring when selected in light mode', () => {
      const { boxShadow } = getNodeHighlightStyles({
        selected: true,
        accentColor: ACCENT,
        highlightMode: HighlightMode.NONE,
        themeMode: 'light',
      })
      expect(boxShadow).toBe(`0 0 0 2px ${ACCENT}, 0 0 16px ${ACCENT}30, 0 8px 32px rgba(45, 27, 14, 0.14)`)
    })

    it('uses SELECT shadow for SELECT highlight', () => {
      const { boxShadow } = getNodeHighlightStyles({
        selected: false,
        accentColor: ACCENT,
        highlightMode: HighlightMode.SELECT,
        themeMode: 'dark',
      })
      expect(boxShadow).toBe(`0 0 0 1px ${ACCENT}40, 0 8px 32px ${ACCENT}22`)
    })

    it('uses HOVER shadow for HOVER highlight', () => {
      const { boxShadow } = getNodeHighlightStyles({
        selected: false,
        accentColor: ACCENT,
        highlightMode: HighlightMode.HOVER,
        themeMode: 'dark',
      })
      expect(boxShadow).toBe(`0 0 0 1px ${ACCENT}20, 0 6px 24px ${ACCENT}14`)
    })

    it('uses simple dark default shadow for NONE', () => {
      const { boxShadow } = getNodeHighlightStyles({
        selected: false,
        accentColor: ACCENT,
        highlightMode: HighlightMode.NONE,
        themeMode: 'dark',
      })
      expect(boxShadow).toBe('0 4px 24px rgba(0, 0, 0, 0.4)')
    })

    it('uses simple light default shadow for NONE', () => {
      const { boxShadow } = getNodeHighlightStyles({
        selected: false,
        accentColor: ACCENT,
        highlightMode: HighlightMode.NONE,
        themeMode: 'light',
      })
      expect(boxShadow).toBe('0 4px 24px rgba(45, 27, 14, 0.12)')
    })
  })

  describe('resizable variant', () => {
    it('uses accent glow ring when selected in dark mode', () => {
      const { boxShadow } = getNodeHighlightStyles({
        selected: true,
        accentColor: ACCENT,
        highlightMode: HighlightMode.NONE,
        themeMode: 'dark',
        variant: 'resizable',
      })
      expect(boxShadow).toBe(`0 0 0 2px ${ACCENT}, 0 0 20px ${ACCENT}40, 0 8px 32px ${ACCENT}30`)
    })

    it('uses accent glow ring when selected in light mode', () => {
      const { boxShadow } = getNodeHighlightStyles({
        selected: true,
        accentColor: ACCENT,
        highlightMode: HighlightMode.NONE,
        themeMode: 'light',
        variant: 'resizable',
      })
      expect(boxShadow).toBe(`0 0 0 2px ${ACCENT}, 0 0 16px ${ACCENT}30, 0 8px 32px rgba(45, 27, 14, 0.14)`)
    })

    it('uses heavier dark default shadow', () => {
      const { boxShadow } = getNodeHighlightStyles({
        selected: false,
        accentColor: ACCENT,
        highlightMode: HighlightMode.NONE,
        themeMode: 'dark',
        variant: 'resizable',
      })
      expect(boxShadow).toBe('0 8px 32px rgba(0, 0, 0, 0.5), 0 2px 8px rgba(0, 0, 0, 0.3)')
    })

    it('uses heavier light default shadow', () => {
      const { boxShadow } = getNodeHighlightStyles({
        selected: false,
        accentColor: ACCENT,
        highlightMode: HighlightMode.NONE,
        themeMode: 'light',
        variant: 'resizable',
      })
      expect(boxShadow).toBe('0 8px 32px rgba(45, 27, 14, 0.14), 0 2px 8px rgba(45, 27, 14, 0.08)')
    })

    it('shares selected shadow with step variant', () => {
      const step = getNodeHighlightStyles({
        selected: true,
        accentColor: ACCENT,
        highlightMode: HighlightMode.NONE,
        themeMode: 'dark',
        variant: 'step',
      })
      const resizable = getNodeHighlightStyles({
        selected: true,
        accentColor: ACCENT,
        highlightMode: HighlightMode.NONE,
        themeMode: 'dark',
        variant: 'resizable',
      })
      expect(step.boxShadow).toBe(resizable.boxShadow)
    })

    it('shares SELECT shadow with step variant', () => {
      const step = getNodeHighlightStyles({
        selected: false,
        accentColor: ACCENT,
        highlightMode: HighlightMode.SELECT,
        themeMode: 'dark',
        variant: 'step',
      })
      const resizable = getNodeHighlightStyles({
        selected: false,
        accentColor: ACCENT,
        highlightMode: HighlightMode.SELECT,
        themeMode: 'dark',
        variant: 'resizable',
      })
      expect(step.boxShadow).toBe(resizable.boxShadow)
    })

    it('shares HOVER shadow with step variant', () => {
      const step = getNodeHighlightStyles({
        selected: false,
        accentColor: ACCENT,
        highlightMode: HighlightMode.HOVER,
        themeMode: 'dark',
        variant: 'step',
      })
      const resizable = getNodeHighlightStyles({
        selected: false,
        accentColor: ACCENT,
        highlightMode: HighlightMode.HOVER,
        themeMode: 'dark',
        variant: 'resizable',
      })
      expect(step.boxShadow).toBe(resizable.boxShadow)
    })
  })
})
