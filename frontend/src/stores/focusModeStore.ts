// ============================================================================
// focusModeStore — Immersive focus mode for navigating workflow nodes
// ============================================================================

import { createStore, logger } from './lib'

// ── Types ────────────────────────────────────────────────────────────────────

type ArtifactKind = 'document' | 'roster-agent' | 'room-member' | 'task-force' | 'room'

type FocusModeState = {
  active: boolean
  orderedStepIds: ReadonlyArray<string>
  currentIndex: number
  expandedArtifactId: string | null
  expandedArtifactKind: ArtifactKind | null
  activeTabId: string
  slideDirection: 'left' | 'right' | 'none'
}

// ── Constants ────────────────────────────────────────────────────────────────

const INITIAL_STATE: FocusModeState = {
  active: false,
  orderedStepIds: [],
  currentIndex: 0,
  expandedArtifactId: null,
  expandedArtifactKind: null,
  activeTabId: 'chat',
  slideDirection: 'none',
}

// ── Store ────────────────────────────────────────────────────────────────────

const store = logger(
  'focusModeStore',
  createStore<FocusModeState>(() => ({ ...INITIAL_STATE })),
)

// ── Selectors ────────────────────────────────────────────────────────────────

const selectActive = (s: FocusModeState): boolean => s.active

const selectCurrentIndex = (s: FocusModeState): number => s.currentIndex

const selectOrderedStepIds = (s: FocusModeState): ReadonlyArray<string> =>
  s.orderedStepIds

const selectCurrentStepId = (s: FocusModeState): string | null =>
  s.orderedStepIds[s.currentIndex] ?? null

const selectExpandedArtifactId = (s: FocusModeState): string | null =>
  s.expandedArtifactId

const selectExpandedArtifactKind = (s: FocusModeState): ArtifactKind | null =>
  s.expandedArtifactKind

const selectActiveTabId = (s: FocusModeState): string => s.activeTabId

const selectSlideDirection = (s: FocusModeState): 'left' | 'right' | 'none' =>
  s.slideDirection

const selectStepCount = (s: FocusModeState): number => s.orderedStepIds.length

// ── Actions ──────────────────────────────────────────────────────────────────

const enter = (orderedStepIds: string[], initialStepId?: string): void => {
  if (orderedStepIds.length === 0) return
  const idx = initialStepId
    ? Math.max(0, orderedStepIds.indexOf(initialStepId))
    : 0
  store.setState({
    active: true,
    orderedStepIds,
    currentIndex: idx === -1 ? 0 : idx,
    expandedArtifactId: null,
    expandedArtifactKind: null,
    activeTabId: 'chat',
    slideDirection: 'none',
  })
}

const exit = (): void => {
  store.setState({ ...INITIAL_STATE })
}

const goNext = (): void => {
  const { currentIndex, orderedStepIds } = store.getState()
  if (currentIndex < orderedStepIds.length - 1) {
    store.setState({
      currentIndex: currentIndex + 1,
      slideDirection: 'left',
      expandedArtifactId: null,
      expandedArtifactKind: null,
      activeTabId: 'chat',
    })
  }
}

const goPrev = (): void => {
  const { currentIndex } = store.getState()
  if (currentIndex > 0) {
    store.setState({
      currentIndex: currentIndex - 1,
      slideDirection: 'right',
      expandedArtifactId: null,
      expandedArtifactKind: null,
      activeTabId: 'chat',
    })
  }
}

const goToIndex = (index: number): void => {
  const { currentIndex, orderedStepIds } = store.getState()
  if (index >= 0 && index < orderedStepIds.length && index !== currentIndex) {
    store.setState({
      currentIndex: index,
      slideDirection: index > currentIndex ? 'left' : 'right',
      expandedArtifactId: null,
      expandedArtifactKind: null,
      activeTabId: 'chat',
    })
  }
}

const expandArtifact = (id: string, kind: ArtifactKind): void => {
  store.setState({ expandedArtifactId: id, expandedArtifactKind: kind })
}

const collapseArtifact = (): void => {
  store.setState({ expandedArtifactId: null, expandedArtifactKind: null })
}

const setActiveTab = (tabId: string): void => {
  store.setState({ activeTabId: tabId })
}

// ── Export ────────────────────────────────────────────────────────────────────

export const focusModeStore = {
  store,
  selectActive,
  selectCurrentIndex,
  selectOrderedStepIds,
  selectCurrentStepId,
  selectExpandedArtifactId,
  selectExpandedArtifactKind,
  selectActiveTabId,
  selectSlideDirection,
  selectStepCount,
  enter,
  exit,
  goNext,
  goPrev,
  goToIndex,
  expandArtifact,
  collapseArtifact,
  setActiveTab,
}

export type { FocusModeState, ArtifactKind }
