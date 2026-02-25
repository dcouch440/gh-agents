// ============================================================================
// Canvas2D — Canvas-Based Drawing Surface
// ============================================================================
//
// Renders all board elements (grid, boxes, arrows, handles) on a single
// <canvas> element. Text editing uses a temporary <textarea> overlay
// positioned over the box being edited — the same approach as Excalidraw.

import { forwardRef, useCallback, useEffect, useMemo, useRef, useState } from 'react'
import rough from 'roughjs'
import type { Side } from '@/utils/geometry'
import { computeArrowPathPoints, computeDrawingArrowPathPoints } from '../arrows/routing'
import { BOARD } from '../constants'
import type { AnchorPoint, BoardElements, DrawingArrow, ResizeHandle, SelectionState, ViewportState } from '../elements'
import { screenToCanvas } from '../elements'
import {
  drawArrow,
  drawBox,
  drawBoxHighlight,
  drawDrawingArrow,
  drawGrid,
  drawHandle,
  drawSelectionRect,
} from './renderer'
import type { DrawTheme } from './renderer'

// ── Props ─────────────────────────────────────────────────────────────────

type Canvas2DProps = {
  readonly elements: BoardElements
  readonly selection: SelectionState
  readonly editingBoxId: string | null
  readonly viewport: ViewportState
  readonly drawingArrow: DrawingArrow
  readonly canvasBg: string
  readonly gridDotColor: string
  readonly connectorColor: string
  readonly accentColor: string
  readonly surfaceBg: string
  readonly textColor: string
  readonly onPointerDown: (e: React.PointerEvent) => void
  readonly onPointerMove: (e: React.PointerEvent) => void
  readonly onPointerUp: (e: React.PointerEvent) => void
  readonly onWheel: (e: React.WheelEvent) => void
  readonly onDoubleClick: (e: React.MouseEvent) => void
  readonly onBoxTextChange: (boxId: string, text: string, width: number, height: number) => void
  readonly onBoxDoubleClick: (boxId: string) => void
  readonly onBoxBlur: (boxId: string) => void
  readonly onBoxPointerDown: (boxId: string, e: React.PointerEvent) => void
  readonly onAnchorPointerDown: (boxId: string, anchor: AnchorPoint, e: React.PointerEvent) => void
  readonly onResizePointerDown: (boxId: string, handle: ResizeHandle, e: React.PointerEvent) => void
}

// ── Edge Hover Detection ──────────────────────────────────────────────────

type EdgeHover = {
  readonly boxId: string
  readonly side: Side
  readonly ratio: number
  readonly cx: number // canvas coordinates
  readonly cy: number // canvas coordinates
}

const EDGE_HOVER_THRESHOLD = 16

const detectEdgeHover = (
  canvasX: number,
  canvasY: number,
  elements: BoardElements,
): EdgeHover | null => {
  // Check boxes in reverse z-order (frontmost first)
  for (let i = elements.boxOrder.length - 1; i >= 0; i--) {
    const boxId = elements.boxOrder[i]!
    const box = elements.boxes.get(boxId)
    if (box === undefined) continue

    // Check if point is within the box's expanded bounds
    const expandedLeft = box.x - EDGE_HOVER_THRESHOLD
    const expandedTop = box.y - EDGE_HOVER_THRESHOLD
    const expandedRight = box.x + box.width + EDGE_HOVER_THRESHOLD
    const expandedBottom = box.y + box.height + EDGE_HOVER_THRESHOLD

    if (canvasX < expandedLeft || canvasX > expandedRight || canvasY < expandedTop || canvasY > expandedBottom) {
      continue
    }

    // Local coordinates relative to box
    const localX = canvasX - box.x
    const localY = canvasY - box.y

    const distances: { side: Side; dist: number; ratio: number }[] = [
      { side: 'top', dist: Math.abs(localY), ratio: clamp(localX / box.width, 0.1, 0.9) },
      { side: 'bottom', dist: Math.abs(localY - box.height), ratio: clamp(localX / box.width, 0.1, 0.9) },
      { side: 'left', dist: Math.abs(localX), ratio: clamp(localY / box.height, 0.1, 0.9) },
      { side: 'right', dist: Math.abs(localX - box.width), ratio: clamp(localY / box.height, 0.1, 0.9) },
    ]

    let best = distances[0]!
    for (let d = 1; d < distances.length; d++) {
      if (distances[d]!.dist < best.dist) best = distances[d]!
    }

    if (best.dist > EDGE_HOVER_THRESHOLD) continue

    // Compute handle position in canvas coords
    let cx: number
    let cy: number
    if (best.side === 'top' || best.side === 'bottom') {
      cx = box.x + best.ratio * box.width
      cy = best.side === 'top' ? box.y : box.y + box.height
    } else {
      cx = best.side === 'left' ? box.x : box.x + box.width
      cy = box.y + best.ratio * box.height
    }

    return { boxId, side: best.side, ratio: best.ratio, cx, cy }
  }

  return null
}

