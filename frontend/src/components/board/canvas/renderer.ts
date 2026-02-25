// ============================================================================
// Canvas Renderer — Pure Drawing Functions
// ============================================================================
//
// All functions are stateless, pure, and accept CanvasRenderingContext2D.
// No React, no hooks, no side effects. The render pipeline calls these
// in order to paint the board onto a single <canvas> element.
//
// Box borders use rough.js for a hand-drawn/sketchy aesthetic.

import rough from 'roughjs'
import type { RoughCanvas } from 'roughjs/bin/canvas'
import { computeArrowPathPoints, computeDrawingArrowPathPoints } from '../arrows/routing'
import type { ArrowPath } from '../arrows/routing'
import { BOARD } from '../constants'
import type { BoardElements, BoxElement, DrawingArrow, MarqueeRect, SelectionState, ViewportState } from '../elements'
import type { EdgeHover } from '../elements'
import { wrapText } from './textMeasure'

type DrawTheme = {
  readonly canvasBg: string
  readonly gridDotColor: string
  readonly connectorColor: string
  readonly strokeColor: string
  readonly accentColor: string
  readonly surfaceBg: string
  readonly textColor: string
}

// ── Grid ──────────────────────────────────────────────────────────────────

/**
 * Draw dot grid, culling off-screen dots for performance.
 */
const drawGrid = (
  ctx: CanvasRenderingContext2D,
  viewport: ViewportState,
  canvasWidth: number,
  canvasHeight: number,
  theme: DrawTheme,
): void => {
  const { zoom, panX, panY } = viewport
  const gridSize = BOARD.GRID_SIZE

  // Compute visible canvas-space bounds
  const left = -panX / zoom
  const top = -panY / zoom
  const right = left + canvasWidth / zoom
  const bottom = top + canvasHeight / zoom

  // Snap to grid boundaries
  const startX = Math.floor(left / gridSize) * gridSize
  const startY = Math.floor(top / gridSize) * gridSize
  const endX = Math.ceil(right / gridSize) * gridSize
  const endY = Math.ceil(bottom / gridSize) * gridSize

  ctx.fillStyle = theme.gridDotColor
  const dotRadius = 1

  for (let x = startX; x <= endX; x += gridSize) {
    for (let y = startY; y <= endY; y += gridSize) {
      ctx.fillRect(x - dotRadius, y - dotRadius, dotRadius * 2, dotRadius * 2)
    }
  }
}

// ── Box ───────────────────────────────────────────────────────────────────

/**
 * Draw a box with rough.js hand-drawn border, background fill, and text.
 * Skips text rendering when isEditing (textarea overlay handles it).
 *
 * Uses a stable seed per box so the sketchy lines don't jitter on re-render.
 */
const drawBox = (
  ctx: CanvasRenderingContext2D,
  rc: RoughCanvas,
  box: BoxElement,
  isSelected: boolean,
  isEditing: boolean,
  theme: DrawTheme,
): void => {
  const { x, y, width, height } = box

  // Stable seed from box id — keeps sketchy lines consistent across re-renders
  const seed = hashStringToSeed(box.id)

  // Rough.js rounded rectangle via SVG path for hand-drawn rounded corners
  const d = roundedRectPath(x, y, width, height, BOARD.BOX_BORDER_RADIUS)
  rc.path(d, {
    fill: theme.surfaceBg,
    fillStyle: 'solid',
    stroke: isSelected ? theme.accentColor : theme.strokeColor,
    strokeWidth: BOARD.BOX_BORDER_WIDTH,
    roughness: 1.0,
    bowing: 1.5,
    seed,
  })

  // Text (skip when editing — textarea overlay is visible)
  if (!isEditing && box.text.length > 0) {
    const maxTextWidth = width - BOARD.BOX_PADDING_X * 2
    const lineHeight = BOARD.FONT_SIZE * BOARD.LINE_HEIGHT

    ctx.save()
    ctx.font = `${BOARD.FONT_SIZE}px ${BOARD.FONT_FAMILY}`
    ctx.fillStyle = theme.textColor
    ctx.textBaseline = 'top'

    const lines = wrapText(ctx, box.text, maxTextWidth, lineHeight)
    const textX = x + BOARD.BOX_PADDING_X
    const textY = y + BOARD.BOX_PADDING_Y

    for (let i = 0; i < lines.length; i++) {
      ctx.fillText(lines[i]!.text, textX, textY + lines[i]!.y)
    }

    ctx.restore()
  }
}

/**
 * Draw a highlight glow around a box when hovering for arrow binding.
 * Draws a translucent accent border + subtle fill to clearly signal connection.
 */
const drawBoxHighlight = (
  ctx: CanvasRenderingContext2D,
  box: BoxElement,
  accentColor: string,
): void => {
  const pad = 4
  const { x, y, width, height } = box

  // Glow fill
  ctx.save()
  ctx.fillStyle = accentColor + '15' // ~8% alpha
  ctx.beginPath()
  ctx.roundRect(x - pad, y - pad, width + pad * 2, height + pad * 2, BOARD.BOX_BORDER_RADIUS + pad)
  ctx.fill()

  // Glow border
  ctx.strokeStyle = accentColor + '80' // 50% alpha
  ctx.lineWidth = 2.5
  ctx.beginPath()
  ctx.roundRect(x - pad, y - pad, width + pad * 2, height + pad * 2, BOARD.BOX_BORDER_RADIUS + pad)
  ctx.stroke()
  ctx.restore()
}

