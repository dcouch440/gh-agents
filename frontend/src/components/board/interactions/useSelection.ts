// ============================================================================
// useSelection — Element Selection Interaction
// ============================================================================

import { useCallback } from 'react'
import type { SelectionState } from '../elements'

type SetSelection = (fn: (s: SelectionState) => SelectionState) => void

const EMPTY_SELECTION: SelectionState = {
  selectedIds: new Set(),
  marquee: null,
}

const useSelection = (
  setSelection: SetSelection,
) => {
  const selectElement = useCallback((elementId: string, additive: boolean) => {
    setSelection((s) => {
      if (additive) {
        const ids = new Set(s.selectedIds)
        if (ids.has(elementId)) {
          ids.delete(elementId)
        } else {
          ids.add(elementId)
        }
        return { ...s, selectedIds: ids }
      }
      return { ...s, selectedIds: new Set([elementId]) }
    })
  }, [setSelection])

  const clearSelection = useCallback(() => {
    setSelection(() => EMPTY_SELECTION)
  }, [setSelection])

  const selectMultiple = useCallback((ids: readonly string[]) => {
    setSelection((s) => ({
      ...s,
      selectedIds: new Set(ids),
    }))
  }, [setSelection])

  return { selectElement, clearSelection, selectMultiple, EMPTY_SELECTION } as const
}

export { EMPTY_SELECTION, useSelection }
