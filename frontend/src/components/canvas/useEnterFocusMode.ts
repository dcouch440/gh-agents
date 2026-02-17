import { useCallback } from 'react'
import { workflowStore, canvasStore, focusModeStore } from '@/stores'
import { topoSortStepIds } from '@/utils/topoSort'

/**
 * Returns a stable callback that enters focus mode.
 * Reads steps/edges from the store at call time (no subscriptions).
 * If no explicit `initialStepId` is provided, falls back to the first selected step.
 */
const useEnterFocusMode = (): ((initialStepId?: string) => void) => {
  return useCallback((initialStepId?: string) => {
    const state = workflowStore.store.getState()
    const stepsArr = [...state.steps.byId.values()]
    const edgesArr = [...state.edges.byId.values()]
    const ordered = topoSortStepIds(stepsArr, edgesArr)
    if (ordered.length === 0) return

    const effectiveId = initialStepId ?? ordered.find((id) => canvasStore.store.getState().selectedStepIds.has(id))
    focusModeStore.enter(ordered, effectiveId)
  }, [])
}

export { useEnterFocusMode }
