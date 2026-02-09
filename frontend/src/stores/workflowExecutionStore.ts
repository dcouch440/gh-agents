// ============================================================================
// workflowExecutionStore — Live workflow execution overlay (WS-driven)
// ============================================================================

import { createStore } from './lib'
import { api } from '@/api'
import { WORKFLOW_EVENT } from '@/types/ws'
import type { WsWireMessage } from '@/types/ws'
import type {
  WorkflowStartedData,
  StepStartedData,
  StepCompletedData,
  StepFailedData,
  StepPausedData,
  ForEachProgressData,
  WorkflowCompletedData,
  WorkflowFailedData,
  WorkflowResumedData,
} from '@/types/ws'
import type { WorkflowExecutionSummary } from '@/types'

// ── Types ────────────────────────────────────────────────────────────────────

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

// ── Helpers ──────────────────────────────────────────────────────────────────

const makeDefaultStepState = (): StepExecutionState => ({
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
})

const updateStep = (
  states: Record<string, StepExecutionState>,
  stepId: string,
  patch: Partial<StepExecutionState>,
): Record<string, StepExecutionState> => ({
  ...states,
  [stepId]: { ...(states[stepId] ?? makeDefaultStepState()), ...patch },
})

const extractError = (e: unknown): string =>
  e instanceof Error ? e.message : 'Unknown error'

// ── Store ────────────────────────────────────────────────────────────────────

const initialState: WorkflowExecutionState = {
  runId: null,
  workflowId: null,
  isRunning: false,
  stepStates: {},
  eventLog: [],
  totalSteps: 0,
  completedStepCount: 0,
  durationMs: null,
  error: null,
  startedAt: null,
  completedAt: null,
  viewMode: 'live',
  runs: [],
  selectedHistoricalRunId: null,
  historicalRun: null,
  historyLoading: false,
  historyError: null,
}

const store = createStore<WorkflowExecutionState>(() => ({ ...initialState }))

// ── Selectors ────────────────────────────────────────────────────────────────

const selectIsRunning = (s: WorkflowExecutionState): boolean => s.isRunning

const selectRunId = (s: WorkflowExecutionState): string | null => s.runId

const selectWorkflowId = (s: WorkflowExecutionState): string | null => s.workflowId

const selectTotalSteps = (s: WorkflowExecutionState): number => s.totalSteps

const selectError = (s: WorkflowExecutionState): string | null => s.error

const selectStepState = (stepId: string) => (s: WorkflowExecutionState): StepExecutionState | undefined =>
  s.stepStates[stepId]

const selectCompletedStepCount = (s: WorkflowExecutionState): number =>
  s.completedStepCount

const selectEventLog = (s: WorkflowExecutionState): StepTimelineEvent[] => s.eventLog

const selectStepStates = (s: WorkflowExecutionState): Record<string, StepExecutionState> => s.stepStates

const selectStartedAt = (s: WorkflowExecutionState): string | null => s.startedAt

const selectCompletedAt = (s: WorkflowExecutionState): string | null => s.completedAt

const selectDurationMs = (s: WorkflowExecutionState): number | null => s.durationMs

const selectViewMode = (s: WorkflowExecutionState): ViewMode => s.viewMode

const selectRuns = (s: WorkflowExecutionState): WorkflowExecutionSummary[] => s.runs

const selectSelectedHistoricalRunId = (s: WorkflowExecutionState): string | null => s.selectedHistoricalRunId

const selectHistoricalRun = (s: WorkflowExecutionState): WorkflowExecutionSummary | null => s.historicalRun

const selectHistoryLoading = (s: WorkflowExecutionState): boolean => s.historyLoading

const selectHistoryError = (s: WorkflowExecutionState): string | null => s.historyError

// ── History Actions ─────────────────────────────────────────────────────────

const fetchRuns = async (workflowId: string): Promise<void> => {
  store.setState({ historyLoading: true, historyError: null })
  try {
    const data = await api.workflows.listExecutions(workflowId)
    store.setState({ runs: data, historyLoading: false })
  } catch (e) {
    store.setState({ historyLoading: false, historyError: extractError(e) })
  }
}

const viewHistoricalRun = (runId: string): void => {
  const { runs } = store.getState()
  const run = runs.find((r) => r.id === runId) ?? null
  store.setState({
    viewMode: 'history',
    selectedHistoricalRunId: runId,
    historicalRun: run,
  })
}

const returnToLive = (): void => {
  store.setState({
    viewMode: 'live',
    selectedHistoricalRunId: null,
    historicalRun: null,
  })
}

// ── WS Event Handler ────────────────────────────────────────────────────────

