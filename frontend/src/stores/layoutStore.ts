// ============================================================================
// layoutStore — Layout panel state (left nav panel, right context panel)
// ============================================================================

import { createStore, lsGet, lsSet } from './lib'
import {
  LS_LEFT_PANEL_OPEN,
  LS_LEFT_PANEL_SECTION,
  LS_RIGHT_PANEL_WIDTH,
  LS_DISPATCH_PANEL_OPEN,
  LS_DISPATCH_PANEL_WIDTH,
  LS_DISPATCH_PANEL_TAB,
  LAYOUT,
} from '@/constants'

// ── Types ────────────────────────────────────────────────────────────────────

type LayoutState = {
  leftPanelOpen: boolean
  leftPanelSection: string | null
  rightPanelOpen: boolean
  rightPanelSection: string | null
  rightPanelWidth: number
  rightPanelDragging: boolean
  /** Board activity overlay — persisted so a refresh does not close it. */
  dispatchPanelOpen: boolean
  dispatchPanelWidth: number
  dispatchPanelTab: DispatchPanelTab
}

type DispatchPanelTab = 'dispatch' | 'run'

const DISPATCH_PANEL_DEFAULT_WIDTH = 400
const DISPATCH_PANEL_MIN_WIDTH = 300
const DISPATCH_PANEL_MAX_WIDTH = 1200

// ── Store ────────────────────────────────────────────────────────────────────

const parseWidth = (raw: string | null): number => {
  if (!raw) return LAYOUT.PANEL_WIDTH
  const n = Number(raw)
  return Number.isFinite(n) ? Math.max(LAYOUT.PANEL_MIN_WIDTH, Math.min(LAYOUT.PANEL_MAX_WIDTH, n)) : LAYOUT.PANEL_WIDTH
}

const parseDispatchWidth = (raw: string | null): number => {
  if (!raw) return DISPATCH_PANEL_DEFAULT_WIDTH
  const n = Number(raw)
  if (!Number.isFinite(n)) return DISPATCH_PANEL_DEFAULT_WIDTH
  return Math.max(DISPATCH_PANEL_MIN_WIDTH, Math.min(DISPATCH_PANEL_MAX_WIDTH, n))
}

const store = createStore<LayoutState>(() => ({
  leftPanelOpen: lsGet(LS_LEFT_PANEL_OPEN) === 'true',
  leftPanelSection: lsGet(LS_LEFT_PANEL_SECTION),
  rightPanelOpen: false,
  rightPanelSection: null,
  rightPanelWidth: parseWidth(lsGet(LS_RIGHT_PANEL_WIDTH)),
  rightPanelDragging: false,
  dispatchPanelOpen: lsGet(LS_DISPATCH_PANEL_OPEN) === 'true',
  dispatchPanelWidth: parseDispatchWidth(lsGet(LS_DISPATCH_PANEL_WIDTH)),
  dispatchPanelTab: lsGet(LS_DISPATCH_PANEL_TAB) === 'run' ? 'run' : 'dispatch',
}))

// ── Selectors ────────────────────────────────────────────────────────────────

const selectLeftPanelOpen = (s: LayoutState): boolean => s.leftPanelOpen

const selectLeftPanelSection = (s: LayoutState): string | null => s.leftPanelSection

const selectRightPanelOpen = (s: LayoutState): boolean => s.rightPanelOpen

const selectRightPanelSection = (s: LayoutState): string | null => s.rightPanelSection

const selectRightPanelWidth = (s: LayoutState): number => s.rightPanelWidth

const selectRightPanelDragging = (s: LayoutState): boolean => s.rightPanelDragging

// ── Left Panel ──────────────────────────────────────────────────────────────

const openLeftPanel = (section: string): void => {
  store.setState({ leftPanelOpen: true, leftPanelSection: section })
  lsSet(LS_LEFT_PANEL_OPEN, 'true')
  lsSet(LS_LEFT_PANEL_SECTION, section)
}

