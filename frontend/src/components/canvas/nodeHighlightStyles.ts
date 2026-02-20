import { HighlightMode } from './canvasKinds'

type NodeHighlightInput = {
  selected: boolean
  accentColor: string
  highlightMode: HighlightMode
  /** Default border color (from theme token). */
  screenBorder: string
  /** Accent ring opacity for selected state (from theme token). */
  accentRing: string
}

type NodeHighlightOutput = {
  borderColor: string
  boxShadow: string
}

/* ── Flat design — identical for light & dark modes ── */

const getNodeHighlightStyles = ({
  selected,
  accentColor,
  highlightMode,
  screenBorder,
  accentRing,
}: NodeHighlightInput): NodeHighlightOutput => {
  if (selected) {
    return {
      borderColor: accentColor,
      boxShadow: `0 0 0 2px ${accentRing}`,
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
    borderColor: screenBorder,
    boxShadow: 'none',
  }
}

export { getNodeHighlightStyles }
export type { NodeHighlightInput, NodeHighlightOutput }
