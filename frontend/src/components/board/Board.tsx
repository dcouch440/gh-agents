import { useCallback, useEffect, useMemo, useRef, useState } from 'react'
import Box from '@mui/material/Box'
import CircularProgress from '@mui/material/CircularProgress'
import type { Point } from '@/utils/geometry'
import { boardStore, workflowStore, sidebarStore, useStore } from '@/stores'
import { useWorkflowRun } from '@/components/canvas/useWorkflowRun'
import { useBoardTheme, useBoardSubmit, useBoardElements, useDispatchHistory, useActivityHistory } from './hooks'
import { BoardContextMenu } from './BoardContextMenu'
import type { MenuPosition } from './BoardContextMenu'
import { SubmitBar } from './SubmitBar'
import { DispatchPanel } from './dispatch'
import { Canvas2D } from './canvas'
import { Toolbar } from './toolbar'
import { useHistory } from './history'
import { useArrowDraw, useDrag, useKeyboard, usePanZoom, usePenDraw, useResize, useSelection, EMPTY_SELECTION } from './interactions'
import { BOARD } from './constants'
import type { ActiveTool, AnchorPoint, BoardElements, DrawingArrow, DrawingBox, DrawingPen, InteractionMode, ResizeHandle, SelectionState, ViewportState } from './elements'
import { addBox, containerEventToCanvas, createBox, createBoxWithSize, emptyBoard, hitTest, removeElements, selectAllIds, updateBoxText } from './elements'

type BoardProps = {
  readonly workflowId: string
}

/**
 * Custom typing-first canvas — all rendering via Canvas 2D API.
 *
 * Serializes internal elements to Excalidraw JSON format on submit
 * so the backend board_serializer remains unchanged.
 */
