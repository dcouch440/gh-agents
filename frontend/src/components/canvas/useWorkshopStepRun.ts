import { useState, useCallback } from 'react'
import { useStore, workflowStore, workflowExecutionStore } from '@/stores'
import { api } from '@/api'

type WorkshopRunStatus = 'idle' | 'initializing' | 'running' | 'completed' | 'error'

type UseWorkshopStepRunResult = {
  status: WorkshopRunStatus
  error: string | null
  handleRun: () => void
}

const RESET_DELAY_MS = 3000

const useWorkshopStepRun = (stepId: string): UseWorkshopStepRunResult => {
  const activeWorkflowId = useStore(workflowStore.store, workflowStore.selectActiveWorkflowId)
  const storeRunId = useStore(workflowExecutionStore.store, workflowExecutionStore.selectRunId)
  const [status, setStatus] = useState<WorkshopRunStatus>('idle')
  const [error, setError] = useState<string | null>(null)

  const handleRun = useCallback(async () => {
    if (!activeWorkflowId || status === 'running' || status === 'initializing') return

    setError(null)
    setStatus('initializing')

    try {
      // Ensure workshop session exists. If the store already has a runId
      // from hydration, skip — execute_workshop_step creates it internally.
      if (storeRunId === null) {
        await api.workflows.getOrCreateWorkshop(activeWorkflowId)
      }

      setStatus('running')
      await api.workflows.executeWorkshopStep(activeWorkflowId, stepId)
      setStatus('completed')
      setTimeout(() => { setStatus('idle') }, RESET_DELAY_MS)
    } catch (err) {
      const message = err instanceof Error ? err.message : 'Step execution failed'
      setError(message)
      setStatus('error')
      setTimeout(() => { setStatus('idle'); setError(null) }, RESET_DELAY_MS)
    }
  }, [activeWorkflowId, stepId, status, storeRunId])

  return {
    status,
    error,
    handleRun: () => { void handleRun() },
  }
}

export { useWorkshopStepRun }
export type { WorkshopRunStatus, UseWorkshopStepRunResult }
