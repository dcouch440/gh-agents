// ============================================================================
// boardElementStore — External Store for Board Canvas Elements
// ============================================================================

import { createStore } from './lib'
import type { BoardElements } from '@/components/board/elements'
import { emptyBoard } from '@/components/board/elements'

// ── Types ────────────────────────────────────────────────────────────────────

type BoardElementState = {
  readonly elements: BoardElements
}

// ── Store ────────────────────────────────────────────────────────────────────

const store = createStore<BoardElementState>(() => ({
  elements: emptyBoard,
}))

// ── Selectors ────────────────────────────────────────────────────────────────

const selectElements = (s: BoardElementState): BoardElements => s.elements

// ── Actions ──────────────────────────────────────────────────────────────────

/** Apply an updater function to the current elements (same signature as the old React setState). */
const setElements = (fn: (s: BoardElements) => BoardElements): void => {
  store.setState((state) => {
    const next = fn(state.elements)
    if (next === state.elements) return {}
    return { elements: next }
  })
}

/** Replace elements wholesale (used by initial load and undo/redo restore). */
const replaceElements = (elements: BoardElements): void => {
  store.setState({ elements })
}

const getElements = (): BoardElements => store.getState().elements

// ── Export ────────────────────────────────────────────────────────────────────

export const boardElementStore = {
  store,
  selectElements,
  setElements,
  replaceElements,
  getElements,
}

export type { BoardElementState }
