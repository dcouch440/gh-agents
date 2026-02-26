// ============================================================================
// Canvas2D — Canvas-Based Drawing Surface
// ============================================================================
//
// Renders all board elements (grid, boxes, arrows, handles) on a single
// <canvas> element. Text editing uses a temporary <textarea> overlay
// positioned over the box being edited — the same approach as Excalidraw.

import { useCallback, useEffect, useMemo, useRef, useState } from 'react'
import { BOARD } from '../constants'
import type { ActiveTool, AnchorPoint, BoardElements, DrawingArrow, DrawingBox, DrawingPen, EdgeHover, InteractionMode, ResizeHandle, SelectionState, ViewportState } from '../elements'
import { detectEdgeHover, eventToCanvas, hitTestArrow, hitTestBox, hitTestPen, hitTestResizeHandles, RESIZE_CURSORS } from '../elements'
import { renderBoard } from './renderer'
import type { DrawTheme } from './renderer'
import { computeTextareaStyle } from './textareaStyle'

// ── Props ─────────────────────────────────────────────────────────────────

type Canvas2DProps = {
  readonly ref: React.Ref<HTMLDivElement>
  readonly elements: BoardElements
  readonly selection: SelectionState
  readonly editingBoxId: string | null
  readonly activeTool: ActiveTool
  readonly interaction: InteractionMode
  readonly viewport: ViewportState
  readonly drawingArrow: DrawingArrow
  readonly drawingBox: DrawingBox
  readonly drawingPen: DrawingPen
  readonly canvasBg: string
  readonly gridDotColor: string
  readonly connectorColor: string
  readonly strokeColor: string
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
  readonly onContextMenu: (x: number, y: number, elementId: string | null) => void
}

// ── Component ─────────────────────────────────────────────────────────────

