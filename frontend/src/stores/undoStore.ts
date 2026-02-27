// ============================================================================
// undoStore — Snapshot-Based Undo/Redo with Transaction Support
// ============================================================================
//
// Captures BoardElements snapshots before mutations. Transactions collapse
// multi-frame operations (drag, resize, text editing) into single undo units.
//
// Usage:
//   undoStore.push('create-box')           // before a discrete mutation
//   undoStore.beginTransaction('move')     // before drag/resize start
//   undoStore.commit()                     // on drag/resize end
//   undoStore.undo() / undoStore.redo()    // Cmd+Z / Cmd+Shift+Z

import { createStore } from './lib'
import { boardElementStore } from './boardElementStore'
import type { BoardElements } from '@/components/board/elements'

// ── Types ────────────────────────────────────────────────────────────────────

type UndoEntry = {
  readonly snapshot: BoardElements
  readonly tag: string
}

type UndoState = {
  readonly past: readonly UndoEntry[]
  readonly future: readonly UndoEntry[]
  readonly transactionDepth: number
  readonly pendingEntry: UndoEntry | null
}

// ── Constants ────────────────────────────────────────────────────────────────

const MAX_DEPTH = 100

// ── Store ────────────────────────────────────────────────────────────────────

const store = createStore<UndoState>(() => ({
  past: [],
  future: [],
  transactionDepth: 0,
  pendingEntry: null,
}))

// ── Helpers ──────────────────────────────────────────────────────────────────

const trimPast = (past: readonly UndoEntry[]): readonly UndoEntry[] =>
  past.length > MAX_DEPTH ? past.slice(past.length - MAX_DEPTH) : past

// ── Actions ──────────────────────────────────────────────────────────────────

/**
 * Record the current state BEFORE a discrete mutation.
 * No-op if inside a transaction (the transaction already captured the snapshot).
 */
const push = (tag: string): void => {
  const { transactionDepth } = store.getState()
  if (transactionDepth > 0) return

  const snapshot = boardElementStore.getElements()
  store.setState((s) => ({
    past: trimPast([...s.past, { snapshot, tag }]),
    future: [],
  }))
}

/**
 * Undo: restore the most recent snapshot, pushing current state to future.
 */
const undo = (): void => {
  const { past } = store.getState()
  if (past.length === 0) return

  const entry = past[past.length - 1]
  const current = boardElementStore.getElements()

  store.setState((s) => ({
    past: s.past.slice(0, -1),
    future: [...s.future, { snapshot: current, tag: entry.tag }],
  }))

  boardElementStore.replaceElements(entry.snapshot)
}

/**
 * Redo: restore the most recent future snapshot, pushing current state to past.
 */
const redo = (): void => {
  const { future } = store.getState()
  if (future.length === 0) return

  const entry = future[future.length - 1]
  const current = boardElementStore.getElements()

  store.setState((s) => ({
    past: [...s.past, { snapshot: current, tag: entry.tag }],
    future: s.future.slice(0, -1),
  }))

  boardElementStore.replaceElements(entry.snapshot)
}

/**
 * Begin a transaction. Captures a snapshot now. All mutations until commit()
 * are collapsed into a single undo unit. Nested transactions are ref-counted.
 */
const beginTransaction = (tag: string): void => {
  const { transactionDepth } = store.getState()
  if (transactionDepth === 0) {
    const snapshot = boardElementStore.getElements()
    store.setState({ transactionDepth: 1, pendingEntry: { snapshot, tag } })
  } else {
    store.setState((s) => ({ transactionDepth: s.transactionDepth + 1 }))
  }
}

/**
 * Commit the transaction. Pushes the snapshot to past only if state changed.
 */
const commit = (): void => {
  const { transactionDepth, pendingEntry } = store.getState()
  if (transactionDepth <= 0) return

  if (transactionDepth === 1) {
    if (pendingEntry !== null) {
      const current = boardElementStore.getElements()
      const changed = current !== pendingEntry.snapshot

      if (changed) {
        store.setState((s) => ({
          past: trimPast([...s.past, pendingEntry]),
          future: [],
          transactionDepth: 0,
          pendingEntry: null,
        }))
      } else {
        store.setState({ transactionDepth: 0, pendingEntry: null })
      }
    } else {
      store.setState({ transactionDepth: 0 })
    }
  } else {
    store.setState((s) => ({ transactionDepth: s.transactionDepth - 1 }))
  }
}

/**
 * Abort transaction — restore the captured snapshot and discard.
 */
const rollback = (): void => {
  const { transactionDepth, pendingEntry } = store.getState()
  if (transactionDepth <= 0) return

  if (pendingEntry !== null) {
    boardElementStore.replaceElements(pendingEntry.snapshot)
  }
  store.setState({ transactionDepth: 0, pendingEntry: null })
}

/** Clear both stacks (e.g. after board submit). */
const clear = (): void => {
  store.setState({ past: [], future: [], transactionDepth: 0, pendingEntry: null })
}

// ── Selectors ────────────────────────────────────────────────────────────────

const selectCanUndo = (s: UndoState): boolean => s.past.length > 0

const selectCanRedo = (s: UndoState): boolean => s.future.length > 0

const selectUndoTag = (s: UndoState): string | null =>
  s.past.length > 0 ? s.past[s.past.length - 1].tag : null

const selectRedoTag = (s: UndoState): string | null =>
  s.future.length > 0 ? s.future[s.future.length - 1].tag : null

// ── Export ────────────────────────────────────────────────────────────────────

export const undoStore = {
  store,
  push,
  undo,
  redo,
  beginTransaction,
  commit,
  rollback,
  clear,
  selectCanUndo,
  selectCanRedo,
  selectUndoTag,
  selectRedoTag,
}

export type { UndoState, UndoEntry }
