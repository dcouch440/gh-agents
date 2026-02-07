// ============================================================================
// layoutStore — Layout panel state (left nav panel, right context panel)
// ============================================================================

import { createStore } from './lib'
import { LS_LEFT_PANEL_OPEN, LS_LEFT_PANEL_SECTION } from '@/constants'

// ── Types ────────────────────────────────────────────────────────────────────

type LayoutState = {
  leftPanelOpen: boolean
  leftPanelSection: string | null
  rightPanelOpen: boolean
  rightPanelSection: string | null
}

// ── Safe localStorage ────────────────────────────────────────────────────────

const lsGet = (key: string): string | null => {
  try { return localStorage.getItem(key) } catch { return null }
}

const lsSet = (key: string, value: string): void => {
  try { localStorage.setItem(key, value) } catch { /* noop */ }
}

// ── Store ────────────────────────────────────────────────────────────────────

const store = createStore<LayoutState>(() => ({
  leftPanelOpen: lsGet(LS_LEFT_PANEL_OPEN) === 'true',
  leftPanelSection: lsGet(LS_LEFT_PANEL_SECTION),
  rightPanelOpen: false,
  rightPanelSection: null,
}))

// ── Selectors ────────────────────────────────────────────────────────────────

const selectLeftPanelOpen = (s: LayoutState): boolean => s.leftPanelOpen

const selectLeftPanelSection = (s: LayoutState): string | null => s.leftPanelSection

const selectRightPanelOpen = (s: LayoutState): boolean => s.rightPanelOpen

const selectRightPanelSection = (s: LayoutState): string | null => s.rightPanelSection

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

// ── Export ────────────────────────────────────────────────────────────────────

export const layoutStore = {
  store,
  selectLeftPanelOpen,
  selectLeftPanelSection,
  selectRightPanelOpen,
  selectRightPanelSection,
  openLeftPanel,
  closeLeftPanel,
  toggleLeftPanel,
  openRightPanel,
  closeRightPanel,
  toggleRightPanel,
}

export type { LayoutState }