// ── Resize Handle Hit Testing ─────────────────────────────────────────────

const RESIZE_HIT_SIZE = 10

type ResizeHit = {
  readonly boxId: string
  readonly handle: ResizeHandle
}

const hitTestResizeHandles = (
  canvasX: number,
  canvasY: number,
  elements: BoardElements,
  selectedIds: ReadonlySet<string>,
): ResizeHit | null => {
  for (const boxId of selectedIds) {
    const box = elements.boxes.get(boxId)
    if (box === undefined) continue

    const { x, y, width: w, height: h } = box
    const half = RESIZE_HIT_SIZE / 2

    const handles: { handle: ResizeHandle; hx: number; hy: number }[] = [
      { handle: 'nw', hx: x, hy: y },
      { handle: 'ne', hx: x + w, hy: y },
      { handle: 'sw', hx: x, hy: y + h },
      { handle: 'se', hx: x + w, hy: y + h },
      { handle: 'n', hx: x + w / 2, hy: y },
      { handle: 's', hx: x + w / 2, hy: y + h },
      { handle: 'e', hx: x + w, hy: y + h / 2 },
      { handle: 'w', hx: x, hy: y + h / 2 },
    ]

    for (let i = 0; i < handles.length; i++) {
      const { handle, hx, hy } = handles[i]!
      if (Math.abs(canvasX - hx) <= half && Math.abs(canvasY - hy) <= half) {
        return { boxId, handle }
      }
    }
  }

  return null
}

// ── Cursor Helpers ────────────────────────────────────────────────────────

const RESIZE_CURSORS: Record<ResizeHandle, string> = {
  nw: 'nwse-resize', ne: 'nesw-resize', sw: 'nesw-resize', se: 'nwse-resize',
  n: 'ns-resize', s: 'ns-resize', e: 'ew-resize', w: 'ew-resize',
}

const clamp = (v: number, min: number, max: number): number => Math.max(min, Math.min(max, v))

// ── Component ─────────────────────────────────────────────────────────────

