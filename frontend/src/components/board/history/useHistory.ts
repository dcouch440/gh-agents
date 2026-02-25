// ============================================================================
// useHistory — Undo/Redo Stack for Board Elements
// ============================================================================

import { useCallback, useRef } from 'react'
import { BOARD } from '../constants'
import type { BoardElements } from '../elements'

type HistoryActions = {
  readonly push: (state: BoardElements) => void
  readonly undo: () => BoardElements | null
  readonly redo: () => BoardElements | null
}

/**
 * Simple undo/redo stack storing full snapshots of BoardElements.
 *
 * `push` records the state BEFORE a mutation. `undo` pops from the undo
 * stack and pushes the current state to redo. `redo` does the reverse.
 *
 * New pushes clear the redo stack.
 */
const useHistory = (currentElements: BoardElements): HistoryActions => {
  const undoStack = useRef<BoardElements[]>([])
  const redoStack = useRef<BoardElements[]>([])

  const push = useCallback((state: BoardElements) => {
    undoStack.current.push(state)
    if (undoStack.current.length > BOARD.HISTORY_MAX_DEPTH) {
      undoStack.current.shift()
    }
    redoStack.current = []
  }, [])

  const undo = useCallback((): BoardElements | null => {
    const prev = undoStack.current.pop()
    if (prev === undefined) return null

    redoStack.current.push(currentElements)
    return prev
  }, [currentElements])

  const redo = useCallback((): BoardElements | null => {
    const next = redoStack.current.pop()
    if (next === undefined) return null

    undoStack.current.push(currentElements)
    return next
  }, [currentElements])

  return { push, undo, redo }
}

export { useHistory }
export type { HistoryActions }
