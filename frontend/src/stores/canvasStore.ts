// ============================================================================
// canvasStore — Workflow editor canvas interaction state
// ============================================================================

import { createStore, logger } from './lib'

// ── Types ────────────────────────────────────────────────────────────────────

type PanelKind = 'closed' | 'step-config' | 'edge-config' | 'step-output'

type InteractionMode = 'select' | 'connect' | 'pan'

type DragItem = {
  type: 'step'
  stepId: string
  startX: number
  startY: number
}

type CanvasState = {
  selectedStepIds: ReadonlySet<string>
  selectedEdgeIds: ReadonlySet<string>
  hoveredStepId: string | null
  hoveredEdgeId: string | null
  panel: PanelKind
  panelTargetId: string | null
  interactionMode: InteractionMode
  dragItem: DragItem | null
  minimapVisible: boolean
}

// ── Constants ────────────────────────────────────────────────────────────────

const EMPTY_SET: ReadonlySet<string> = new Set()

// ── Store ────────────────────────────────────────────────────────────────────

const store = logger('canvasStore', createStore<CanvasState>(() => ({
  selectedStepIds: EMPTY_SET,
  selectedEdgeIds: EMPTY_SET,
  hoveredStepId: null,
  hoveredEdgeId: null,
  panel: 'closed',
  panelTargetId: null,
  interactionMode: 'select',
  dragItem: null,
  minimapVisible: false,
})))

// ── Selectors ────────────────────────────────────────────────────────────────

const selectSelectedStepIds = (s: CanvasState): ReadonlySet<string> => s.selectedStepIds

const selectSelectedEdgeIds = (s: CanvasState): ReadonlySet<string> => s.selectedEdgeIds

const selectHoveredStepId = (s: CanvasState): string | null => s.hoveredStepId

const selectHoveredEdgeId = (s: CanvasState): string | null => s.hoveredEdgeId

const selectPanel = (s: CanvasState): PanelKind => s.panel

const selectPanelTargetId = (s: CanvasState): string | null => s.panelTargetId

const selectInteractionMode = (s: CanvasState): InteractionMode => s.interactionMode

const selectMinimapVisible = (s: CanvasState): boolean => s.minimapVisible

const selectHasSelection = (s: CanvasState): boolean =>
  s.selectedStepIds.size > 0 || s.selectedEdgeIds.size > 0

// ── Selection ────────────────────────────────────────────────────────────────

const selectSteps = (ids: string[]): void => {
  const current = store.getState().selectedStepIds
  if (ids.length === current.size && ids.every((id) => current.has(id))) return
  store.setState({ selectedStepIds: new Set(ids) })
}

const selectEdges = (ids: string[]): void => {
  const current = store.getState().selectedEdgeIds
  if (ids.length === current.size && ids.every((id) => current.has(id))) return
  store.setState({ selectedEdgeIds: new Set(ids) })
}

const addToSelection = (stepId: string): void => {
  store.setState((s) => {
    const next = new Set(s.selectedStepIds)
    if (next.has(stepId)) {
      next.delete(stepId)
    } else {
      next.add(stepId)
    }
    return { selectedStepIds: next }
  })
}

const clearSelection = (): void => {
  store.setState({ selectedStepIds: EMPTY_SET, selectedEdgeIds: EMPTY_SET })
}

// ── Hover ────────────────────────────────────────────────────────────────────

const setHoveredStep = (id: string | null): void => {
  store.setState({ hoveredStepId: id })
}

const setHoveredEdge = (id: string | null): void => {
  store.setState({ hoveredEdgeId: id })
}

// ── Panel ────────────────────────────────────────────────────────────────────

const openPanel = (kind: PanelKind, targetId: string): void => {
  store.setState({ panel: kind, panelTargetId: targetId })
}

const closePanel = (): void => {
  store.setState({ panel: 'closed', panelTargetId: null })
}

// ── Interaction ──────────────────────────────────────────────────────────────

const setInteractionMode = (mode: InteractionMode): void => {
  store.setState({ interactionMode: mode })
}

// ── Drag ─────────────────────────────────────────────────────────────────────

const setDragItem = (item: DragItem | null): void => {
  store.setState({ dragItem: item })
}

// ── Minimap ──────────────────────────────────────────────────────────────────

const toggleMinimap = (): void => {
  store.setState((s) => ({ minimapVisible: !s.minimapVisible }))
}

// ── Reset ────────────────────────────────────────────────────────────────────

const reset = (): void => {
  store.setState({
    selectedStepIds: EMPTY_SET,
    selectedEdgeIds: EMPTY_SET,
    hoveredStepId: null,
    hoveredEdgeId: null,
    panel: 'closed',
    panelTargetId: null,
    interactionMode: 'select',
    dragItem: null,
    minimapVisible: false,
  })
}

// ── Export ────────────────────────────────────────────────────────────────────

export const canvasStore = {
  store,
  selectSelectedStepIds,
  selectSelectedEdgeIds,
  selectHoveredStepId,
  selectHoveredEdgeId,
  selectPanel,
  selectPanelTargetId,
  selectInteractionMode,
  selectMinimapVisible,
  selectHasSelection,
  selectSteps,
  selectEdges,
  addToSelection,
  clearSelection,
  setHoveredStep,
  setHoveredEdge,
  openPanel,
  closePanel,
  setInteractionMode,
  setDragItem,
  toggleMinimap,
  reset,
}

export type { CanvasState, PanelKind, InteractionMode, DragItem }
