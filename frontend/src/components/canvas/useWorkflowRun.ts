import { useState, useCallback } from 'react'
import { useStore, workflowStore } from '@/stores'
import { api } from '@/api'

type RunStatus = 'idle' | 'running' | 'completed' | 'error'

type UseWorkflowRunResult = {
  status: RunStatus
  handleRun: () => void
  tooltipText: string
}

const RESET_DELAY_MS = 3000

const TOOLTIP_MAP: Record<RunStatus, string> = {
  running: 'Workflow is running...',
  completed: 'Execution started successfully',
  error: 'Execution failed to start',
  idle: 'Run workflow',
}

const useWorkflowRun = (promptInput: string): UseWorkflowRunResult => {
  const activeWorkflowId = useStore(workflowStore.store, workflowStore.selectActiveWorkflowId)
  const [status, setStatus] = useState<RunStatus>('idle')

  const handleRun = useCallback(async () => {
    if (!activeWorkflowId || status === 'running') return
    setStatus('running')
    try {
      const input = promptInput.trim()
      const body = input ? { initial_input: input } : undefined
      await api.workflows.run(activeWorkflowId, body)
      setStatus('completed')
      setTimeout(() => { setStatus('idle') }, RESET_DELAY_MS)
    } catch {
      setStatus('error')
      setTimeout(() => { setStatus('idle') }, RESET_DELAY_MS)
    }
  }, [activeWorkflowId, status, promptInput])

  return {
    status,
    handleRun: () => { void handleRun() },
    tooltipText: TOOLTIP_MAP[status],
  }
}

export { useWorkflowRun }
export type { RunStatus, UseWorkflowRunResult }
