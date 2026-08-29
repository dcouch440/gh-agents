import { useState, useCallback } from 'react'
import { useStore, workflowStore, workflowExecutionStore, workflowLiveStore } from '@/stores'
import { api } from '@/api'

type RunStatus = 'idle' | 'running' | 'error'

type UseWorkflowRunResult = {
  status: RunStatus
  handleRun: () => void
  handleCancel: () => void
  tooltipText: string
}

const TOOLTIP_MAP: Record<RunStatus, string> = {
  running: 'Click to cancel',
  error: 'Execution failed to start',
  idle: 'Run workflow',
}

/**
 * Run button state, derived from the server rather than a local timer.
 *
 * The old version flipped to 'completed' and reset itself after three seconds,
 * so the button read as idle while the run was still going — and a refresh
 * always showed idle regardless of what the server was doing.
 */
const useWorkflowRun = (promptInput: string): UseWorkflowRunResult => {
  const activeWorkflowId = useStore(workflowStore.store, workflowStore.selectActiveWorkflowId)
  const isRunning = useStore(workflowExecutionStore.store, workflowExecutionStore.selectIsRunning)
  const runId = useStore(workflowExecutionStore.store, workflowExecutionStore.selectRunId)
  const [lastError, setLastError] = useState<string | null>(null)

  const handleRun = useCallback(async () => {
    if (!activeWorkflowId || isRunning) return
    setLastError(null)
    try {
      const input = promptInput.trim()
      const body = input ? { initial_input: input } : undefined
      const resp = await api.workflows.run(activeWorkflowId, body)
      // The REST-path equivalent of a `workflow_started` event: open the overlay
      // for the new run immediately so the previous run's results cannot linger.
      workflowExecutionStore.beginRun(resp.execution_id, activeWorkflowId)
      void workflowLiveStore.hydrateActive()
    } catch (e) {
      setLastError(e instanceof Error ? e.message : 'Execution failed to start')
    }
  }, [activeWorkflowId, isRunning, promptInput])

  const handleCancel = useCallback(async () => {
    if (!runId) return
    try {
      await api.workflows.cancel(runId)
    } catch (e) {
      console.error('Cancel run failed:', e)
    }
  }, [runId])

  const status: RunStatus = isRunning ? 'running' : lastError !== null ? 'error' : 'idle'

  return {
    status,
    handleRun: () => { void handleRun() },
    handleCancel: () => { void handleCancel() },
    tooltipText: TOOLTIP_MAP[status],
  }
}

export { useWorkflowRun }
export type { RunStatus, UseWorkflowRunResult }
