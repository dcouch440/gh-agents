import { useCallback, useEffect, useRef, useState } from 'react'
import Box from '@mui/material/Box'
import CircularProgress from '@mui/material/CircularProgress'
import { boardStore } from '@/stores'
import { useBoardTheme, useBoardSubmit, useBoardElements, useDispatchHistory } from './hooks'
import { BoardContextMenu } from './BoardContextMenu'
import type { MenuPosition } from './BoardContextMenu'
import { SubmitBar } from './SubmitBar'
import { DebugPanel } from './debug'
import { Canvas2D } from './canvas'
import { Toolbar } from './toolbar'
import { useHistory } from './history'
import { useArrowDraw, useDrag, useKeyboard, usePanZoom, useResize, useSelection, EMPTY_SELECTION } from './interactions'
import type { ActiveTool, AnchorPoint, BoardElements, DrawingArrow, InteractionMode, ResizeHandle, SelectionState, ViewportState } from './elements'
import { addBox, createBox, emptyBoard, hitTest, removeElements, screenToCanvas, updateBoxText } from './elements'

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

  useDispatchHistory(workflowId)

  useEffect(() => {
    return () => { boardStore.resetBoard() }
  }, [workflowId])

  // ── Interaction hooks ────────────────────────────────────────────────────
  const { onWheel, zoomIn, zoomOut, resetZoom } = usePanZoom(viewport, setViewport)
  const drag = useDrag(setElements, setInteraction, viewport, containerRef)
  const arrowDraw = useArrowDraw(setElements, setInteraction, viewport, containerRef)
  const resize = useResize(setElements, setInteraction, viewport, containerRef)
  const sel = useSelection(setSelection)
  useKeyboard(elements, setElements, selection, setSelection, interaction, setInteraction, history)

  // ── Derived state ────────────────────────────────────────────────────────
  const editingBoxId = interaction.type === 'editing' ? interaction.boxId : null
  const drawingArrow: DrawingArrow = interaction.type === 'drawing-arrow'
    ? { sourceBoxId: interaction.sourceBoxId, sourceAnchor: interaction.sourceAnchor, cursorX: interaction.cursorX, cursorY: interaction.cursorY }
    : null

  // ── Event handlers ─────────────────────────────────────────────────────

  const handlePointerDown = useCallback((e: React.PointerEvent) => {
    // Only handle primary button
    if (e.button !== 0) return

    const container = containerRef.current
    if (container === null) return
    const rect = container.getBoundingClientRect()
    const canvas = screenToCanvas(e.clientX, e.clientY, viewport, rect)

    // Box tool: create a new box at click position
    if (activeTool === 'box') {
      history.push(elements)
      const box = createBox(canvas.x - 100, canvas.y - 24)
      setElements((s) => addBox(s, box))
      setInteraction({ type: 'editing', boxId: box.id })
      sel.selectElement(box.id, false)
      setActiveTool('select')
      return
    }

    // Check if we hit something
    const hitId = hitTest(elements, canvas)
    if (hitId !== null) {
      sel.selectElement(hitId, e.shiftKey)
    } else {
      sel.clearSelection()
    }
  }, [activeTool, elements, history, sel, setActiveTool, viewport])

  const handlePointerMove = useCallback((e: React.PointerEvent) => {
    if (interaction.type === 'dragging') {
      drag.onDragMove(e, interaction)
    } else if (interaction.type === 'drawing-arrow') {
      arrowDraw.onArrowMove(e, interaction)
    } else if (interaction.type === 'resizing') {
      resize.onResizeMove(e, interaction)
    }
  }, [arrowDraw, drag, interaction, resize])

  const handlePointerUp = useCallback((e: React.PointerEvent) => {
    if (interaction.type === 'dragging') {
      drag.onDragEnd()
    } else if (interaction.type === 'drawing-arrow') {
      arrowDraw.onArrowEnd(e, interaction, elements)
    } else if (interaction.type === 'resizing') {
      resize.onResizeEnd()
    }
  }, [arrowDraw, drag, elements, interaction, resize])

  const handleDoubleClick = useCallback((e: React.MouseEvent) => {
    const container = containerRef.current
    if (container === null) return
    const rect = container.getBoundingClientRect()
    const canvas = screenToCanvas(e.clientX, e.clientY, viewport, rect)

    // Check if we hit an existing box
    const hitId = hitTest(elements, canvas)
    if (hitId !== null && elements.boxes.has(hitId)) {
      setInteraction({ type: 'editing', boxId: hitId })
      return
    }

    // Create new box at double-click position
    history.push(elements)
    const box = createBox(canvas.x - 100, canvas.y - 24)
    setElements((s) => addBox(s, box))
    setInteraction({ type: 'editing', boxId: box.id })
    sel.selectElement(box.id, false)
  }, [elements, history, sel, viewport])

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
    // If right-clicked element is in selection, delete the whole selection
    const ids = selection.selectedIds.has(elementId)
      ? selection.selectedIds
      : new Set([elementId])
    setElements((s) => removeElements(s, ids))
    setSelection(() => ({ selectedIds: new Set(), marquee: null }))
  }, [contextMenu, elements, history, selection.selectedIds])

  const handleContextMenuSelectAll = useCallback(() => {
    const allIds = new Set<string>()
    for (const id of elements.boxes.keys()) allIds.add(id)
    for (const id of elements.arrows.keys()) allIds.add(id)
    setSelection(() => ({ selectedIds: allIds, marquee: null }))
  }, [elements])

  const closeContextMenu = useCallback(() => {
    setContextMenu(null)
  }, [])

  // Close context menu on any pointer down outside the menu
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
          viewport={viewport}
          drawingArrow={drawingArrow}
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
        showDebug={showDebug}
        onToggleDebug={() => setShowDebug((v) => !v)}
      />

      {showDebug && <DebugPanel onClose={() => setShowDebug(false)} />}

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
