import { useEffect, useMemo } from 'react'
import Box from '@mui/material/Box'
import PlayArrowOutlined from '@mui/icons-material/PlayArrowOutlined'
import ErrorOutline from '@mui/icons-material/ErrorOutline'
import { EmptyState } from '@/components/primitives'
import { useStore, workflowExecutionStore, workflowStore } from '@/stores'
import { ExecutionRunSelector } from './ExecutionRunSelector'
import { ExecutionRunHeader } from './ExecutionRunHeader'
import { ExecutionTimeline } from './ExecutionTimeline'
import { HistoricalRunSummary } from './HistoricalRunSummary'
import type { StepTimelineEvent } from '@/stores'

const deriveStepIds = (eventLog: StepTimelineEvent[]): string[] => {
  const seen = new Set<string>()
  const ordered: string[] = []
  for (const entry of eventLog) {
    if (!seen.has(entry.stepId)) {
      seen.add(entry.stepId)
      ordered.push(entry.stepId)
    }
  }
  return ordered
}

function ExecutionPanel() {
  const activeWorkflowId = useStore(workflowStore.store, workflowStore.selectActiveWorkflowId)
  const runId = useStore(workflowExecutionStore.store, workflowExecutionStore.selectRunId)
  const isRunning = useStore(workflowExecutionStore.store, workflowExecutionStore.selectIsRunning)
  const eventLog = useStore(workflowExecutionStore.store, workflowExecutionStore.selectEventLog)
  const completedSteps = useStore(workflowExecutionStore.store, workflowExecutionStore.selectCompletedStepCount)
  const totalSteps = useStore(workflowExecutionStore.store, workflowExecutionStore.selectTotalSteps)
  const durationMs = useStore(workflowExecutionStore.store, workflowExecutionStore.selectDurationMs)
  const error = useStore(workflowExecutionStore.store, workflowExecutionStore.selectError)
  const startedAt = useStore(workflowExecutionStore.store, workflowExecutionStore.selectStartedAt)
  const completedAt = useStore(workflowExecutionStore.store, workflowExecutionStore.selectCompletedAt)
  const viewMode = useStore(workflowExecutionStore.store, workflowExecutionStore.selectViewMode)
  const runs = useStore(workflowExecutionStore.store, workflowExecutionStore.selectRuns)
  const selectedHistoricalRunId = useStore(workflowExecutionStore.store, workflowExecutionStore.selectSelectedHistoricalRunId)
  const historicalRun = useStore(workflowExecutionStore.store, workflowExecutionStore.selectHistoricalRun)
  const historyLoading = useStore(workflowExecutionStore.store, workflowExecutionStore.selectHistoryLoading)
  const historyError = useStore(workflowExecutionStore.store, workflowExecutionStore.selectHistoryError)

  const stepIds = useMemo(() => deriveStepIds(eventLog), [eventLog])

  useEffect(() => {
    if (activeWorkflowId) {
      void workflowExecutionStore.fetchRuns(activeWorkflowId)
    }
  }, [activeWorkflowId])

  const hasLiveRun = runId !== null
  const hasHistory = runs.length > 0

  if (!hasLiveRun && !hasHistory && !historyLoading) {
    if (historyError) {
      return (
        <EmptyState
          icon={<ErrorOutline fontSize="large" />}
          message={`Failed to load history: ${historyError}`}
        />
      )
    }
    return (
      <EmptyState
        icon={<PlayArrowOutlined fontSize="large" />}
        message="Run a workflow to see execution details"
      />
    )
  }

  return (
    <Box sx={{ display: 'flex', flexDirection: 'column', height: '100%' }}>
      <ExecutionRunSelector
        currentRunId={runId}
        runs={runs}
        selectedHistoricalRunId={selectedHistoricalRunId}
        isRunning={isRunning}
        loading={historyLoading}
        onSelectRun={workflowExecutionStore.viewHistoricalRun}
        onReturnToLive={workflowExecutionStore.returnToLive}
      />
      {viewMode === 'live' && hasLiveRun && (
        <>
          <ExecutionRunHeader
            isRunning={isRunning}
            completedSteps={completedSteps}
            totalSteps={totalSteps}
            durationMs={durationMs}
            error={error}
            startedAt={startedAt}
            completedAt={completedAt}
          />
          <ExecutionTimeline stepIds={stepIds} isWorkflowRunning={isRunning} />
        </>
      )}
      {viewMode === 'history' && historicalRun && (
        <HistoricalRunSummary run={historicalRun} />
      )}
    </Box>
  )
}

export { ExecutionPanel }
