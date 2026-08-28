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
import type { CanvasChangeCallback, SetElements } from './types'

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
  onCanvasChange: CanvasChangeCallback,
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
  useKeyboard(elements, setElements, selection, setSelection, interaction, setInteraction, onDeleteElements, setActiveTool, onCanvasChange)

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
    const newBox = createBox(point.x - BOARD.DEFAULT_BOX_WIDTH / 2, point.y - BOARD.DEFAULT_BOX_HEIGHT / 2)
    setElements((s) => addBox(s, newBox))
    setInteraction({ type: 'editing', boxId: newBox.id })
    sel.selectElement(newBox.id, false)
    onCanvasChange({ kind: 'node_created', box: newBox })
  }, [onCanvasChange, sel, setElements])

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
        // Sync final position to backend
        const draggedBox = elements.boxes.get(interaction.elementId)
        if (draggedBox) {
          onCanvasChange({ kind: 'moved', elementId: draggedBox.id, x: draggedBox.x, y: draggedBox.y, width: draggedBox.width, height: draggedBox.height })
        }
        break
      }
      case 'drawing-box': {
        const dx = Math.abs(interaction.cursorX - interaction.startX)
        const dy = Math.abs(interaction.cursorY - interaction.startY)

        undoStore.push('draw-box')

        if (dx < 5 && dy < 5) {
          const newBox = createBox(
            interaction.startX - BOARD.DEFAULT_BOX_WIDTH / 2,
            interaction.startY - BOARD.DEFAULT_BOX_HEIGHT / 2,
          )
          setElements((s) => addBox(s, newBox))
          setInteraction({ type: 'editing', boxId: newBox.id })
          sel.selectElement(newBox.id, false)
          onCanvasChange({ kind: 'node_created', box: newBox })
        } else {
          const bx = Math.min(interaction.startX, interaction.cursorX)
          const by = Math.min(interaction.startY, interaction.cursorY)
          const bw = Math.max(BOARD.MIN_BOX_WIDTH, dx)
          const bh = Math.max(BOARD.MIN_BOX_HEIGHT, dy)

          const newBox = createBoxWithSize(bx, by, bw, bh)
          setElements((s) => addBox(s, newBox))
          setInteraction({ type: 'editing', boxId: newBox.id })
          sel.selectElement(newBox.id, false)
          onCanvasChange({ kind: 'node_created', box: newBox })
        }

        setActiveTool('select')
        break
      }
      case 'drawing-arrow': {
        const createdArrow = arrowDraw.onArrowEnd(e, interaction, elements)
        if (createdArrow) {
          onCanvasChange({ kind: 'edge_created', arrow: createdArrow })
        }
        break
      }
      case 'resizing': {
        resize.onResizeEnd()
        undoStore.commit()
        // Sync resized geometry to backend
        const resizedBox = elements.boxes.get(interaction.boxId)
        if (resizedBox) {
          onCanvasChange({ kind: 'moved', elementId: resizedBox.id, x: resizedBox.x, y: resizedBox.y, width: resizedBox.width, height: resizedBox.height })
        }
        break
      }
      case 'panning': { setInteraction({ type: 'idle' }); break }
      case 'idle': case 'editing': break
    }
  }, [arrowDraw, drag, elements, interaction, onCanvasChange, penDraw, resize, sel, setElements])

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
      // Sync text change to backend
      const blurredBox = elements.boxes.get(boxId)
      if (blurredBox) {
        onCanvasChange({ kind: 'text_changed', elementId: boxId, text: blurredBox.text, width: blurredBox.width, height: blurredBox.height })
      }
    }
  }, [elements, interaction, onCanvasChange])

  const onBoxTextChange = useCallback((boxId: string, text: string, width: number, height: number) => {
    setElements((s) => updateBoxText(s, boxId, text, width, height))
    // Sync as the user types, not only on blur. Text stranded in local state
    // until a blur that may never come (navigating away, or clicking Generate
    // straight after typing) is text the server never sees — and a step with an
    // empty description is skipped by generate.
    onCanvasChange({ kind: 'text_changed', elementId: boxId, text, width, height })
  }, [onCanvasChange, setElements])

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
    // Notify backend before removing from local state (need element type info)
    onCanvasChange({ kind: 'elements_deleted', deletedIds: ids, elements })
    setElements((s) => removeElements(s, ids))
    onDeleteElements(ids)
    setSelection(() => EMPTY_SELECTION)
  }, [elements, selection.selectedIds, setElements, onDeleteElements, onCanvasChange])

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
