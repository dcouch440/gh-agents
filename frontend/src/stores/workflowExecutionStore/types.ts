import type { WorkflowExecutionSummary } from '@/types'

type StepExecutionStatus = 'idle' | 'pending' | 'running' | 'success' | 'error' | 'skipped' | 'paused'

type ChildStepState = {
  childStepId: string
  childStepName: string
  status: 'running' | 'success' | 'error'
  inputTokens: number | null
  outputTokens: number | null
  durationMs: number | null
  error: string | null
}

type SubWorkflowProgress = {
  childExecutionId: string
  totalSteps: number
  completedSteps: number
  status: 'running' | 'completed' | 'failed'
  childSteps: ChildStepState[]
}

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
  subWorkflowProgress: SubWorkflowProgress | null
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

export type { WorkflowExecutionState, StepExecutionState, StepExecutionStatus, StepTimelineEvent, ViewMode, ChildStepState, SubWorkflowProgress }
