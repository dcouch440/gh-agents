// ============================================================================
// sidebarStore — Workflow editor sidebar state (tree/chat tab, step selection, width)
// ============================================================================

import { createStore, lsGet, lsSet } from './lib'

// ── Constants ───────────────────────────────────────────────────────────────

const LS_SIDEBAR_WIDTH = 'nexor_sidebar_width'
const DEFAULT_WIDTH = 320
const MIN_WIDTH = 240
const MAX_WIDTH = 480

// ── Types ───────────────────────────────────────────────────────────────────

type SidebarTab = 'tree' | 'chat'

type SidebarState = {
  activeTab: SidebarTab
  selectedStepId: string | null
  width: number
  dragging: boolean
}

// ── Store ───────────────────────────────────────────────────────────────────

const parseWidth = (raw: string | null): number => {
  if (!raw) return DEFAULT_WIDTH
  const n = Number(raw)
  return Number.isFinite(n) ? Math.max(MIN_WIDTH, Math.min(MAX_WIDTH, n)) : DEFAULT_WIDTH
}

const store = createStore<SidebarState>(() => ({
  activeTab: 'tree',
  selectedStepId: null,
  width: parseWidth(lsGet(LS_SIDEBAR_WIDTH)),
  dragging: false,
}))

// ── Selectors ───────────────────────────────────────────────────────────────

const selectActiveTab = (s: SidebarState): SidebarTab => s.activeTab

const selectSelectedStepId = (s: SidebarState): string | null => s.selectedStepId

const selectWidth = (s: SidebarState): number => s.width

const selectDragging = (s: SidebarState): boolean => s.dragging

// ── Actions ─────────────────────────────────────────────────────────────────

const setActiveTab = (tab: SidebarTab): void => {
  store.setState({ activeTab: tab })
}

const selectStep = (stepId: string): void => {
  store.setState({ selectedStepId: stepId, activeTab: 'tree' })
}

const clearSelection = (): void => {
  store.setState({ selectedStepId: null })
}

const setWidth = (width: number): void => {
  const clamped = Math.max(MIN_WIDTH, Math.min(MAX_WIDTH, width))
  store.setState({ width: clamped })
  lsSet(LS_SIDEBAR_WIDTH, String(clamped))
}

const startDrag = (): void => {
  store.setState({ dragging: true })
}

const stopDrag = (): void => {
  store.setState({ dragging: false })
}

const reset = (): void => {
  store.setState({ activeTab: 'tree', selectedStepId: null, dragging: false })
}

// ── Export ───────────────────────────────────────────────────────────────────

export const sidebarStore = {
  store,
  selectActiveTab,
  selectSelectedStepId,
  selectWidth,
  selectDragging,
  setActiveTab,
  selectStep,
  clearSelection,
  setWidth,
  startDrag,
  stopDrag,
  reset,
  MIN_WIDTH,
  MAX_WIDTH,
}

export type { SidebarState, SidebarTab }
