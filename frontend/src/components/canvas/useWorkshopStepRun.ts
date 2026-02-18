import { useState, useCallback, useRef } from 'react'
import { useStore, workflowStore } from '@/stores'
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
  const [status, setStatus] = useState<WorkshopRunStatus>('idle')
  const [error, setError] = useState<string | null>(null)
  const workshopRunIdRef = useRef<string | null>(null)

  const handleRun = useCallback(async () => {
    if (!activeWorkflowId || status === 'running' || status === 'initializing') return

    setError(null)
    setStatus('initializing')

    try {
      // Get or create workshop session (cached after first call)
      if (workshopRunIdRef.current === null) {
        const workshop = await api.workflows.getOrCreateWorkshop(activeWorkflowId)
        workshopRunIdRef.current = workshop.run_id
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
  }, [activeWorkflowId, stepId, status])

  return {
    status,
    error,
    handleRun: () => { void handleRun() },
  }
}

export { useWorkshopStepRun }
export type { WorkshopRunStatus, UseWorkshopStepRunResult }
