import { useEffect, useRef } from 'react'
import Box from '@mui/material/Box'
import { useStore, workflowExecutionStore } from '@/stores'
import { ExecutionTimelineEntry } from './ExecutionTimelineEntry'
import type { StepExecutionState } from '@/stores'

type ExecutionTimelineProps = {
  stepIds: string[]
  isWorkflowRunning: boolean
}

const defaultStep: StepExecutionState = {
  status: 'pending',
  stepName: null,
  agentId: null,
  executionId: null,
  output: null,
  error: null,
  inputTokens: null,
  outputTokens: null,
  durationMs: null,
  forEachProgress: null,
  startedAt: null,
  completedAt: null,
}

function ExecutionTimeline({ stepIds, isWorkflowRunning }: ExecutionTimelineProps) {
  const stepStates = useStore(workflowExecutionStore.store, workflowExecutionStore.selectStepStates)
  const bottomRef = useRef<HTMLDivElement>(null)

  useEffect(() => {
    if (isWorkflowRunning && bottomRef.current?.scrollIntoView) {
      bottomRef.current.scrollIntoView({ behavior: 'smooth', block: 'end' })
    }
  }, [stepIds.length, isWorkflowRunning])

  return (
    <Box sx={{ flex: 1, overflow: 'auto', px: 1, py: 1 }}>
      {stepIds.map((stepId, idx) => (
        <ExecutionTimelineEntry
          key={stepId}
          stepId={stepId}
          stepState={stepStates[stepId] ?? defaultStep}
          isLast={idx === stepIds.length - 1}
        />
      ))}
      <div ref={bottomRef} />
    </Box>
  )
}

export { ExecutionTimeline }
export type { ExecutionTimelineProps }