const handleWsEvent = (msg: WsWireMessage): void => {
  try {
  switch (msg.event) {
    case WORKFLOW_EVENT.STARTED: {
      const d = msg.data as WorkflowStartedData
      store.setState({
        runId: msg.run_id,
        workflowId: d.workflow_id,
        isRunning: true,
        stepStates: {},
        eventLog: [],
        totalSteps: d.total_steps,
        completedStepCount: 0,
        durationMs: null,
        error: null,
        startedAt: msg.ts,
        completedAt: null,
        viewMode: 'live',
        selectedHistoricalRunId: null,
        historicalRun: null,
      })
      break
    }
    case WORKFLOW_EVENT.STEP_STARTED: {
      const d = msg.data as StepStartedData
      store.setState((s) => ({
        stepStates: updateStep(s.stepStates, d.step_id, {
          status: 'running',
          stepName: d.step_name,
          agentId: d.agent_id ?? null,
          executionId: d.execution_id ?? null,
          startedAt: msg.ts,
        }),
        eventLog: [...s.eventLog, { stepId: d.step_id, stepName: d.step_name, eventType: 'started' as const, ts: msg.ts }],
      }))
      break
    }
    case WORKFLOW_EVENT.STEP_COMPLETED: {
      const d = msg.data as StepCompletedData
      store.setState((s) => ({
        completedStepCount: s.completedStepCount + 1,
        stepStates: updateStep(s.stepStates, d.step_id, {
          status: 'success',
          stepName: d.step_name,
          output: d.output ?? null,
          inputTokens: d.input_tokens ?? null,
          outputTokens: d.output_tokens ?? null,
          durationMs: d.duration_ms ?? null,
          completedAt: msg.ts,
        }),
        eventLog: [...s.eventLog, { stepId: d.step_id, stepName: d.step_name, eventType: 'completed' as const, ts: msg.ts }],
      }))
      break
    }
    case WORKFLOW_EVENT.STEP_FAILED: {
      const d = msg.data as StepFailedData
      store.setState((s) => ({
        stepStates: updateStep(s.stepStates, d.step_id, {
          status: 'error',
          stepName: d.step_name,
          error: d.error,
          completedAt: msg.ts,
        }),
        eventLog: [...s.eventLog, { stepId: d.step_id, stepName: d.step_name, eventType: 'failed' as const, ts: msg.ts }],
      }))
      break
    }
    case WORKFLOW_EVENT.STEP_PAUSED: {
      const d = msg.data as StepPausedData
      store.setState((s) => ({
        stepStates: updateStep(s.stepStates, d.step_id, {
          status: 'paused',
          stepName: d.step_name,
        }),
        eventLog: [...s.eventLog, { stepId: d.step_id, stepName: d.step_name, eventType: 'paused' as const, ts: msg.ts }],
      }))
      break
    }
    case WORKFLOW_EVENT.FOR_EACH_PROGRESS: {
      const d = msg.data as ForEachProgressData
      store.setState((s) => ({
        stepStates: updateStep(s.stepStates, d.step_id, {
          forEachProgress: { completed: d.completed, total: d.total },
        }),
      }))
      break
    }
    case WORKFLOW_EVENT.COMPLETED: {
      const d = msg.data as WorkflowCompletedData
      const currentWorkflowId = store.getState().workflowId
      store.setState({
        isRunning: false,
        durationMs: d.duration_ms ?? null,
        completedAt: msg.ts,
      })
      if (currentWorkflowId) void fetchRuns(currentWorkflowId)
      break
    }
    case WORKFLOW_EVENT.FAILED: {
      const d = msg.data as WorkflowFailedData
      const currentWorkflowId = store.getState().workflowId
      store.setState({
        isRunning: false,
        error: d.error,
        completedAt: msg.ts,
      })
      if (currentWorkflowId) void fetchRuns(currentWorkflowId)
      break
    }
    case WORKFLOW_EVENT.RESUMED: {
      const d = msg.data as WorkflowResumedData
      store.setState((s) => ({
        isRunning: true,
        stepStates: updateStep(s.stepStates, d.step_id, {
          status: 'running',
          startedAt: msg.ts,
        }),
        eventLog: [...s.eventLog, { stepId: d.step_id, stepName: s.stepStates[d.step_id]?.stepName ?? null, eventType: 'resumed' as const, ts: msg.ts }],
      }))
      break
    }
  }
  } catch (err) {
    console.error(`[workflowExecutionStore] WS handler error on "${msg.event}":`, err)
  }
}

// ── Reset ────────────────────────────────────────────────────────────────────

const reset = (): void => {
  store.setState({ ...initialState })
}

// ── Export ────────────────────────────────────────────────────────────────────

export const workflowExecutionStore = {
  store,
  selectIsRunning,
  selectRunId,
  selectWorkflowId,
  selectTotalSteps,
  selectError,
  selectStepState,
  selectCompletedStepCount,
  selectEventLog,
  selectStepStates,
  selectStartedAt,
  selectCompletedAt,
  selectDurationMs,
  selectViewMode,
  selectRuns,
  selectSelectedHistoricalRunId,
  selectHistoricalRun,
  selectHistoryLoading,
  selectHistoryError,
  fetchRuns,
  viewHistoricalRun,
  returnToLive,
  handleWsEvent,
  reset,
}

export type { WorkflowExecutionState, StepExecutionState, StepExecutionStatus, StepTimelineEvent, ViewMode }
