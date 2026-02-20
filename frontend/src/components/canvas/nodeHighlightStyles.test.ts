import { describe, it, expect } from 'vitest'
import { getNodeHighlightStyles } from './nodeHighlightStyles'
import { HighlightMode } from './canvasKinds'

const ACCENT = '#E57373'
const SCREEN_BORDER = '#d6cfc4'
const ACCENT_RING = 'rgba(90, 138, 110, 0.18)'

/** Shared defaults for every test call. */
const defaults = { screenBorder: SCREEN_BORDER, accentRing: ACCENT_RING } as const

describe('getNodeHighlightStyles', () => {
  describe('selected state', () => {
    it('returns accentColor border + accent ring when selected', () => {
      const result = getNodeHighlightStyles({
        ...defaults,
        selected: true,
        accentColor: ACCENT,
        highlightMode: HighlightMode.NONE,
      })
      expect(result.borderColor).toBe(ACCENT)
      expect(result.boxShadow).toBe(`0 0 0 2px ${ACCENT_RING}`)
    })

    it('selected takes precedence over highlightMode', () => {
      const result = getNodeHighlightStyles({
        ...defaults,
        selected: true,
        accentColor: ACCENT,
        highlightMode: HighlightMode.SELECT,
      })
      expect(result.borderColor).toBe(ACCENT)
      expect(result.boxShadow).toBe(`0 0 0 2px ${ACCENT_RING}`)
    })
  })

  describe('SELECT highlight', () => {
    it('returns accentColor+60 border, no shadow', () => {
      const result = getNodeHighlightStyles({
        ...defaults,
        selected: false,
        accentColor: ACCENT,
        highlightMode: HighlightMode.SELECT,
      })
      expect(result.borderColor).toBe(`${ACCENT}60`)
      expect(result.boxShadow).toBe('none')
    })
  })

  describe('HOVER highlight', () => {
    it('returns accentColor+40 border, no shadow', () => {
      const result = getNodeHighlightStyles({
        ...defaults,
        selected: false,
        accentColor: ACCENT,
        highlightMode: HighlightMode.HOVER,
      })
      expect(result.borderColor).toBe(`${ACCENT}40`)
      expect(result.boxShadow).toBe('none')
    })
  })

  describe('default state (NONE)', () => {
    it('returns screenBorder color and no shadow', () => {
      const result = getNodeHighlightStyles({
        ...defaults,
        selected: false,
        accentColor: ACCENT,
        highlightMode: HighlightMode.NONE,
      })
      expect(result.borderColor).toBe(SCREEN_BORDER)
      expect(result.boxShadow).toBe('none')
    })
  })
})
