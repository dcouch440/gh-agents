// ============================================================================
// useKeyboard — Keyboard Shortcuts for the Board
// ============================================================================

import { useCallback, useEffect } from 'react'
import type { BoardElements, InteractionMode, SelectionState } from '../elements'
import { removeElements } from '../elements'

type SetElements = (fn: (s: BoardElements) => BoardElements) => void
type SetSelection = (fn: (s: SelectionState) => SelectionState) => void
type SetInteraction = (mode: InteractionMode) => void
type HistoryActions = {
  readonly undo: () => BoardElements | null
  readonly redo: () => BoardElements | null
}

const useKeyboard = (
  elements: BoardElements,
  setElements: SetElements,
  selection: SelectionState,
  setSelection: SetSelection,
  interaction: InteractionMode,
  setInteraction: SetInteraction,
  history: HistoryActions,
) => {
  const handleKeyDown = useCallback((e: KeyboardEvent) => {
    // Don't intercept when editing text
    if (interaction.type === 'editing') return

    // Delete selected elements
    if ((e.key === 'Delete' || e.key === 'Backspace') && selection.selectedIds.size > 0) {
      e.preventDefault()
      setElements((s) => removeElements(s, selection.selectedIds))
      setSelection(() => ({ selectedIds: new Set(), marquee: null }))
      return
    }

    // Undo
    if (e.key === 'z' && (e.ctrlKey || e.metaKey) && !e.shiftKey) {
      e.preventDefault()
      const prev = history.undo()
      if (prev !== null) {
        setElements(() => prev)
      }
      return
    }

    // Redo
    if (
      (e.key === 'z' && (e.ctrlKey || e.metaKey) && e.shiftKey) ||
      (e.key === 'y' && (e.ctrlKey || e.metaKey))
    ) {
      e.preventDefault()
      const next = history.redo()
      if (next !== null) {
        setElements(() => next)
      }
      return
    }

    // Select all
    if (e.key === 'a' && (e.ctrlKey || e.metaKey)) {
      e.preventDefault()
      const allIds = new Set<string>()
      for (const id of elements.boxes.keys()) allIds.add(id)
      for (const id of elements.arrows.keys()) allIds.add(id)
      setSelection(() => ({ selectedIds: allIds, marquee: null }))
      return
    }

    // Escape — cancel interaction or clear selection
    if (e.key === 'Escape') {
      e.preventDefault()
      if (interaction.type !== 'idle') {
        setInteraction({ type: 'idle' })
      } else if (selection.selectedIds.size > 0) {
        setSelection(() => ({ selectedIds: new Set(), marquee: null }))
      }
      return
    }
  }, [elements, history, interaction, selection, setElements, setInteraction, setSelection])

  useEffect(() => {
    window.addEventListener('keydown', handleKeyDown)
    return () => { window.removeEventListener('keydown', handleKeyDown) }
  }, [handleKeyDown])
}

export { useKeyboard }