function Canvas2D({
  ref,
  elements,
  selection,
  editingBoxId,
  activeTool,
  interaction,
  viewport,
  drawingArrow,
  drawingBox,
  drawingPen,
  canvasBg,
  gridDotColor,
  connectorColor,
  strokeColor,
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
  onContextMenu,
}: Canvas2DProps) {
  const canvasRef = useRef<HTMLCanvasElement>(null)
  const textareaRef = useRef<HTMLTextAreaElement>(null)
  const sizeRef = useRef<{ width: number; height: number }>({ width: 0, height: 0 })
  const [edgeHover, setEdgeHover] = useState<EdgeHover | null>(null)
  const [cursor, setCursor] = useState('default')
  const [fontGeneration, setFontGeneration] = useState(0)
  const renderRef = useRef<() => void>(() => {})

  const theme: DrawTheme = useMemo(() => (
    { canvasBg, gridDotColor, connectorColor, strokeColor, accentColor, surfaceBg, textColor }
  ), [canvasBg, gridDotColor, connectorColor, strokeColor, accentColor, surfaceBg, textColor])

  // Keep renderRef pointing at the latest render closure so the
  // ResizeObserver can repaint synchronously with current state.
  useEffect(() => {
    renderRef.current = () => {
      const cvs = canvasRef.current
      if (cvs === null) return
      const { width, height } = sizeRef.current
      renderBoard(cvs, width, height, elements, selection, editingBoxId, viewport, drawingArrow, drawingBox, drawingPen, edgeHover, theme)
    }
  })

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
      renderRef.current()
    }

    const observer = new ResizeObserver(updateSize)
    observer.observe(parent)
    updateSize()

    return () => observer.disconnect()
  }, [])

  // ── Canvas Render Pipeline ────────────────────────────────────────────
  useEffect(() => {
    renderRef.current()
  }, [elements, selection, editingBoxId, viewport, drawingArrow, drawingBox, drawingPen, edgeHover, theme, fontGeneration])

  // ── Focus textarea when editing starts ────────────────────────────────
  useEffect(() => {
    if (editingBoxId !== null && textareaRef.current !== null) {
      const box = elements.boxes.get(editingBoxId)
      if (box !== undefined) {
        textareaRef.current.value = box.text
      }
      textareaRef.current.focus()
      const len = textareaRef.current.value.length
      textareaRef.current.setSelectionRange(len, len)
    }
  }, [editingBoxId, elements.boxes])

  // ── Pointer Event Handlers ────────────────────────────────────────────

  const handlePointerMove = useCallback((e: React.PointerEvent) => {
    onPointerMove(e)

    // During panning, show grabbing cursor and skip hit testing
    if (interaction.type === 'panning') {
      setCursor('grabbing')
      setEdgeHover(null)
      return
    }

    // During box drawing, show crosshair and skip hit testing
    if (interaction.type === 'drawing-box') {
      setCursor('crosshair')
      setEdgeHover(null)
      return
    }

    // During pen drawing, show crosshair and skip hit testing
    if (interaction.type === 'drawing-pen') {
      setCursor('crosshair')
      setEdgeHover(null)
      return
    }

    // During active drag/resize/draw, skip hit testing
    if (interaction.type === 'dragging' || interaction.type === 'resizing' || interaction.type === 'drawing-arrow') {
      return
    }

    const canvas = eventToCanvas(e, viewport)

    // Check resize handles first (for cursor)
    const resizeHit = hitTestResizeHandles(canvas.x, canvas.y, elements, selection.selectedIds)
    if (resizeHit !== null) {
      setCursor(RESIZE_CURSORS[resizeHit.handle])
      setEdgeHover(null)
      return
    }

    // Check edge hover (only when arrow tool is active)
    if (activeTool === 'arrow' && editingBoxId === null) {
      const hover = detectEdgeHover(canvas.x, canvas.y, elements, viewport.zoom)
      setEdgeHover(hover)

      if (hover !== null) {
        setCursor('crosshair')
        return
      }
    } else {
      setEdgeHover(null)
    }

    // Check if over a box (for grab cursor)
    const overBox = hitTestBox(elements, canvas) !== null
    if (activeTool === 'box' || activeTool === 'pen') {
      setCursor('crosshair')
    } else {
      setCursor(overBox ? 'grab' : 'default')
    }
  }, [activeTool, editingBoxId, elements, interaction.type, onPointerMove, selection.selectedIds, viewport])

  const handlePointerDown = useCallback((e: React.PointerEvent) => {
    if (e.button !== 0) return

    const canvas = eventToCanvas(e, viewport)

    // Pen tool: always pass through to Board handler (draw anywhere, including over boxes)
    if (activeTool === 'pen') {
      onPointerDown(e)
      return
    }

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
    const boxId = hitTestBox(elements, canvas)
    if (boxId !== null) {
      onBoxPointerDown(boxId, e)
      return
    }

    // Clicked on empty space
    onPointerDown(e)
  }, [activeTool, edgeHover, elements, onAnchorPointerDown, onBoxPointerDown, onPointerDown, onResizePointerDown, selection.selectedIds, viewport])

  const handleDoubleClick = useCallback((e: React.MouseEvent) => {
    const canvas = eventToCanvas(e, viewport)

    const boxId = hitTestBox(elements, canvas)
    if (boxId !== null) {
      onBoxDoubleClick(boxId)
      return
    }

    onDoubleClick(e)
  }, [elements, onBoxDoubleClick, onDoubleClick, viewport])

  // ── Textarea Handlers ─────────────────────────────────────────────────

  const handleTextareaInput = useCallback(() => {
    if (editingBoxId === null || textareaRef.current === null) return

    const text = textareaRef.current.value
    const el = textareaRef.current
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
    e.stopPropagation()
  }, [editingBoxId, onBoxBlur])

  const handleTextareaBlur = useCallback(() => {
    if (editingBoxId !== null) {
      onBoxBlur(editingBoxId)
    }
  }, [editingBoxId, onBoxBlur])

  // ── Context Menu ────────────────────────────────────────────────────
  const handleContextMenu = useCallback((e: React.MouseEvent) => {
    e.preventDefault()

    const canvas = eventToCanvas(e, viewport)

    const boxId = hitTestBox(elements, canvas)
    if (boxId !== null) {
      onContextMenu(e.clientX, e.clientY, boxId)
      return
    }

    const arrowId = hitTestArrow(elements, canvas, 8)
    if (arrowId !== null) {
      onContextMenu(e.clientX, e.clientY, arrowId)
      return
    }

    const penId = hitTestPen(elements, canvas, 8)
    if (penId !== null) {
      onContextMenu(e.clientX, e.clientY, penId)
      return
    }

    onContextMenu(e.clientX, e.clientY, null)
  }, [elements, onContextMenu, viewport])

  // ── Render ──────────────────────────────────────────────────────────────

  const editingBox = editingBoxId !== null ? elements.boxes.get(editingBoxId) ?? null : null

  return (
    <div
      ref={ref}
      onPointerDown={handlePointerDown}
      onPointerMove={handlePointerMove}
      onPointerUp={onPointerUp}
      onWheel={onWheel}
      onDoubleClick={handleDoubleClick}
      onContextMenu={handleContextMenu}
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

      {editingBox !== null && (
        <textarea
          ref={textareaRef}
          onInput={handleTextareaInput}
          onKeyDown={handleTextareaKeyDown}
          onBlur={handleTextareaBlur}
          style={computeTextareaStyle(editingBox, viewport, textColor)}
        />
      )}
    </div>
  )
}

export { Canvas2D }
