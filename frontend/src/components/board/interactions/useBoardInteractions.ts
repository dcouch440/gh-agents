// ============================================================================
// useBoardInteractions — Consolidated Board Interaction Hook
// ============================================================================
//
// Owns all interaction state (selection, active tool, viewport, interaction
// mode) and wires up the individual interaction hooks (drag, resize, pen,
// arrow, pan-zoom, keyboard, selection). Board.tsx consumes the returned
// state and handlers without needing to know how interactions are dispatched.

import { useCallback, useState } from 'react'
import type { Point } from '@/utils/geometry'
import { undoStore } from '@/stores/undoStore'
import type { DrawingPreviews } from '../canvas'
import { BOARD } from '../constants'
import type { ActiveTool, AnchorPoint, BoardElements, DrawingArrow, DrawingBox, DrawingPen, InteractionMode, ResizeHandle, SelectionState, ViewportState } from '../elements'
import { addBox, containerEventToCanvas, createBox, createBoxWithSize, hitTest, removeElements, selectAllIds, updateBoxText } from '../elements'
import { useArrowDraw } from './useArrowDraw'
import { useDrag } from './useDrag'
import { useKeyboard } from './useKeyboard'
import { usePanZoom } from './usePanZoom'
import { usePenDraw } from './usePenDraw'
import { useResize } from './useResize'
import { EMPTY_SELECTION, useSelection } from './useSelection'
import type { SetElements } from './types'

// ── Return type ──────────────────────────────────────────────────────────

