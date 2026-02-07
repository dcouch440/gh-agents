// ============================================================================
// layoutStore — Layout panel state (left nav panel, right context panel)
// ============================================================================

import { createStore } from './lib'
import { LS_LEFT_PANEL_OPEN, LS_LEFT_PANEL_SECTION, LS_RIGHT_PANEL_WIDTH, LAYOUT } from '@/constants'

// ── Types ────────────────────────────────────────────────────────────────────

type LayoutState = {
  leftPanelOpen: boolean
  leftPanelSection: string | null
  rightPanelOpen: boolean
  rightPanelSection: string | null
  rightPanelWidth: number
  rightPanelDragging: boolean
}

// ── Safe localStorage ────────────────────────────────────────────────────────

const lsGet = (key: string): string | null => {
  try { return localStorage.getItem(key) } catch { return null }
}

const lsSet = (key: string, value: string): void => {
  try { localStorage.setItem(key, value) } catch { /* noop */ }
}

// ── Store ────────────────────────────────────────────────────────────────────

const parseWidth = (raw: string | null): number => {
  if (!raw) return LAYOUT.PANEL_WIDTH
  const n = Number(raw)
  return Number.isFinite(n) ? Math.max(LAYOUT.PANEL_MIN_WIDTH, Math.min(LAYOUT.PANEL_MAX_WIDTH, n)) : LAYOUT.PANEL_WIDTH
}

const store = createStore<LayoutState>(() => ({
  leftPanelOpen: lsGet(LS_LEFT_PANEL_OPEN) === 'true',
  leftPanelSection: lsGet(LS_LEFT_PANEL_SECTION),
  rightPanelOpen: false,
  rightPanelSection: null,
  rightPanelWidth: parseWidth(lsGet(LS_RIGHT_PANEL_WIDTH)),
  rightPanelDragging: false,
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
  closeRightPanel,
  toggleRightPanel,
  setRightPanelWidth,
  startRightPanelDrag,
  stopRightPanelDrag,
}

export type { LayoutState }
