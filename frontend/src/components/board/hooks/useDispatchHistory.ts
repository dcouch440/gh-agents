import { useEffect, useRef } from 'react'
import { api } from '@/api'
import { dispatchStore } from '@/stores/dispatchStore'
import { workflowStore } from '@/stores/workflowStore'

/**
 * Fetch historical dispatch traces for all steps in the current workflow.
 *
 * Subscribes to workflowStore directly (not via useStore — avoids
 * re-render loops from array reference instability). Waits for steps
 * to appear, then fetches the most recent dispatch trace for each.
 */
const useDispatchHistory = (workflowId: string): void => {
  const fetchedForRef = useRef<string | null>(null)

  useEffect(() => {
    // Reset when workflow changes
    if (fetchedForRef.current !== workflowId) {
      fetchedForRef.current = null
    }

    const fetchHistory = async (stepIds: readonly string[]) => {
      if (stepIds.length === 0) return

      await Promise.all(stepIds.map(async (stepId) => {
        try {
          const tasksResp = await api.dispatch.listForStep(stepId)
          if (tasksResp.tasks.length === 0) return

          const latest = tasksResp.tasks[tasksResp.tasks.length - 1]!
          const traceResp = await api.dispatch.trace(latest.execution_id)
          dispatchStore.hydrateFromApi(traceResp)
        } catch {
          // Silently skip — historical data is best-effort
        }
      }))
    }

    // Check if steps are already loaded
    const tryFetch = () => {
      if (fetchedForRef.current === workflowId) return true
      const { steps } = workflowStore.store.getState()
      const stepIds = Array.from(steps.byId.keys())
      if (stepIds.length === 0) return false

      fetchedForRef.current = workflowId
      void fetchHistory(stepIds)
      return true
    }

    // If steps already available, fetch immediately
    if (tryFetch()) return

    // Otherwise, subscribe and wait for steps to appear
    const unsub = workflowStore.store.subscribe(() => {
      if (tryFetch()) {
        unsub()
      }
    })

    return () => { unsub() }
  }, [workflowId])
}

export { useDispatchHistory }
