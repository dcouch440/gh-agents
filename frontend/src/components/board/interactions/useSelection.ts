// ============================================================================
// useSelection — Element Selection Interaction
// ============================================================================

import { useCallback } from 'react'
import type { SelectionState } from '../elements'
import type { SetSelection } from './types'

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

  return { selectElement, clearSelection, EMPTY_SELECTION } as const
}

export { EMPTY_SELECTION, useSelection }
