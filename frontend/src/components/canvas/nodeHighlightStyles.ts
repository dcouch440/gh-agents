import { HighlightMode } from './canvasKinds'

/**
 * 'step' — StepNode: simpler selected/default shadows (no accent glow ring)
 * 'resizable' — ContextNode, DocumentNode: accent-glow ring on selected, heavier default shadow
 */
type NodeHighlightVariant = 'step' | 'resizable'

type NodeHighlightInput = {
  selected: boolean
  hasProtocol: boolean
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
  hasProtocol: boolean,
  accentColor: string,
  highlightMode: HighlightMode,
): string => {
  if (selected) return hasProtocol ? accentColor : 'primary.main'
  if (!hasProtocol) return 'divider'
  if (highlightMode === HighlightMode.SELECT) return accentColor
  if (highlightMode === HighlightMode.HOVER) return `${accentColor}80`
  return `${accentColor}50`
}

const getStepBoxShadow = (
  selected: boolean,
  accentColor: string,
  highlightMode: HighlightMode,
  isDark: boolean,
): string => {
  if (selected) {
    return isDark
      ? '0 8px 32px rgba(59, 130, 246, 0.15)'
      : '0 8px 32px rgba(255, 150, 79, 0.16)'
  }
  if (highlightMode === HighlightMode.SELECT) {
    return `0 0 0 1px ${accentColor}40, 0 8px 32px ${accentColor}22`
  }
  if (highlightMode === HighlightMode.HOVER) {
    return `0 0 0 1px ${accentColor}20, 0 6px 24px ${accentColor}14`
  }
  return isDark
    ? '0 4px 24px rgba(0, 0, 0, 0.4)'
    : '0 4px 24px rgba(45, 27, 14, 0.12)'
}

const getResizableBoxShadow = (
  selected: boolean,
  accentColor: string,
  highlightMode: HighlightMode,
  isDark: boolean,
): string => {
  if (selected) {
    return isDark
      ? `0 0 0 1px ${accentColor}40, 0 8px 32px ${accentColor}22, 0 2px 8px rgba(0, 0, 0, 0.3)`
      : `0 0 0 1px ${accentColor}30, 0 12px 40px rgba(45, 27, 14, 0.18), 0 4px 12px ${accentColor}18`
  }
  if (highlightMode === HighlightMode.SELECT) {
    return `0 0 0 1px ${accentColor}40, 0 8px 32px ${accentColor}22`
  }
  if (highlightMode === HighlightMode.HOVER) {
    return `0 0 0 1px ${accentColor}20, 0 6px 24px ${accentColor}14`
  }
  return isDark
    ? '0 8px 32px rgba(0, 0, 0, 0.5), 0 2px 8px rgba(0, 0, 0, 0.3)'
    : '0 8px 32px rgba(45, 27, 14, 0.14), 0 2px 8px rgba(45, 27, 14, 0.08)'
}

const getNodeHighlightStyles = ({
  selected,
  hasProtocol,
  accentColor,
  highlightMode,
  themeMode,
  variant = 'step',
}: NodeHighlightInput): NodeHighlightOutput => {
  const isDark = themeMode === 'dark'
  const borderColor = getNodeBorderColor(selected, hasProtocol, accentColor, highlightMode)
  const boxShadow = variant === 'step'
    ? getStepBoxShadow(selected, accentColor, highlightMode, isDark)
    : getResizableBoxShadow(selected, accentColor, highlightMode, isDark)

  return { borderColor, boxShadow }
}

export { getNodeHighlightStyles }
export type { NodeHighlightInput, NodeHighlightOutput, NodeHighlightVariant }