type CanvasHandlers = {
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

type BoardInteractions = {
  readonly interaction: InteractionMode
  readonly selection: SelectionState
  readonly viewport: ViewportState
  readonly activeTool: ActiveTool
  readonly editingBoxId: string | null
  readonly previews: DrawingPreviews
  readonly handlers: CanvasHandlers
  readonly setActiveTool: (tool: ActiveTool) => void
  readonly zoomIn: () => void
  readonly zoomOut: () => void
  readonly resetZoom: () => void
  readonly handleContextMenuDelete: (elementId: string | null | undefined) => void
  readonly handleContextMenuSelectAll: () => void
}

// ── Hook ─────────────────────────────────────────────────────────────────

const useBoardInteractions = (
  elements: BoardElements,
  setElements: SetElements,
  containerRef: React.RefObject<HTMLDivElement | null>,
  onDeleteElements: (deletedIds: ReadonlySet<string>) => void,
  onContextMenuOpen: (x: number, y: number, elementId: string | null) => void,
): BoardInteractions => {
  // ── State ────────────────────────────────────────────────────────────
  const [selection, setSelection] = useState<SelectionState>(EMPTY_SELECTION)
  const [interaction, setInteraction] = useState<InteractionMode>({ type: 'idle' })
  const [viewport, setViewport] = useState<ViewportState>({ panX: 0, panY: 0, zoom: 1 })
  const [activeTool, setActiveTool] = useState<ActiveTool>('select')

  // ── Sub-hooks ────────────────────────────────────────────────────────
  const { onWheel, zoomIn, zoomOut, resetZoom } = usePanZoom(viewport, setViewport)
  const drag = useDrag(setElements, setInteraction, viewport, containerRef)
  const arrowDraw = useArrowDraw(setElements, setInteraction, viewport, containerRef)
  const resize = useResize(setElements, setInteraction, viewport, containerRef)
  const penDraw = usePenDraw(setElements, setInteraction, viewport, containerRef)
  const sel = useSelection(setSelection)
  useKeyboard(elements, setElements, selection, setSelection, interaction, setInteraction, onDeleteElements, setActiveTool)

  // ── Derived state ────────────────────────────────────────────────────
  const editingBoxId = interaction.type === 'editing' ? interaction.boxId : null

  const drawingArrow: DrawingArrow = interaction.type === 'drawing-arrow'
    ? { sourceBoxId: interaction.sourceBoxId, sourceFocus: interaction.sourceFocus, cursorX: interaction.cursorX, cursorY: interaction.cursorY }
    : null

  const drawingBox: DrawingBox = interaction.type === 'drawing-box'
    ? {
      x: Math.min(interaction.startX, interaction.cursorX),
      y: Math.min(interaction.startY, interaction.cursorY),
      width: Math.abs(interaction.cursorX - interaction.startX),
      height: Math.abs(interaction.cursorY - interaction.startY),
    }
    : null

  const drawingPen: DrawingPen = interaction.type === 'drawing-pen'
    ? { points: interaction.points, pressures: interaction.pressures }
    : null

  const previews: DrawingPreviews = { arrow: drawingArrow, box: drawingBox, pen: drawingPen }

  // ── Helpers ──────────────────────────────────────────────────────────

  const createBoxAtPoint = useCallback((point: Point) => {
    undoStore.push('create-box')
    const box = createBox(point.x - BOARD.DEFAULT_BOX_WIDTH / 2, point.y - BOARD.DEFAULT_BOX_HEIGHT / 2)
    setElements((s) => addBox(s, box))
    setInteraction({ type: 'editing', boxId: box.id })
    sel.selectElement(box.id, false)
  }, [sel, setElements])

  // ── Pointer event handlers ───────────────────────────────────────────

  const onPointerDown = useCallback((e: React.PointerEvent) => {
    if (e.button !== 0) return

    const canvas = containerEventToCanvas(containerRef, e, viewport)
    if (canvas === null) return

    if (activeTool === 'pen') {
      penDraw.onPenStart(e)
      return
    }

    if (activeTool === 'box') {
      setInteraction({ type: 'drawing-box', startX: canvas.x, startY: canvas.y, cursorX: canvas.x, cursorY: canvas.y })
      return
    }

    const hitId = hitTest(elements, canvas)
    if (hitId !== null) {
      sel.selectElement(hitId, e.shiftKey)
    } else {
      sel.clearSelection()
      setInteraction({
        type: 'panning',
        startX: e.clientX,
        startY: e.clientY,
        startPanX: viewport.panX,
        startPanY: viewport.panY,
      })
    }
  }, [activeTool, containerRef, elements, penDraw, sel, viewport])

  const onPointerMove = useCallback((e: React.PointerEvent) => {
    switch (interaction.type) {
      case 'drawing-pen': { penDraw.onPenMove(e, interaction); break }
      case 'dragging': { drag.onDragMove(e, interaction); break }
      case 'drawing-box': {
        const canvas = containerEventToCanvas(containerRef, e, viewport)
        if (canvas !== null) {
          setInteraction({ ...interaction, cursorX: canvas.x, cursorY: canvas.y })
        }
        break
      }
      case 'drawing-arrow': { arrowDraw.onArrowMove(e, interaction); break }
      case 'resizing': { resize.onResizeMove(e, interaction); break }
      case 'panning': {
        const dx = e.clientX - interaction.startX
        const dy = e.clientY - interaction.startY
        setViewport(() => ({
          ...viewport,
          panX: interaction.startPanX + dx,
          panY: interaction.startPanY + dy,
        }))
        break
      }
      case 'idle': case 'editing': break
    }
  }, [arrowDraw, containerRef, drag, interaction, penDraw, resize, setViewport, viewport])

  const onPointerUp = useCallback((e: React.PointerEvent) => {
    switch (interaction.type) {
      case 'drawing-pen': { penDraw.onPenEnd(interaction); break }
      case 'dragging': {
        drag.onDragEnd()
        undoStore.commit()
        break
      }
      case 'drawing-box': {
        const dx = Math.abs(interaction.cursorX - interaction.startX)
        const dy = Math.abs(interaction.cursorY - interaction.startY)

        undoStore.push('draw-box')

        if (dx < 5 && dy < 5) {
          const box = createBox(
            interaction.startX - BOARD.DEFAULT_BOX_WIDTH / 2,
            interaction.startY - BOARD.DEFAULT_BOX_HEIGHT / 2,
          )
          setElements((s) => addBox(s, box))
          setInteraction({ type: 'editing', boxId: box.id })
          sel.selectElement(box.id, false)
        } else {
          const x = Math.min(interaction.startX, interaction.cursorX)
          const y = Math.min(interaction.startY, interaction.cursorY)
          const w = Math.max(BOARD.MIN_BOX_WIDTH, dx)
          const h = Math.max(BOARD.MIN_BOX_HEIGHT, dy)

          const box = createBoxWithSize(x, y, w, h)
          setElements((s) => addBox(s, box))
          setInteraction({ type: 'editing', boxId: box.id })
          sel.selectElement(box.id, false)
        }

        setActiveTool('select')
        break
      }
      case 'drawing-arrow': { arrowDraw.onArrowEnd(e, interaction, elements); break }
      case 'resizing': {
        resize.onResizeEnd()
        undoStore.commit()
        break
      }
      case 'panning': { setInteraction({ type: 'idle' }); break }
      case 'idle': case 'editing': break
    }
  }, [arrowDraw, drag, elements, interaction, penDraw, resize, sel, setElements])

  const onDoubleClick = useCallback((e: React.MouseEvent) => {
    const canvas = containerEventToCanvas(containerRef, e, viewport)
    if (canvas === null) return

    const hitId = hitTest(elements, canvas)
    if (hitId !== null && elements.boxes.has(hitId)) {
      undoStore.beginTransaction('edit-text')
      setInteraction({ type: 'editing', boxId: hitId })
      return
    }

    createBoxAtPoint(canvas)
  }, [containerRef, createBoxAtPoint, elements, viewport])

  // ── Element event handlers ───────────────────────────────────────────

  const onBoxPointerDown = useCallback((boxId: string, e: React.PointerEvent) => {
    e.stopPropagation()
    sel.selectElement(boxId, e.shiftKey)
    undoStore.beginTransaction('move')
    drag.onDragStart(boxId, e, elements)
  }, [drag, elements, sel])

  const onBoxDoubleClick = useCallback((boxId: string) => {
    undoStore.beginTransaction('edit-text')
    setInteraction({ type: 'editing', boxId })
  }, [])

  const onBoxBlur = useCallback((boxId: string) => {
    if (interaction.type === 'editing' && interaction.boxId === boxId) {
      undoStore.commit()
      setInteraction({ type: 'idle' })
    }
  }, [interaction])

  const onBoxTextChange = useCallback((boxId: string, text: string, width: number, height: number) => {
    setElements((s) => updateBoxText(s, boxId, text, width, height))
  }, [setElements])

  const onAnchorPointerDown = useCallback((boxId: string, anchor: AnchorPoint, e: React.PointerEvent) => {
    arrowDraw.onArrowStart(boxId, anchor, e)
  }, [arrowDraw])

  const onResizePointerDown = useCallback((boxId: string, handle: ResizeHandle, e: React.PointerEvent) => {
    undoStore.beginTransaction('resize')
    resize.onResizeStart(boxId, handle, e, elements)
  }, [elements, resize])

  const onContextMenu = onContextMenuOpen

  // ── Context menu actions ─────────────────────────────────────────────

  const handleContextMenuDelete = useCallback((elementId: string | null | undefined) => {
    if (elementId === null || elementId === undefined) return
    undoStore.push('delete')
    const ids = selection.selectedIds.has(elementId)
      ? selection.selectedIds
      : new Set([elementId])
    setElements((s) => removeElements(s, ids))
    onDeleteElements(ids)
    setSelection(() => EMPTY_SELECTION)
  }, [selection.selectedIds, setElements, onDeleteElements])

  const handleContextMenuSelectAll = useCallback(() => {
    setSelection(() => ({ selectedIds: selectAllIds(elements), marquee: null }))
  }, [elements])

  // ── Return ───────────────────────────────────────────────────────────

  return {
    interaction,
    selection,
    viewport,
    activeTool,
    editingBoxId,
    previews,
    handlers: {
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
    },
    setActiveTool,
    zoomIn,
    zoomOut,
    resetZoom,
    handleContextMenuDelete,
    handleContextMenuSelectAll,
  }
}

export { useBoardInteractions }
export type { BoardInteractions, CanvasHandlers }
