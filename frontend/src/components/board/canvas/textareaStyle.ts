// ============================================================================
// Textarea Style — Position a textarea overlay on the canvas
// ============================================================================

import { BOARD } from '../constants'
import type { BoxElement, ViewportState } from '../elements'

/**
 * Compute the CSS style for the textarea overlay that appears when editing a box.
 * Positions the textarea directly over the box's text area in screen coordinates.
 */
const computeTextareaStyle = (
  box: BoxElement,
  viewport: ViewportState,
  textColor: string,
): React.CSSProperties => ({
  position: 'absolute',
  left: box.x * viewport.zoom + viewport.panX + BOARD.BOX_PADDING_X * viewport.zoom,
  top: box.y * viewport.zoom + viewport.panY + BOARD.BOX_PADDING_Y * viewport.zoom,
  width: (box.width - BOARD.BOX_PADDING_X * 2) * viewport.zoom,
  minHeight: (box.height - BOARD.BOX_PADDING_Y * 2) * viewport.zoom,
  fontFamily: BOARD.FONT_FAMILY,
  fontSize: BOARD.FONT_SIZE * viewport.zoom,
  lineHeight: BOARD.LINE_HEIGHT,
  color: textColor,
  background: 'transparent',
  border: 'none',
  outline: 'none',
  resize: 'none',
  overflow: 'hidden',
  whiteSpace: 'pre-wrap',
  wordBreak: 'break-word',
  padding: 0,
  margin: 0,
  zIndex: 1,
})

export { computeTextareaStyle }
