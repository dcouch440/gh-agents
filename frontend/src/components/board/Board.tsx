import { useCallback, useEffect, useMemo, useRef, useState } from 'react'
import Box from '@mui/material/Box'
import CircularProgress from '@mui/material/CircularProgress'
import { api } from '@/api'
import { boardStore, workflowStore, workflowLiveStore, layoutStore, sidebarStore, useStore } from '@/stores'
import { boardElementStore } from '@/stores/boardElementStore'
import { useWorkflowRun } from '@/components/canvas/useWorkflowRun'
import { useBoardTheme, useBoardSubmit, useBoardElements, useCanvasSync, useActivityHistory } from './hooks'
import { BoardContextMenu } from './BoardContextMenu'
import type { MenuPosition } from './BoardContextMenu'
import { SubmitBar } from './SubmitBar'
import { DispatchPanel } from './dispatch'
import { Canvas2D } from './canvas'
import { Toolbar } from './toolbar'
import { useBoardInteractions } from './interactions'

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
  const elements = useStore(boardElementStore.store, boardElementStore.selectElements)
  const setElements = boardElementStore.setElements
  // Persisted: reopening the editor should not silently hide the activity panel.
  const showDebug = useStore(layoutStore.store, layoutStore.selectDispatchPanelOpen)
  const [contextMenu, setContextMenu] = useState<MenuPosition | null>(null)
  const containerRef = useRef<HTMLDivElement>(null)

  // ── Data hooks ────────────────────────────────────────────────────────────
  const theme = useBoardTheme()
  const { loading } = useBoardElements(workflowId)
  useBoardSubmit(workflowId) // kept for initial element load
  // Server truth, not local state — a refresh mid-generation still reads as
  // generating, and the flag clears when the server says the work is done.
  const isGenerating = useStore(workflowLiveStore.store, workflowLiveStore.selectIsGenerating)
  const handleGenerate = useCallback(() => {
    // Optimistic until the sync tick confirms it. Deliberately no re-fetch on
    // success: `POST /generate` returns before its pipeline has registered
    // anything, so reading straight back reports "not generating" for work that
    // is about to start. The poll, re-armed by `setGenerating`, settles it.
    workflowLiveStore.setGenerating(true)
    void api.workflows.generate(workflowId)
      .catch((err: unknown) => {
        console.error('Generate failed:', err)
        // Nothing is going to start, so drop the spinner now rather than making
        // the user wait out the confirmation grace.
        workflowLiveStore.setGenerating(false)
        void workflowLiveStore.hydrateActive()
      })
  }, [workflowId])

  const steps = useStore(workflowStore.store, workflowStore.selectSteps)
  const entryStep = useMemo(() => {
    const inputStep = steps.find((s) => s.execution_mode === 'input')
    if (inputStep) return inputStep
    return steps.find((s) => s.execution_mode === 'context') ?? null
  }, [steps])
  const { status: runStatus, handleRun } = useWorkflowRun(entryStep?.prompt_template ?? '')

  useActivityHistory(workflowId)

  useEffect(() => {
    return () => { boardStore.resetBoard() }
  }, [workflowId])

  // ── Helpers ──────────────────────────────────────────────────────────────

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

  const handleContextMenuOpen = useCallback((x: number, y: number, elementId: string | null) => {
    setContextMenu({ x, y, elementId })
  }, [])

  // ── Live sync ──────────────────────────────────────────────────────────
  const handleCanvasChange = useCanvasSync(workflowId)

  // ── Interactions ─────────────────────────────────────────────────────────

  const {
    interaction, selection, viewport, activeTool,
    editingBoxId, previews, handlers,
    setActiveTool, zoomIn, zoomOut, resetZoom,
    handleContextMenuDelete, handleContextMenuSelectAll,
  } = useBoardInteractions(elements, setElements, containerRef, syncDeletedElements, handleContextMenuOpen, handleCanvasChange)

  // ── Context menu ─────────────────────────────────────────────────────────

  const closeContextMenu = useCallback(() => { setContextMenu(null) }, [])

  const handleRootPointerDown = useCallback(() => {
    if (contextMenu !== null) { setContextMenu(null) }
  }, [contextMenu])

  const onContextMenuDelete = useCallback(() => {
    handleContextMenuDelete(contextMenu?.elementId)
  }, [contextMenu, handleContextMenuDelete])

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
          theme={theme}
          previews={previews}
          {...handlers}
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
        onGenerate={handleGenerate}
        isGenerating={isGenerating}
        onRun={handleRun}
        runStatus={runStatus}
        showDebug={showDebug}
        onToggleDebug={() => { layoutStore.toggleDispatchPanel() }}
      />

      {showDebug && <DispatchPanel onClose={() => { layoutStore.setDispatchPanelOpen(false) }} />}

      {contextMenu !== null && (
        <BoardContextMenu
          position={contextMenu}
          onDelete={onContextMenuDelete}
          onSelectAll={handleContextMenuSelectAll}
          onClose={closeContextMenu}
        />
      )}
    </Box>
  )
}

export { Board }
export type { BoardProps }
