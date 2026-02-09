import { useMemo } from 'react'
import Box from '@mui/material/Box'
import PlayArrowOutlined from '@mui/icons-material/PlayArrowOutlined'
import { EmptyState } from '@/components/primitives'
import { useStore, workflowExecutionStore } from '@/stores'
import { ExecutionRunSelector } from './ExecutionRunSelector'
import { ExecutionRunHeader } from './ExecutionRunHeader'
import { ExecutionTimeline } from './ExecutionTimeline'
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
  const runId = useStore(workflowExecutionStore.store, workflowExecutionStore.selectRunId)
  const isRunning = useStore(workflowExecutionStore.store, workflowExecutionStore.selectIsRunning)
  const eventLog = useStore(workflowExecutionStore.store, workflowExecutionStore.selectEventLog)
  const completedSteps = useStore(workflowExecutionStore.store, workflowExecutionStore.selectCompletedStepCount)
  const totalSteps = useStore(workflowExecutionStore.store, workflowExecutionStore.selectTotalSteps)
  const durationMs = useStore(workflowExecutionStore.store, workflowExecutionStore.selectDurationMs)
  const error = useStore(workflowExecutionStore.store, workflowExecutionStore.selectError)
  const startedAt = useStore(workflowExecutionStore.store, workflowExecutionStore.selectStartedAt)
  const completedAt = useStore(workflowExecutionStore.store, workflowExecutionStore.selectCompletedAt)

  const stepIds = useMemo(() => deriveStepIds(eventLog), [eventLog])

  if (runId === null) {
    return (
      <EmptyState
        icon={<PlayArrowOutlined fontSize="large" />}
        message="Run a workflow to see execution details"
      />
    )
  }

  return (
    <Box sx={{ display: 'flex', flexDirection: 'column', height: '100%' }}>
      <ExecutionRunSelector currentRunId={runId} />
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
    </Box>
  )
}

export { ExecutionPanel }
