// ============================================================================
// Bounds — Bounding Box and Anchor Point Computation
// ============================================================================

import { Geometry } from '@/utils/geometry'
import type { Point } from '@/utils/geometry'
import { BOARD } from '../constants'
import type { AnchorPoint, BoxElement } from './types'

/**
 * Resolve an anchor point on a box to absolute canvas coordinates.
 *
 * Uses the box's bounding rect and the anchor's side + ratio to compute
 * the exact point where an arrow should connect.
 */
const resolveAnchor = (box: BoxElement, anchor: AnchorPoint): Point =>
  Geometry.pointAlongSide(box, anchor.side, anchor.ratio)

/**
 * Compute auto-sized box dimensions from text content.
 *
 * Uses a hidden DOM element (created once, reused) to measure text with the
 * same font metrics as EditableBox. Returns the outer box dimensions
 * including padding.
 *
 * In test environments where DOM measurement returns 0, falls back to
 * character-count estimation.
 */
const computeBoxSize = (text: string): { width: number; height: number } => {
  if (text.length === 0) {
    return { width: BOARD.DEFAULT_BOX_WIDTH, height: BOARD.DEFAULT_BOX_HEIGHT }
  }

  const measurer = getMeasurer()
  measurer.textContent = text

  const contentWidth = measurer.scrollWidth
  const contentHeight = measurer.scrollHeight

  // Fallback for test environments (jsdom returns 0)
  if (contentWidth === 0 || contentHeight === 0) {
    return estimateBoxSize(text)
  }

  return {
    width: Math.max(BOARD.MIN_BOX_WIDTH, Math.min(contentWidth + BOARD.BOX_PADDING_X * 2, BOARD.MAX_BOX_WIDTH)),
    height: Math.max(BOARD.MIN_BOX_HEIGHT, contentHeight + BOARD.BOX_PADDING_Y * 2),
  }
}

/**
 * Character-count estimation for environments without DOM measurement.
 */
const estimateBoxSize = (text: string): { width: number; height: number } => {
  const charWidth = BOARD.FONT_SIZE * 0.6
  const lineHeight = BOARD.FONT_SIZE * BOARD.LINE_HEIGHT
  const maxContentWidth = BOARD.MAX_BOX_WIDTH - BOARD.BOX_PADDING_X * 2
  const lines = text.split('\n')

  let maxLineWidth = 0
  let totalLines = 0

  for (let i = 0; i < lines.length; i++) {
    const lineWidth = lines[i]!.length * charWidth
    if (lineWidth > maxContentWidth) {
      totalLines += Math.ceil(lineWidth / maxContentWidth)
    } else {
      totalLines += 1
    }
    if (lineWidth > maxLineWidth) maxLineWidth = lineWidth
  }

  const contentWidth = Math.min(maxLineWidth, maxContentWidth)
  const contentHeight = totalLines * lineHeight

  return {
    width: Math.max(BOARD.MIN_BOX_WIDTH, contentWidth + BOARD.BOX_PADDING_X * 2),
    height: Math.max(BOARD.MIN_BOX_HEIGHT, contentHeight + BOARD.BOX_PADDING_Y * 2),
  }
}

// ── Hidden measurer singleton ──────────────────────────────────────────────

let _measurer: HTMLDivElement | null = null

const getMeasurer = (): HTMLDivElement => {
  if (_measurer !== null) return _measurer

  const el = document.createElement('div')
  el.style.position = 'absolute'
  el.style.visibility = 'hidden'
  el.style.height = 'auto'
  el.style.width = 'auto'
  el.style.maxWidth = `${BOARD.MAX_BOX_WIDTH - BOARD.BOX_PADDING_X * 2}px`
  el.style.whiteSpace = 'pre-wrap'
  el.style.wordBreak = 'break-word'
  el.style.fontFamily = BOARD.FONT_FAMILY
  el.style.fontSize = `${BOARD.FONT_SIZE}px`
  el.style.lineHeight = `${BOARD.LINE_HEIGHT}`
  el.style.padding = '0'
  el.style.border = 'none'
  el.style.pointerEvents = 'none'
  document.body.appendChild(el)
  _measurer = el
  return el
}

export { computeBoxSize, estimateBoxSize, resolveAnchor }
