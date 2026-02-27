// ============================================================================
// useKeyboard — Keyboard Shortcuts for the Board
// ============================================================================

import { useCallback, useEffect } from 'react'
import { undoStore } from '@/stores/undoStore'
import type { ActiveTool, BoardElements } from '../elements'
import { removeElements, selectAllIds } from '../elements'
import { EMPTY_SELECTION } from './useSelection'
import type { SetElements, SetInteraction, SetSelection } from './types'

const useKeyboard = (
  elements: BoardElements,
  setElements: SetElements,
  selection: { readonly selectedIds: ReadonlySet<string> },
  setSelection: SetSelection,
  interaction: { readonly type: string },
  setInteraction: SetInteraction,
  onDelete?: (deletedIds: ReadonlySet<string>) => void,
  setActiveTool?: (tool: ActiveTool) => void,
) => {
  const handleKeyDown = useCallback((e: KeyboardEvent) => {
    // Don't intercept when editing text
    if (interaction.type === 'editing') return

    // Delete selected elements
    if ((e.key === 'Delete' || e.key === 'Backspace') && selection.selectedIds.size > 0) {
      e.preventDefault()
      undoStore.push('delete')
      setElements((s) => removeElements(s, selection.selectedIds))
      onDelete?.(selection.selectedIds)
      setSelection(() => EMPTY_SELECTION)
      return
    }

    // Undo
    if (e.key === 'z' && (e.ctrlKey || e.metaKey) && !e.shiftKey) {
      e.preventDefault()
      undoStore.undo()
      return
    }

    // Redo
    if (
      (e.key === 'z' && (e.ctrlKey || e.metaKey) && e.shiftKey) ||
      (e.key === 'y' && (e.ctrlKey || e.metaKey))
    ) {
      e.preventDefault()
      undoStore.redo()
      return
    }

    // Select all
    if (e.key === 'a' && (e.ctrlKey || e.metaKey)) {
      e.preventDefault()
      setSelection(() => ({ selectedIds: selectAllIds(elements), marquee: null }))
      return
    }

    // Tool shortcuts (only when no modifier keys)
    if (setActiveTool !== undefined && !e.ctrlKey && !e.metaKey && !e.altKey) {
      if (e.key === 'v' || e.key === 'V') { setActiveTool('select'); return }
      if (e.key === 'b' || e.key === 'B') { setActiveTool('box'); return }
      if (e.key === 'a' || e.key === 'A') { setActiveTool('arrow'); return }
      if (e.key === 'p' || e.key === 'P') { setActiveTool('pen'); return }
    }

    // Escape — cancel interaction or clear selection
    if (e.key === 'Escape') {
      e.preventDefault()
      if (interaction.type !== 'idle') {
        setInteraction({ type: 'idle' })
      } else if (selection.selectedIds.size > 0) {
        setSelection(() => EMPTY_SELECTION)
      }
      return
    }
  }, [elements, interaction, onDelete, selection, setActiveTool, setElements, setInteraction, setSelection])

  useEffect(() => {
    window.addEventListener('keydown', handleKeyDown)
    return () => { window.removeEventListener('keydown', handleKeyDown) }
  }, [handleKeyDown])
}

export { useKeyboard }