const closeLeftPanel = (): void => {
  store.setState({ leftPanelOpen: false })
  lsSet(LS_LEFT_PANEL_OPEN, 'false')
}

const toggleLeftPanel = (section: string): void => {
  const s = store.getState()
  if (s.leftPanelOpen && s.leftPanelSection === section) {
    closeLeftPanel()
  } else {
    openLeftPanel(section)
  }
}

// ── Right Panel ─────────────────────────────────────────────────────────────

const openRightPanel = (section: string): void => {
  store.setState({ rightPanelOpen: true, rightPanelSection: section })
}

const openRightPanelIfClosed = (section: string): void => {
  if (!store.getState().rightPanelOpen) {
    openRightPanel(section)
  }
}

const closeRightPanel = (): void => {
  store.setState({ rightPanelOpen: false })
}

const toggleRightPanel = (section: string): void => {
  const s = store.getState()
  if (s.rightPanelOpen && s.rightPanelSection === section) {
    closeRightPanel()
  } else {
    openRightPanel(section)
  }
}

const setRightPanelWidth = (width: number): void => {
  const clamped = Math.max(LAYOUT.PANEL_MIN_WIDTH, Math.min(LAYOUT.PANEL_MAX_WIDTH, width))
  store.setState({ rightPanelWidth: clamped })
  lsSet(LS_RIGHT_PANEL_WIDTH, String(clamped))
}

const startRightPanelDrag = (): void => {
  store.setState({ rightPanelDragging: true })
}

const stopRightPanelDrag = (): void => {
  store.setState({ rightPanelDragging: false })
}

// ── Dispatch panel ───────────────────────────────────────────────────────────

const selectDispatchPanelOpen = (s: LayoutState): boolean => s.dispatchPanelOpen

const selectDispatchPanelWidth = (s: LayoutState): number => s.dispatchPanelWidth

const selectDispatchPanelTab = (s: LayoutState): DispatchPanelTab => s.dispatchPanelTab

const setDispatchPanelOpen = (open: boolean): void => {
  store.setState({ dispatchPanelOpen: open })
  lsSet(LS_DISPATCH_PANEL_OPEN, String(open))
}

const toggleDispatchPanel = (): void => {
  setDispatchPanelOpen(!store.getState().dispatchPanelOpen)
}

const setDispatchPanelWidth = (width: number): void => {
  const clamped = Math.max(DISPATCH_PANEL_MIN_WIDTH, Math.min(DISPATCH_PANEL_MAX_WIDTH, width))
  store.setState({ dispatchPanelWidth: clamped })
  lsSet(LS_DISPATCH_PANEL_WIDTH, String(clamped))
}

const setDispatchPanelTab = (tab: DispatchPanelTab): void => {
  store.setState({ dispatchPanelTab: tab })
  lsSet(LS_DISPATCH_PANEL_TAB, tab)
}

// ── Export ────────────────────────────────────────────────────────────────────

export const layoutStore = {
  store,
  selectLeftPanelOpen,
  selectLeftPanelSection,
  selectRightPanelOpen,
  selectRightPanelSection,
  selectRightPanelWidth,
  selectRightPanelDragging,
  openLeftPanel,
  closeLeftPanel,
  toggleLeftPanel,
  openRightPanel,
  openRightPanelIfClosed,
  closeRightPanel,
  toggleRightPanel,
  setRightPanelWidth,
  startRightPanelDrag,
  stopRightPanelDrag,
  selectDispatchPanelOpen,
  selectDispatchPanelWidth,
  selectDispatchPanelTab,
  setDispatchPanelOpen,
  toggleDispatchPanel,
  setDispatchPanelWidth,
  setDispatchPanelTab,
  DISPATCH_PANEL_MIN_WIDTH,
  DISPATCH_PANEL_MAX_WIDTH,
}

export type { LayoutState, DispatchPanelTab }
