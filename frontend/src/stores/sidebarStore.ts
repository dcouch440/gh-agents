// ============================================================================
// sidebarStore — Workflow editor sidebar state (tree/chat tab, step selection, width)
// ============================================================================

import { createStore, lsGet, lsSet } from './lib'

// ── Constants ───────────────────────────────────────────────────────────────

const LS_SIDEBAR_WIDTH = 'nexor_sidebar_width'
const DEFAULT_WIDTH = 320
const MIN_WIDTH = 240
const MAX_WIDTH = 800

// ── Types ───────────────────────────────────────────────────────────────────

type SidebarTab = 'tree' | 'chat'

type SidebarState = {
  activeTab: SidebarTab
  selectedStepId: string | null
  expandedStepIds: Record<string, boolean>
  outputExpandedStepIds: Record<string, boolean>
  expandedAgentKeys: Record<string, boolean>
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
  expandedStepIds: {},
  outputExpandedStepIds: {},
  expandedAgentKeys: {},
  width: parseWidth(lsGet(LS_SIDEBAR_WIDTH)),
  dragging: false,
}))

// ── Selectors ───────────────────────────────────────────────────────────────

const selectActiveTab = (s: SidebarState): SidebarTab => s.activeTab

const selectSelectedStepId = (s: SidebarState): string | null => s.selectedStepId

const selectWidth = (s: SidebarState): number => s.width

const selectDragging = (s: SidebarState): boolean => s.dragging

const selectExpandedStepIds = (s: SidebarState): Record<string, boolean> => s.expandedStepIds

const selectOutputExpandedStepIds = (s: SidebarState): Record<string, boolean> => s.outputExpandedStepIds

const selectExpandedAgentKeys = (s: SidebarState): Record<string, boolean> => s.expandedAgentKeys

const selectIsExpanded = (id: string) => (s: SidebarState): boolean => s.expandedStepIds[id] === true

const selectIsOutputExpanded = (id: string) => (s: SidebarState): boolean => s.outputExpandedStepIds[id] === true

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

const toggleStep = (id: string): void => {
  const { expandedStepIds } = store.getState()
  store.setState({
    expandedStepIds: { ...expandedStepIds, [id]: !expandedStepIds[id] },
  })
}

const expandStep = (id: string): void => {
  const { expandedStepIds } = store.getState()
  if (expandedStepIds[id]) return
  store.setState({ expandedStepIds: { ...expandedStepIds, [id]: true } })
}

const collapseStep = (id: string): void => {
  const { expandedStepIds } = store.getState()
  if (!expandedStepIds[id]) return
  store.setState({ expandedStepIds: { ...expandedStepIds, [id]: false } })
}

const toggleOutputExpand = (id: string): void => {
  const { outputExpandedStepIds } = store.getState()
  store.setState({
    outputExpandedStepIds: { ...outputExpandedStepIds, [id]: !outputExpandedStepIds[id] },
  })
}

const toggleAgent = (key: string): void => {
  const { expandedAgentKeys } = store.getState()
  store.setState({
    expandedAgentKeys: { ...expandedAgentKeys, [key]: !expandedAgentKeys[key] },
  })
}

const expandAgent = (key: string): void => {
  const { expandedAgentKeys } = store.getState()
  if (expandedAgentKeys[key]) return
  store.setState({ expandedAgentKeys: { ...expandedAgentKeys, [key]: true } })
}

const reset = (): void => {
  store.setState({
    activeTab: 'tree',
    selectedStepId: null,
    expandedStepIds: {},
    outputExpandedStepIds: {},
    expandedAgentKeys: {},
    dragging: false,
  })
}

// ── Export ───────────────────────────────────────────────────────────────────

export const sidebarStore = {
  store,
  selectActiveTab,
  selectSelectedStepId,
  selectExpandedStepIds,
  selectOutputExpandedStepIds,
  selectExpandedAgentKeys,
  selectIsExpanded,
  selectIsOutputExpanded,
  selectWidth,
  selectDragging,
  setActiveTab,
  selectStep,
  clearSelection,
  toggleStep,
  expandStep,
  collapseStep,
  toggleOutputExpand,
  toggleAgent,
  expandAgent,
  setWidth,
  startDrag,
  stopDrag,
  reset,
  MIN_WIDTH,
  MAX_WIDTH,
}

export type { SidebarState, SidebarTab }
