import type { WorkflowExecutionSummary } from '@/types'

type StepExecutionStatus = 'idle' | 'pending' | 'running' | 'success' | 'error' | 'skipped' | 'paused'

type StepExecutionState = {
  status: StepExecutionStatus
  stepName: string | null
  agentId: string | null
  executionId: string | null
  output: string | null
  error: string | null
  inputTokens: number | null
  outputTokens: number | null
  durationMs: number | null
  forEachProgress: { completed: number; total: number } | null
  startedAt: string | null
  completedAt: string | null
}

type StepTimelineEvent = {
  stepId: string
  stepName: string | null
  eventType: 'started' | 'completed' | 'failed' | 'paused' | 'resumed'
  ts: string
}

type ViewMode = 'live' | 'history'

type WorkflowExecutionState = {
  // Live execution state
  runId: string | null
  workflowId: string | null
  isRunning: boolean
  stepStates: Record<string, StepExecutionState>
  eventLog: StepTimelineEvent[]
  totalSteps: number
  completedStepCount: number
  durationMs: number | null
  error: string | null
  startedAt: string | null
  completedAt: string | null
  // History state
  viewMode: ViewMode
  runs: WorkflowExecutionSummary[]
  selectedHistoricalRunId: string | null
  historicalRun: WorkflowExecutionSummary | null
  historyLoading: boolean
  historyError: string | null
}

export type { WorkflowExecutionState, StepExecutionState, StepExecutionStatus, StepTimelineEvent, ViewMode }
