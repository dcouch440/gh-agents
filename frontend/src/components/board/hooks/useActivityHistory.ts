import { useEffect, useRef } from 'react'
import { api } from '@/api'
import { activityStore } from '@/stores/activity'
import { buildHistoricalEntries } from '@/stores/activity/buildHistoricalEntries'

/**
 * Fetch historical workflow execution data and hydrate the activity feed.
 *
 * Loads the most recent execution for the workflow, converts the step-level
 * results into ActivityEntry objects, and prepends them to the activity store.
 * Live WebSocket events continue appending after hydration.
 */
const useActivityHistory = (workflowId: string): void => {
  const fetchedForRef = useRef<string | null>(null)

  useEffect(() => {
    if (fetchedForRef.current !== workflowId) {
      fetchedForRef.current = null
    }

    const fetchHistory = async () => {
      if (fetchedForRef.current === workflowId) return
      fetchedForRef.current = workflowId

      try {
        // Get list of executions (most recent first)
        const executions = await api.workflows.listExecutions(workflowId)
        if (executions.length === 0) return

        // Fetch detail for the most recent execution
        const latest = executions[0]!
        const detail = await api.workflows.getRunDetail(workflowId, latest.id)

        // Convert to activity entries and hydrate the store
        const entries = buildHistoricalEntries(detail.execution, detail.steps)
        if (entries.length > 0) {
          activityStore.hydrateFromHistory(entries)
        }
      } catch {
        // Silently skip — historical data is best-effort
      }
    }

    void fetchHistory()
  }, [workflowId])
}

export { useActivityHistory }
