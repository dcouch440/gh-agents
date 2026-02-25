// ============================================================================
// Text Measurement — Canvas-Based Word-Wrapping and Sizing
// ============================================================================

import { BOARD } from '../constants'

type WrappedLine = {
  readonly text: string
  readonly y: number // offset from top of text area
}

/**
 * Word-wrap text to fit within maxWidth using canvas text metrics.
 *
 * Preserves explicit line breaks. Each visual line includes a pre-computed
 * y offset for rendering with ctx.fillText().
 */
const wrapText = (
  ctx: CanvasRenderingContext2D,
  text: string,
  maxWidth: number,
  lineHeight: number,
): readonly WrappedLine[] => {
  if (text.length === 0) return []

  const lines: WrappedLine[] = []
  const paragraphs = text.split('\n')
  let lineIndex = 0

  for (let p = 0; p < paragraphs.length; p++) {
    const paragraph = paragraphs[p]!

    if (paragraph.length === 0) {
      lines.push({ text: '', y: lineIndex * lineHeight })
      lineIndex++
      continue
    }

    const words = paragraph.split(' ')
    let currentLine = words[0]!

    for (let w = 1; w < words.length; w++) {
      const word = words[w]!
      const testLine = currentLine + ' ' + word
      const metrics = ctx.measureText(testLine)

      if (metrics.width > maxWidth) {
        lines.push({ text: currentLine, y: lineIndex * lineHeight })
        lineIndex++
        currentLine = word
      } else {
        currentLine = testLine
      }
    }

    lines.push({ text: currentLine, y: lineIndex * lineHeight })
    lineIndex++
  }

  return lines
}

/**
 * Measure the bounding dimensions of word-wrapped text.
 *
 * Returns the content area size (without padding). Add BOX_PADDING_X/Y
 * for the full box size.
 */
const measureWrappedText = (
  ctx: CanvasRenderingContext2D,
  text: string,
  maxWidth: number,
): { width: number; height: number } => {
  const lineHeight = BOARD.FONT_SIZE * BOARD.LINE_HEIGHT
  const lines = wrapText(ctx, text, maxWidth, lineHeight)

  if (lines.length === 0) {
    return { width: 0, height: lineHeight }
  }

  let maxLineWidth = 0
  for (let i = 0; i < lines.length; i++) {
    const w = ctx.measureText(lines[i]!.text).width
    if (w > maxLineWidth) maxLineWidth = w
  }

  return {
    width: Math.min(maxLineWidth, maxWidth),
    height: lines.length * lineHeight,
  }
}

export { measureWrappedText, wrapText }
export type { WrappedLine }
