// ============================================================================
// historyStore — Undo/redo via command pattern
// ============================================================================

import { createStore } from './lib'

// ── Types ────────────────────────────────────────────────────────────────────

type Command = {
  type: string
  description: string
  execute: () => void
  undo: () => void
}

type HistoryState = {
  past: Command[]
  future: Command[]
  maxSize: number
}

// ── Constants ────────────────────────────────────────────────────────────────

const DEFAULT_MAX_SIZE = 50

// ── Store ────────────────────────────────────────────────────────────────────

const store = createStore<HistoryState>(() => ({
  past: [],
  future: [],
  maxSize: DEFAULT_MAX_SIZE,
}))

// ── Selectors ────────────────────────────────────────────────────────────────

const selectCanUndo = (s: HistoryState): boolean => s.past.length > 0

const selectCanRedo = (s: HistoryState): boolean => s.future.length > 0

const selectUndoDescription = (s: HistoryState): string | null => (s.past.length > 0 ? s.past[s.past.length - 1].description : null)

const selectRedoDescription = (s: HistoryState): string | null => (s.future.length > 0 ? s.future[s.future.length - 1].description : null)

// ── Actions ──────────────────────────────────────────────────────────────────

const push = (cmd: Command): void => {
  cmd.execute()
  store.setState((s) => {
    const past = [...s.past, cmd]
    if (past.length > s.maxSize) {
      past.splice(0, past.length - s.maxSize)
    }
    return { past, future: [] }
  })
}

const undo = (): void => {
  const { past } = store.getState()
  if (past.length === 0) return

  const cmd = past[past.length - 1]
  cmd.undo()
  store.setState((s) => ({
    past: s.past.slice(0, -1),
    future: [...s.future, cmd],
  }))
}

const redo = (): void => {
  const { future } = store.getState()
  if (future.length === 0) return

  const cmd = future[future.length - 1]
  cmd.execute()
  store.setState((s) => ({
    past: [...s.past, cmd],
    future: s.future.slice(0, -1),
  }))
}

const clear = (): void => {
  store.setState({ past: [], future: [] })
}

const setMaxSize = (maxSize: number): void => {
  store.setState((s) => {
    const past = s.past.length > maxSize ? s.past.slice(s.past.length - maxSize) : s.past
    return { maxSize, past }
  })
}

// ── Export ────────────────────────────────────────────────────────────────────

export const historyStore = {
  store,
  selectCanUndo,
  selectCanRedo,
  selectUndoDescription,
  selectRedoDescription,
  push,
  undo,
  redo,
  clear,
  setMaxSize,
}

export type { HistoryState, Command }