const Canvas2D = forwardRef<HTMLDivElement, Canvas2DProps>(function Canvas2D(
  {
    elements,
    selection,
    editingBoxId,
    viewport,
    drawingArrow,
    canvasBg,
    gridDotColor,
    connectorColor,
    accentColor,
    surfaceBg,
    textColor,
    onPointerDown,
    onPointerMove,
    onPointerUp,
    onWheel,
    onDoubleClick,
    onBoxTextChange,
    onBoxDoubleClick,
    onBoxBlur,
    onBoxPointerDown,
    onAnchorPointerDown,
    onResizePointerDown,
  },
  ref,
) {
  const canvasRef = useRef<HTMLCanvasElement>(null)
  const textareaRef = useRef<HTMLTextAreaElement>(null)
  const sizeRef = useRef<{ width: number; height: number }>({ width: 0, height: 0 })
  const [edgeHover, setEdgeHover] = useState<EdgeHover | null>(null)
  const [cursor, setCursor] = useState('default')
  const [fontGeneration, setFontGeneration] = useState(0)

  const theme: DrawTheme = useMemo(() => (
    { canvasBg, gridDotColor, connectorColor, accentColor, surfaceBg, textColor }
  ), [canvasBg, gridDotColor, connectorColor, accentColor, surfaceBg, textColor])

  // ── Re-render when fonts finish loading (Virgil woff2) ──────────────
  // Guard: jsdom test environment doesn't implement document.fonts
  useEffect(() => {
    if (!('fonts' in document)) return

    const onLoadingDone = () => { setFontGeneration((g) => g + 1) }
    document.fonts.addEventListener('loadingdone', onLoadingDone)
    return () => { document.fonts.removeEventListener('loadingdone', onLoadingDone) }
  }, [])

  // ── High-DPI Setup + ResizeObserver ───────────────────────────────────
  useEffect(() => {
    const canvas = canvasRef.current
    if (canvas === null) return

    const parent = canvas.parentElement
    if (parent === null) return

    const updateSize = () => {
      const dpr = window.devicePixelRatio || 1
      const rect = parent.getBoundingClientRect()
      sizeRef.current = { width: rect.width, height: rect.height }
      canvas.width = rect.width * dpr
      canvas.height = rect.height * dpr
      canvas.style.width = `${rect.width}px`
      canvas.style.height = `${rect.height}px`
    }

    const observer = new ResizeObserver(updateSize)
    observer.observe(parent)
    updateSize()

    return () => observer.disconnect()
  }, [])

  // ── Canvas Render Pipeline ────────────────────────────────────────────
  useEffect(() => {
    const canvas = canvasRef.current
    if (canvas === null) return

    const ctx = canvas.getContext('2d')
    if (ctx === null) return

    const dpr = window.devicePixelRatio || 1
    const { width: canvasWidth, height: canvasHeight } = sizeRef.current

    // Create RoughCanvas for hand-drawn rendering
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
        drawBoxHighlight(ctx, hoverBox, accentColor)
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

      // Resize still works via cursor change on edge hover — no visible handles
    }

    // Arrows
    for (const [arrowId, arrow] of elements.arrows) {
      const sourceBox = elements.boxes.get(arrow.sourceBoxId)
      const targetBox = elements.boxes.get(arrow.targetBoxId)
      if (sourceBox === undefined || targetBox === undefined) continue

      const path = computeArrowPathPoints(sourceBox, arrow.sourceAnchor, targetBox, arrow.targetAnchor)
      const isSelected = selection.selectedIds.has(arrowId)
      drawArrow(ctx, path, isSelected, theme)
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
        drawDrawingArrow(ctx, path, accentColor)
      }
    }

    // Edge hover handle (small dot on top of the highlight)
    if (edgeHover !== null && editingBoxId === null) {
      drawHandle(ctx, edgeHover.cx, edgeHover.cy, theme)
    }

    // Selection marquee
    if (selection.marquee !== null) {
      drawSelectionRect(ctx, selection.marquee, accentColor)
    }

    ctx.restore()
  }, [elements, selection, editingBoxId, viewport, drawingArrow, edgeHover, theme, accentColor, fontGeneration])

  // ── Focus textarea when editing starts ────────────────────────────────
  useEffect(() => {
    if (editingBoxId !== null && textareaRef.current !== null) {
      const box = elements.boxes.get(editingBoxId)
      if (box !== undefined) {
        textareaRef.current.value = box.text
      }
      textareaRef.current.focus()
      // Place cursor at end
      const len = textareaRef.current.value.length
      textareaRef.current.setSelectionRange(len, len)
    }
  }, [editingBoxId, elements.boxes])

  // ── Pointer Event Handlers ────────────────────────────────────────────

  const handlePointerMove = useCallback((e: React.PointerEvent) => {
    // Forward to parent handler (drag, arrow draw, resize)
    onPointerMove(e)

    // Edge hover detection (only when idle)
    const wrapper = (e.currentTarget as HTMLElement)
    const rect = wrapper.getBoundingClientRect()
    const canvas = screenToCanvas(e.clientX, e.clientY, viewport, rect)

    // Check resize handles first (for cursor)
    const resizeHit = hitTestResizeHandles(canvas.x, canvas.y, elements, selection.selectedIds)
    if (resizeHit !== null) {
      setCursor(RESIZE_CURSORS[resizeHit.handle])
      setEdgeHover(null)
      return
    }

    // Check edge hover
    if (editingBoxId === null) {
      const hover = detectEdgeHover(canvas.x, canvas.y, elements)
      setEdgeHover(hover)

      if (hover !== null) {
        setCursor('crosshair')
        return
      }
    } else {
      setEdgeHover(null)
    }

    // Check if over a box (for grab cursor)
    let overBox = false
    for (let i = elements.boxOrder.length - 1; i >= 0; i--) {
      const boxId = elements.boxOrder[i]!
      const box = elements.boxes.get(boxId)
      if (box === undefined) continue
      if (canvas.x >= box.x && canvas.x <= box.x + box.width &&
          canvas.y >= box.y && canvas.y <= box.y + box.height) {
        overBox = true
        break
      }
    }

    setCursor(overBox ? 'grab' : 'default')
  }, [editingBoxId, elements, onPointerMove, selection.selectedIds, viewport])

  const handlePointerDown = useCallback((e: React.PointerEvent) => {
    if (e.button !== 0) return

    const wrapper = (e.currentTarget as HTMLElement)
    const rect = wrapper.getBoundingClientRect()
    const canvas = screenToCanvas(e.clientX, e.clientY, viewport, rect)

    // Check resize handles first
    const resizeHit = hitTestResizeHandles(canvas.x, canvas.y, elements, selection.selectedIds)
    if (resizeHit !== null) {
      e.stopPropagation()
      e.preventDefault()
      onResizePointerDown(resizeHit.boxId, resizeHit.handle, e)
      return
    }

    // Check edge hover (start arrow drawing)
    if (edgeHover !== null) {
      e.stopPropagation()
      e.preventDefault()
      onAnchorPointerDown(edgeHover.boxId, { side: edgeHover.side, ratio: edgeHover.ratio }, e)
      setEdgeHover(null)
      return
    }

    // Check if clicking on a box
    for (let i = elements.boxOrder.length - 1; i >= 0; i--) {
      const boxId = elements.boxOrder[i]!
      const box = elements.boxes.get(boxId)
      if (box === undefined) continue
      if (canvas.x >= box.x && canvas.x <= box.x + box.width &&
          canvas.y >= box.y && canvas.y <= box.y + box.height) {
        onBoxPointerDown(boxId, e)
        return
      }
    }

    // Clicked on empty space
    onPointerDown(e)
  }, [edgeHover, elements, onAnchorPointerDown, onBoxPointerDown, onPointerDown, onResizePointerDown, selection.selectedIds, viewport])

  const handleDoubleClick = useCallback((e: React.MouseEvent) => {
    const wrapper = (e.currentTarget as HTMLElement)
    const rect = wrapper.getBoundingClientRect()
    const canvas = screenToCanvas(e.clientX, e.clientY, viewport, rect)

    // Check if double-clicking on a box
    for (let i = elements.boxOrder.length - 1; i >= 0; i--) {
      const boxId = elements.boxOrder[i]!
      const box = elements.boxes.get(boxId)
      if (box === undefined) continue
      if (canvas.x >= box.x && canvas.x <= box.x + box.width &&
          canvas.y >= box.y && canvas.y <= box.y + box.height) {
        onBoxDoubleClick(boxId)
        return
      }
    }

    // Double-click on empty space
    onDoubleClick(e)
  }, [elements, onBoxDoubleClick, onDoubleClick, viewport])

  // ── Textarea Handlers ─────────────────────────────────────────────────

  const handleTextareaInput = useCallback(() => {
    if (editingBoxId === null || textareaRef.current === null) return

    const text = textareaRef.current.value
    const el = textareaRef.current
    // Reset height to get accurate scrollHeight
    el.style.height = 'auto'
    const contentWidth = Math.max(BOARD.MIN_BOX_WIDTH - BOARD.BOX_PADDING_X * 2, el.scrollWidth)
    const contentHeight = el.scrollHeight
    el.style.height = `${contentHeight}px`

    const width = Math.max(BOARD.MIN_BOX_WIDTH, Math.min(contentWidth + BOARD.BOX_PADDING_X * 2, BOARD.MAX_BOX_WIDTH))
    const height = Math.max(BOARD.MIN_BOX_HEIGHT, contentHeight + BOARD.BOX_PADDING_Y * 2)

    onBoxTextChange(editingBoxId, text, width, height)
  }, [editingBoxId, onBoxTextChange])

  const handleTextareaKeyDown = useCallback((e: React.KeyboardEvent) => {
    if (e.key === 'Escape') {
      e.preventDefault()
      if (editingBoxId !== null) {
        onBoxBlur(editingBoxId)
      }
      return
    }
    // Stop propagation to prevent board keyboard shortcuts while typing
    e.stopPropagation()
  }, [editingBoxId, onBoxBlur])

  const handleTextareaBlur = useCallback(() => {
    if (editingBoxId !== null) {
      onBoxBlur(editingBoxId)
    }
  }, [editingBoxId, onBoxBlur])

  // ── Textarea Position (screen coords) ─────────────────────────────────

  const editingBox = editingBoxId !== null ? elements.boxes.get(editingBoxId) ?? null : null

  return (
    <div
      ref={ref}
      onPointerDown={handlePointerDown}
      onPointerMove={handlePointerMove}
      onPointerUp={onPointerUp}
      onWheel={onWheel}
      onDoubleClick={handleDoubleClick}
      style={{
        position: 'relative',
        width: '100%',
        height: '100%',
        overflow: 'hidden',
        backgroundColor: canvasBg,
        cursor,
      }}
    >
      <canvas
        ref={canvasRef}
        style={{
          position: 'absolute',
          inset: 0,
          pointerEvents: 'none',
        }}
      />

      {/* Textarea overlay for text editing */}
      {editingBox !== null && (
        <textarea
          ref={textareaRef}
          onInput={handleTextareaInput}
          onKeyDown={handleTextareaKeyDown}
          onBlur={handleTextareaBlur}
          style={{
            position: 'absolute',
            left: editingBox.x * viewport.zoom + viewport.panX + BOARD.BOX_PADDING_X * viewport.zoom,
            top: editingBox.y * viewport.zoom + viewport.panY + BOARD.BOX_PADDING_Y * viewport.zoom,
            width: (editingBox.width - BOARD.BOX_PADDING_X * 2) * viewport.zoom,
            minHeight: (editingBox.height - BOARD.BOX_PADDING_Y * 2) * viewport.zoom,
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
          }}
        />
      )}
    </div>
  )
})

export { Canvas2D }