/**
 * Deterministic seed from string — so each box gets consistent sketchy lines.
 */
const hashStringToSeed = (s: string): number => {
  let hash = 0
  for (let i = 0; i < s.length; i++) {
    hash = ((hash << 5) - hash + s.charCodeAt(i)) | 0
  }
  return Math.abs(hash)
}

/**
 * SVG path string for a rounded rectangle.
 * Uses quadratic bezier (Q) for corners, producing smooth arcs
 * that rough.js will render with its hand-drawn effect.
 */
const roundedRectPath = (x: number, y: number, w: number, h: number, r: number): string => {
  // Clamp radius to half the smallest dimension
  const cr = Math.min(r, w / 2, h / 2)
  return [
    `M ${x + cr} ${y}`,
    `L ${x + w - cr} ${y}`,
    `Q ${x + w} ${y} ${x + w} ${y + cr}`,
    `L ${x + w} ${y + h - cr}`,
    `Q ${x + w} ${y + h} ${x + w - cr} ${y + h}`,
    `L ${x + cr} ${y + h}`,
    `Q ${x} ${y + h} ${x} ${y + h - cr}`,
    `L ${x} ${y + cr}`,
    `Q ${x} ${y} ${x + cr} ${y}`,
    'Z',
  ].join(' ')
}

// ── Arrow ─────────────────────────────────────────────────────────────────

/**
 * Draw a hand-drawn cubic bezier arrow with arrowhead using rough.js.
 */
const drawArrow = (
  rc: RoughCanvas,
  path: ArrowPath,
  arrowId: string,
  isSelected: boolean,
  theme: DrawTheme,
): void => {
  const color = isSelected ? theme.accentColor : theme.strokeColor
  const seed = hashStringToSeed(arrowId)

  // Rough.js curve through the 4 bezier points
  rc.curve(
    [[path.start.x, path.start.y], [path.cp1.x, path.cp1.y], [path.cp2.x, path.cp2.y], [path.end.x, path.end.y]],
    {
      stroke: color,
      strokeWidth: BOARD.ARROW_STROKE_WIDTH,
      roughness: 0.8,
      bowing: 1.0,
      seed,
    },
  )

  // Compute arrowhead angle from curve tangent at endpoint
  const angle = Math.atan2(path.end.y - path.cp2.y, path.end.x - path.cp2.x)
  drawArrowhead(rc, path.end, angle, BOARD.ARROW_HEAD_SIZE, color, seed)
}

/**
 * Draw a dashed arrow preview while the user is drawing.
 * Kept smooth (no rough.js) — the preview moves every frame and rough
 * jitter on each pixel of cursor movement looks bad.
 */
const drawDrawingArrow = (
  ctx: CanvasRenderingContext2D,
  path: ArrowPath,
  color: string,
): void => {
  ctx.save()
  ctx.strokeStyle = color
  ctx.lineWidth = BOARD.ARROW_STROKE_WIDTH
  ctx.lineCap = 'round'
  ctx.lineJoin = 'round'
  ctx.setLineDash([6, 4])

  ctx.beginPath()
  ctx.moveTo(path.start.x, path.start.y)
  ctx.bezierCurveTo(path.cp1.x, path.cp1.y, path.cp2.x, path.cp2.y, path.end.x, path.end.y)
  ctx.stroke()

  ctx.setLineDash([])

  // Smooth arrowhead for preview
  const angle = Math.atan2(path.end.y - path.cp2.y, path.end.x - path.cp2.x)
  const spread = Math.PI / 6
  ctx.beginPath()
  ctx.moveTo(
    path.end.x - BOARD.ARROW_HEAD_SIZE * Math.cos(angle - spread),
    path.end.y - BOARD.ARROW_HEAD_SIZE * Math.sin(angle - spread),
  )
  ctx.lineTo(path.end.x, path.end.y)
  ctx.lineTo(
    path.end.x - BOARD.ARROW_HEAD_SIZE * Math.cos(angle + spread),
    path.end.y - BOARD.ARROW_HEAD_SIZE * Math.sin(angle + spread),
  )
  ctx.stroke()

  ctx.restore()
}

/**
 * Draw a hand-drawn arrowhead (two rough lines from tip going backward at ±30°).
 */
const drawArrowhead = (
  rc: RoughCanvas,
  tip: { x: number; y: number },
  angle: number,
  size: number,
  color: string,
  seed: number,
): void => {
  const spread = Math.PI / 6 // 30 degrees
  const opts = { stroke: color, strokeWidth: BOARD.ARROW_STROKE_WIDTH, roughness: 0.6, seed }

  // Left barb
  rc.line(
    tip.x - size * Math.cos(angle - spread),
    tip.y - size * Math.sin(angle - spread),
    tip.x,
    tip.y,
    opts,
  )

  // Right barb
  rc.line(
    tip.x - size * Math.cos(angle + spread),
    tip.y - size * Math.sin(angle + spread),
    tip.x,
    tip.y,
    opts,
  )
}

