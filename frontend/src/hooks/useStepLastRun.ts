import { useState, useEffect, useCallback } from 'react'
import { useStore, workflowStore } from '@/stores'
import { api } from '@/api'
import type { StepLastRunResponse } from '@/types'

type UseStepLastRunResult = {
  data: StepLastRunResponse | null
  isLoading: boolean
  error: string | null
  refresh: () => void
}

const useStepLastRun = (stepId: string): UseStepLastRunResult => {
  const workflowId = useStore(workflowStore.store, workflowStore.selectActiveWorkflowId)
  const [isLoading, setIsLoading] = useState(true)
  const [error, setError] = useState<string | null>(null)
  const [data, setData] = useState<StepLastRunResponse | null>(null)

  const fetchData = useCallback(async () => {
    if (!workflowId) return

    setIsLoading(true)
    setError(null)
    try {
      const result = await api.workflows.getStepLastRun(workflowId, stepId)
      setData(result)
    } catch (e) {
      const is404 = e instanceof Error && e.message.includes('404')
      if (is404) {
        setData(null)
      } else {
        setError(e instanceof Error ? e.message : 'Failed to load last run data')
      }
    } finally {
      setIsLoading(false)
    }
  }, [workflowId, stepId])

  useEffect(() => {
    void fetchData()
  }, [fetchData])

  const refresh = useCallback(() => {
    void fetchData()
  }, [fetchData])

  return { data, isLoading, error, refresh }
}

export { useStepLastRun }
export type { UseStepLastRunResult }
