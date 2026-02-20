import { HighlightMode } from './canvasKinds'

/**
 * 'step' — StepNode: simpler selected/default shadows (no accent glow ring)
 * 'resizable' — ContextNode, DocumentNode: accent-glow ring on selected, heavier default shadow
 */
type NodeHighlightVariant = 'step' | 'resizable'

type NodeHighlightInput = {
  selected: boolean
  accentColor: string
  highlightMode: HighlightMode
  themeMode: 'light' | 'dark'
  variant?: NodeHighlightVariant
}

type NodeHighlightOutput = {
  borderColor: string
  boxShadow: string
}

/* ── Dark mode (unchanged) ── */

const getNodeBorderColorDark = (
  selected: boolean,
  accentColor: string,
  highlightMode: HighlightMode,
): string => {
  if (selected) return accentColor
  if (highlightMode === HighlightMode.SELECT) return accentColor
  if (highlightMode === HighlightMode.HOVER) return `${accentColor}80`
  return `${accentColor}30`
}

type ShadowPreset = { dark: string; light: string }

const STEP_DEFAULT_SHADOW: ShadowPreset = {
  dark: '0 4px 24px rgba(0, 0, 0, 0.4)',
  light: 'none',
}

const RESIZABLE_DEFAULT_SHADOW: ShadowPreset = {
  dark: '0 8px 32px rgba(0, 0, 0, 0.5), 0 2px 8px rgba(0, 0, 0, 0.3)',
  light: 'none',
}

const getBoxShadowDark = (
  selected: boolean,
  accentColor: string,
  highlightMode: HighlightMode,
  defaultPreset: ShadowPreset,
): string => {
  if (selected) {
    return `0 0 0 2px ${accentColor}, 0 0 20px ${accentColor}40, 0 8px 32px ${accentColor}30`
  }
  if (highlightMode === HighlightMode.SELECT) {
    return `0 0 0 1px ${accentColor}40, 0 8px 32px ${accentColor}22`
  }
  if (highlightMode === HighlightMode.HOVER) {
    return `0 0 0 1px ${accentColor}20, 0 6px 24px ${accentColor}14`
  }
  return defaultPreset.dark
}

/* ── Light mode (flat design matching mockup) ── */

const SCREEN_BORDER = '#d6cfc4'
const ACCENT_RING = 'rgba(90, 138, 110, 0.18)'

const getNodeHighlightStyles = ({
  selected,
  accentColor,
  highlightMode,
  themeMode,
  variant = 'step',
}: NodeHighlightInput): NodeHighlightOutput => {
  if (themeMode === 'dark') {
    const borderColor = getNodeBorderColorDark(selected, accentColor, highlightMode)
    const preset = variant === 'step' ? STEP_DEFAULT_SHADOW : RESIZABLE_DEFAULT_SHADOW
    const boxShadow = getBoxShadowDark(selected, accentColor, highlightMode, preset)
    return { borderColor, boxShadow }
  }

  // Light mode — flat, minimal
  if (selected) {
    return {
      borderColor: accentColor,
      boxShadow: `0 0 0 2px ${ACCENT_RING}`,
    }
  }
  if (highlightMode === HighlightMode.SELECT) {
    return {
      borderColor: `${accentColor}60`,
      boxShadow: 'none',
    }
  }
  if (highlightMode === HighlightMode.HOVER) {
    return {
      borderColor: `${accentColor}40`,
      boxShadow: 'none',
    }
  }
  return {
    borderColor: SCREEN_BORDER,
    boxShadow: 'none',
  }
}

export { getNodeHighlightStyles }
export type { NodeHighlightOutput, NodeHighlightVariant }
export type { NodeHighlightInput, NodeHighlightOutput, NodeHighlightVariant }