// ── Handles ───────────────────────────────────────────────────────────────

/**
 * Draw a handle circle (for edge hover arrow binding).
 */
const drawHandle = (
  ctx: CanvasRenderingContext2D,
  cx: number,
  cy: number,
  theme: DrawTheme,
): void => {
  const radius = BOARD.HANDLE_SIZE / 2

  ctx.beginPath()
  ctx.arc(cx, cy, radius, 0, Math.PI * 2)
  ctx.fillStyle = theme.accentColor
  ctx.fill()
  ctx.strokeStyle = theme.surfaceBg
  ctx.lineWidth = 2
  ctx.stroke()
}

// ── Selection ─────────────────────────────────────────────────────────────

/**
 * Draw a selection marquee rectangle (dashed border + translucent fill).
 */
const drawSelectionRect = (
  ctx: CanvasRenderingContext2D,
  rect: MarqueeRect,
  accentColor: string,
): void => {
  ctx.save()

  ctx.fillStyle = accentColor + '10' // ~6% alpha
  ctx.fillRect(rect.x, rect.y, rect.width, rect.height)

  ctx.strokeStyle = accentColor
  ctx.lineWidth = 1
  ctx.setLineDash([4, 4])
  ctx.strokeRect(rect.x, rect.y, rect.width, rect.height)

  ctx.setLineDash([])
  ctx.restore()
}

// ── Render Pipeline ──────────────────────────────────────────────────────

/**
 * Full board render — clears the canvas and draws all layers in order.
 * Called from Canvas2D's render effect on every frame.
 */
const renderBoard = (
  canvas: HTMLCanvasElement,
  canvasWidth: number,
  canvasHeight: number,
  elements: BoardElements,
  selection: SelectionState,
  editingBoxId: string | null,
  viewport: ViewportState,
  drawingArrow: DrawingArrow,
  edgeHover: EdgeHover | null,
  theme: DrawTheme,
): void => {
  const ctx = canvas.getContext('2d')
  if (ctx === null) return

  const dpr = window.devicePixelRatio || 1
  const rc = rough.canvas(canvas)

  // Clear
  ctx.setTransform(dpr, 0, 0, dpr, 0, 0)
  ctx.clearRect(0, 0, canvasWidth, canvasHeight)

  // Viewport transform
  ctx.save()
  ctx.translate(viewport.panX, viewport.panY)
  ctx.scale(viewport.zoom, viewport.zoom)

  // Grid
  drawGrid(ctx, viewport, canvasWidth, canvasHeight, theme)

  // Box highlight for edge hover (draw under boxes so the glow is behind)
  if (edgeHover !== null && editingBoxId === null) {
    const hoverBox = elements.boxes.get(edgeHover.boxId)
    if (hoverBox !== undefined) {
      drawBoxHighlight(ctx, hoverBox, theme.accentColor)
    }
  }

  // Boxes in z-order
  for (let i = 0; i < elements.boxOrder.length; i++) {
    const boxId = elements.boxOrder[i]!
    const box = elements.boxes.get(boxId)
    if (box === undefined) continue

    const isSelected = selection.selectedIds.has(boxId)
    const isEditing = editingBoxId === boxId
    drawBox(ctx, rc, box, isSelected, isEditing, theme)
  }

  // Arrows
  for (const [arrowId, arrow] of elements.arrows) {
    const sourceBox = elements.boxes.get(arrow.sourceBoxId)
    const targetBox = elements.boxes.get(arrow.targetBoxId)
    if (sourceBox === undefined || targetBox === undefined) continue

    const path = computeArrowPathPoints(sourceBox, arrow.sourceAnchor, targetBox, arrow.targetAnchor)
    const isSelected = selection.selectedIds.has(arrowId)
    drawArrow(rc, path, arrowId, isSelected, theme)
  }

  // Drawing arrow preview
  if (drawingArrow !== null) {
    const sourceBox = elements.boxes.get(drawingArrow.sourceBoxId)
    if (sourceBox !== undefined) {
      const path = computeDrawingArrowPathPoints(
        sourceBox,
        drawingArrow.sourceAnchor,
        drawingArrow.cursorX,
        drawingArrow.cursorY,
      )
      drawDrawingArrow(ctx, path, theme.accentColor)
    }
  }

  // Edge hover handle
  if (edgeHover !== null && editingBoxId === null) {
    drawHandle(ctx, edgeHover.cx, edgeHover.cy, theme)
  }

  // Selection marquee
  if (selection.marquee !== null) {
    drawSelectionRect(ctx, selection.marquee, theme.accentColor)
  }

  ctx.restore()
}

export {
  drawArrow,
  drawBox,
  drawBoxHighlight,
  drawDrawingArrow,
  drawGrid,
  drawHandle,
  drawSelectionRect,
  renderBoard,
}
export type { DrawTheme }
