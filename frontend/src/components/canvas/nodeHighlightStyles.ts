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

const getNodeBorderColor = (
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
  light: '0 4px 24px rgba(45, 27, 14, 0.12)',
}

const RESIZABLE_DEFAULT_SHADOW: ShadowPreset = {
  dark: '0 8px 32px rgba(0, 0, 0, 0.5), 0 2px 8px rgba(0, 0, 0, 0.3)',
  light: '0 8px 32px rgba(45, 27, 14, 0.14), 0 2px 8px rgba(45, 27, 14, 0.08)',
}

const getBoxShadow = (
  selected: boolean,
  accentColor: string,
  highlightMode: HighlightMode,
  isDark: boolean,
  defaultPreset: ShadowPreset,
): string => {
  if (selected) {
    return isDark
      ? `0 0 0 2px ${accentColor}, 0 0 20px ${accentColor}40, 0 8px 32px ${accentColor}30`
      : `0 0 0 2px ${accentColor}, 0 0 16px ${accentColor}30, 0 8px 32px rgba(45, 27, 14, 0.14)`
  }
  if (highlightMode === HighlightMode.SELECT) {
    return `0 0 0 1px ${accentColor}40, 0 8px 32px ${accentColor}22`
  }
  if (highlightMode === HighlightMode.HOVER) {
    return `0 0 0 1px ${accentColor}20, 0 6px 24px ${accentColor}14`
  }
  return isDark ? defaultPreset.dark : defaultPreset.light
}

const getNodeHighlightStyles = ({
  selected,
  accentColor,
  highlightMode,
  themeMode,
  variant = 'step',
}: NodeHighlightInput): NodeHighlightOutput => {
  const isDark = themeMode === 'dark'
  const borderColor = getNodeBorderColor(selected, accentColor, highlightMode)
  const preset = variant === 'step' ? STEP_DEFAULT_SHADOW : RESIZABLE_DEFAULT_SHADOW
  const boxShadow = getBoxShadow(selected, accentColor, highlightMode, isDark, preset)

  return { borderColor, boxShadow }
}

export { getNodeHighlightStyles }
export type { NodeHighlightInput, NodeHighlightOutput, NodeHighlightVariant }