function Board({ workflowId }: BoardProps) {
  // ── State ────────────────────────────────────────────────────────────────
  const [elements, setElements] = useState<BoardElements>(emptyBoard)
  const [selection, setSelection] = useState<SelectionState>(EMPTY_SELECTION)
  const [interaction, setInteraction] = useState<InteractionMode>({ type: 'idle' })
  const [viewport, setViewport] = useState<ViewportState>({ panX: 0, panY: 0, zoom: 1 })
  const [activeTool, setActiveTool] = useState<ActiveTool>('select')
  const [showDebug, setShowDebug] = useState(false)
  const [contextMenu, setContextMenu] = useState<MenuPosition | null>(null)
  const containerRef = useRef<HTMLDivElement>(null)

  // ── Hooks ────────────────────────────────────────────────────────────────
  const theme = useBoardTheme()
  const { loading } = useBoardElements(workflowId, setElements)
  const { handleSubmit, isSubmitting, error, status } = useBoardSubmit(workflowId, elements)
  const history = useHistory(elements)

  const steps = useStore(workflowStore.store, workflowStore.selectSteps)
  const entryStep = useMemo(() => {
    const inputStep = steps.find((s) => s.execution_mode === 'input')
    if (inputStep) return inputStep
    return steps.find((s) => s.execution_mode === 'context') ?? null
  }, [steps])
  const { status: runStatus, handleRun } = useWorkflowRun(entryStep?.prompt_template ?? '')

  useDispatchHistory(workflowId)
  useActivityHistory(workflowId)

  useEffect(() => {
    return () => { boardStore.resetBoard() }
  }, [workflowId])

  // ── Helpers ─────────────────────────────────────────────────────────────

  /** Sync element deletions to the workflow store so the sidebar updates. */
  const syncDeletedElements = useCallback((deletedIds: ReadonlySet<string>) => {
    const elementStepMap = boardStore.store.getState().elementStepMap
    for (const elementId of deletedIds) {
      const stepId = elementStepMap[elementId]
      if (stepId) {
        workflowStore.removeStepLocal(stepId)
        if (sidebarStore.store.getState().selectedStepId === stepId) {
          sidebarStore.clearSelection()
        }
      }
    }
  }, [])

  // ── Interaction hooks ────────────────────────────────────────────────────
  const { onWheel, zoomIn, zoomOut, resetZoom } = usePanZoom(viewport, setViewport)
  const drag = useDrag(setElements, setInteraction, viewport, containerRef)
  const arrowDraw = useArrowDraw(setElements, setInteraction, viewport, containerRef)
  const resize = useResize(setElements, setInteraction, viewport, containerRef)
  const penDraw = usePenDraw(setElements, setInteraction, viewport, containerRef)
  const sel = useSelection(setSelection)
  useKeyboard(elements, setElements, selection, setSelection, interaction, setInteraction, history, syncDeletedElements, setActiveTool)

  // ── Derived state ────────────────────────────────────────────────────────
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

  const createBoxAtPoint = useCallback((point: Point) => {
    history.push(elements)
    const box = createBox(point.x - BOARD.DEFAULT_BOX_WIDTH / 2, point.y - BOARD.DEFAULT_BOX_HEIGHT / 2)
    setElements((s) => addBox(s, box))
    setInteraction({ type: 'editing', boxId: box.id })
    sel.selectElement(box.id, false)
  }, [elements, history, sel])

  // ── Event handlers ─────────────────────────────────────────────────────

  const handlePointerDown = useCallback((e: React.PointerEvent) => {
    if (e.button !== 0) return

    const canvas = containerEventToCanvas(containerRef, e, viewport)
    if (canvas === null) return

    // Pen tool: start drawing stroke
    if (activeTool === 'pen') {
      penDraw.onPenStart(e)
      return
    }

    // Box tool: start drag-to-size
    if (activeTool === 'box') {
      setInteraction({ type: 'drawing-box', startX: canvas.x, startY: canvas.y, cursorX: canvas.x, cursorY: canvas.y })
      return
    }

    // Check if we hit something
    const hitId = hitTest(elements, canvas)
    if (hitId !== null) {
      sel.selectElement(hitId, e.shiftKey)
    } else {
      // Click on empty canvas — start panning
      sel.clearSelection()
      setInteraction({
        type: 'panning',
        startX: e.clientX,
        startY: e.clientY,
        startPanX: viewport.panX,
        startPanY: viewport.panY,
      })
    }
  }, [activeTool, elements, penDraw, sel, viewport])

  const handlePointerMove = useCallback((e: React.PointerEvent) => {
    if (interaction.type === 'drawing-pen') {
      penDraw.onPenMove(e, interaction)
      return
    } else if (interaction.type === 'dragging') {
      drag.onDragMove(e, interaction)
    } else if (interaction.type === 'drawing-box') {
      const canvas = containerEventToCanvas(containerRef, e, viewport)
      if (canvas !== null) {
        setInteraction({ ...interaction, cursorX: canvas.x, cursorY: canvas.y })
      }
    } else if (interaction.type === 'drawing-arrow') {
      arrowDraw.onArrowMove(e, interaction)
    } else if (interaction.type === 'resizing') {
      resize.onResizeMove(e, interaction)
    } else if (interaction.type === 'panning') {
      const dx = e.clientX - interaction.startX
      const dy = e.clientY - interaction.startY
      setViewport(() => ({
        ...viewport,
        panX: interaction.startPanX + dx,
        panY: interaction.startPanY + dy,
      }))
    }
  }, [arrowDraw, drag, interaction, penDraw, resize, setViewport, viewport])

  const handlePointerUp = useCallback((e: React.PointerEvent) => {
    if (interaction.type === 'drawing-pen') {
      penDraw.onPenEnd(interaction)
      return
    } else if (interaction.type === 'dragging') {
      drag.onDragEnd()
    } else if (interaction.type === 'drawing-box') {
      const dx = Math.abs(interaction.cursorX - interaction.startX)
      const dy = Math.abs(interaction.cursorY - interaction.startY)

      history.push(elements)

      // Tiny drag — treat as click, use default size
      if (dx < 5 && dy < 5) {
        const box = createBox(
          interaction.startX - BOARD.DEFAULT_BOX_WIDTH / 2,
          interaction.startY - BOARD.DEFAULT_BOX_HEIGHT / 2,
        )
        setElements((s) => addBox(s, box))
        setInteraction({ type: 'editing', boxId: box.id })
        sel.selectElement(box.id, false)
      } else {
        // Normalize rect (handle drag up-left)
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
    } else if (interaction.type === 'drawing-arrow') {
      arrowDraw.onArrowEnd(e, interaction, elements)
    } else if (interaction.type === 'resizing') {
      resize.onResizeEnd()
    } else if (interaction.type === 'panning') {
      setInteraction({ type: 'idle' })
    }
  }, [arrowDraw, drag, elements, history, interaction, penDraw, resize, sel])

  const handleDoubleClick = useCallback((e: React.MouseEvent) => {
    const canvas = containerEventToCanvas(containerRef, e, viewport)
    if (canvas === null) return

    // Check if we hit an existing box
    const hitId = hitTest(elements, canvas)
    if (hitId !== null && elements.boxes.has(hitId)) {
      setInteraction({ type: 'editing', boxId: hitId })
      return
    }

    // Create new box at double-click position
    createBoxAtPoint(canvas)
  }, [createBoxAtPoint, elements, viewport])

  const handleBoxPointerDown = useCallback((boxId: string, e: React.PointerEvent) => {
    e.stopPropagation()
    sel.selectElement(boxId, e.shiftKey)
    history.push(elements)
    drag.onDragStart(boxId, e, elements)
  }, [drag, elements, history, sel])

  const handleBoxDoubleClick = useCallback((boxId: string) => {
    setInteraction({ type: 'editing', boxId })
  }, [])

  const handleBoxBlur = useCallback((boxId: string) => {
    if (interaction.type === 'editing' && interaction.boxId === boxId) {
      setInteraction({ type: 'idle' })
    }
  }, [interaction])

  const handleBoxTextChange = useCallback((boxId: string, text: string, width: number, height: number) => {
    setElements((s) => updateBoxText(s, boxId, text, width, height))
  }, [])

  const handleAnchorPointerDown = useCallback((boxId: string, anchor: AnchorPoint, e: React.PointerEvent) => {
    arrowDraw.onArrowStart(boxId, anchor, e)
  }, [arrowDraw])

  const handleResizePointerDown = useCallback((boxId: string, handle: ResizeHandle, e: React.PointerEvent) => {
    history.push(elements)
    resize.onResizeStart(boxId, handle, e, elements)
  }, [elements, history, resize])

  // ── Context menu handlers ──────────────────────────────────────────────

  const handleContextMenu = useCallback((x: number, y: number, elementId: string | null) => {
    setContextMenu({ x, y, elementId })
  }, [])

  const handleContextMenuDelete = useCallback(() => {
    const elementId = contextMenu?.elementId
    if (elementId === null || elementId === undefined) return
    history.push(elements)
    const ids = selection.selectedIds.has(elementId)
      ? selection.selectedIds
      : new Set([elementId])
    setElements((s) => removeElements(s, ids))
    syncDeletedElements(ids)
    setSelection(() => EMPTY_SELECTION)
  }, [contextMenu, elements, history, selection.selectedIds, syncDeletedElements])

  const handleContextMenuSelectAll = useCallback(() => {
    setSelection(() => ({ selectedIds: selectAllIds(elements), marquee: null }))
  }, [elements])

  const closeContextMenu = useCallback(() => {
    setContextMenu(null)
  }, [])

  const handleRootPointerDown = useCallback(() => {
    if (contextMenu !== null) {
      setContextMenu(null)
    }
  }, [contextMenu])

  // ── Render ───────────────────────────────────────────────────────────────

  return (
    <Box onPointerDown={handleRootPointerDown} sx={{ width: '100%', height: '100%', position: 'relative' }}>
      {loading ? (
        <Box sx={{ display: 'flex', alignItems: 'center', justifyContent: 'center', height: '100%' }}>
          <CircularProgress size={32} />
        </Box>
      ) : (
        <Canvas2D
          ref={containerRef}
          elements={elements}
          selection={selection}
          editingBoxId={editingBoxId}
          activeTool={activeTool}
          interaction={interaction}
          viewport={viewport}
          drawingArrow={drawingArrow}
          drawingBox={drawingBox}
          drawingPen={drawingPen}
          canvasBg={theme.canvasBg}
          gridDotColor={theme.gridDotColor}
          connectorColor={theme.connectorColor}
          strokeColor={theme.strokeColor}
          accentColor={theme.accent}
          surfaceBg={theme.surfaceBg}
          textColor={theme.textPrimary}
          onPointerDown={handlePointerDown}
          onPointerMove={handlePointerMove}
          onPointerUp={handlePointerUp}
          onWheel={onWheel}
          onDoubleClick={handleDoubleClick}
          onBoxTextChange={handleBoxTextChange}
          onBoxDoubleClick={handleBoxDoubleClick}
          onBoxBlur={handleBoxBlur}
          onBoxPointerDown={handleBoxPointerDown}
          onAnchorPointerDown={handleAnchorPointerDown}
          onResizePointerDown={handleResizePointerDown}
          onContextMenu={handleContextMenu}
        />
      )}

      <Toolbar
        activeTool={activeTool}
        onToolChange={setActiveTool}
        onZoomIn={zoomIn}
        onZoomOut={zoomOut}
        onResetZoom={resetZoom}
      />

      <SubmitBar
        onSubmit={handleSubmit}
        isSubmitting={isSubmitting}
        status={status}
        error={error}
        onRun={handleRun}
        runStatus={runStatus}
        showDebug={showDebug}
        onToggleDebug={() => setShowDebug((v) => !v)}
      />

      {showDebug && <DispatchPanel onClose={() => setShowDebug(false)} />}

      {contextMenu !== null && (
        <BoardContextMenu
          position={contextMenu}
          onDelete={handleContextMenuDelete}
          onSelectAll={handleContextMenuSelectAll}
          onClose={closeContextMenu}
        />
      )}
    </Box>
  )
}

export { Board }
export type { BoardProps }
