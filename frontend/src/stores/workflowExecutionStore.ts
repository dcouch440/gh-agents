// ============================================================================
// workflowExecutionStore — Live workflow execution overlay (WS-driven)
// ============================================================================

import { createStore } from './lib'
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

// ── Types ────────────────────────────────────────────────────────────────────

type StepExecutionStatus = 'idle' | 'pending' | 'running' | 'success' | 'error' | 'skipped' | 'paused'

type StepExecutionState = {
  status: StepExecutionStatus
  output: string | null
  error: string | null
  inputTokens: number | null
  outputTokens: number | null
  durationMs: number | null
  forEachProgress: { completed: number; total: number } | null
  startedAt: string | null
  completedAt: string | null
}

type WorkflowExecutionState = {
  runId: string | null
  workflowId: string | null
  isRunning: boolean
  stepStates: Record<string, StepExecutionState>
  totalSteps: number
  durationMs: number | null
  error: string | null
  startedAt: string | null
  completedAt: string | null
}

// ── Helpers ──────────────────────────────────────────────────────────────────

const makeDefaultStepState = (): StepExecutionState => ({
  status: 'pending',
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

// ── Store ────────────────────────────────────────────────────────────────────

const store = createStore<WorkflowExecutionState>(() => ({
  runId: null,
  workflowId: null,
  isRunning: false,
  stepStates: {},
  totalSteps: 0,
  durationMs: null,
  error: null,
  startedAt: null,
  completedAt: null,
}))

// ── Selectors ────────────────────────────────────────────────────────────────

const selectIsRunning = (s: WorkflowExecutionState): boolean => s.isRunning

const selectRunId = (s: WorkflowExecutionState): string | null => s.runId

const selectWorkflowId = (s: WorkflowExecutionState): string | null => s.workflowId

const selectTotalSteps = (s: WorkflowExecutionState): number => s.totalSteps

const selectError = (s: WorkflowExecutionState): string | null => s.error

const selectStepState = (stepId: string) => (s: WorkflowExecutionState): StepExecutionState | undefined =>
  s.stepStates[stepId]

const selectCompletedStepCount = (s: WorkflowExecutionState): number =>
  Object.values(s.stepStates).filter((ss) => ss.status === 'success').length

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
        totalSteps: d.total_steps,
        durationMs: null,
        error: null,
        startedAt: msg.ts,
        completedAt: null,
      })
      break
    }
    case WORKFLOW_EVENT.STEP_STARTED: {
      const d = msg.data as StepStartedData
      store.setState((s) => ({
        stepStates: updateStep(s.stepStates, d.step_id, {
          status: 'running',
          startedAt: msg.ts,
        }),
      }))
      break
    }
    case WORKFLOW_EVENT.STEP_COMPLETED: {
      const d = msg.data as StepCompletedData
      store.setState((s) => ({
        stepStates: updateStep(s.stepStates, d.step_id, {
          status: 'success',
          output: d.output ?? null,
          inputTokens: d.input_tokens ?? null,
          outputTokens: d.output_tokens ?? null,
          durationMs: d.duration_ms ?? null,
          completedAt: msg.ts,
        }),
      }))
      break
    }
    case WORKFLOW_EVENT.STEP_FAILED: {
      const d = msg.data as StepFailedData
      store.setState((s) => ({
        stepStates: updateStep(s.stepStates, d.step_id, {
          status: 'error',
          error: d.error,
          completedAt: msg.ts,
        }),
      }))
      break
    }
    case WORKFLOW_EVENT.STEP_PAUSED: {
      const d = msg.data as StepPausedData
      store.setState((s) => ({
        stepStates: updateStep(s.stepStates, d.step_id, {
          status: 'paused',
        }),
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
      store.setState({
        isRunning: false,
        durationMs: d.duration_ms ?? null,
        completedAt: msg.ts,
      })
      break
    }
    case WORKFLOW_EVENT.FAILED: {
      const d = msg.data as WorkflowFailedData
      store.setState({
        isRunning: false,
        error: d.error,
        completedAt: msg.ts,
      })
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
  store.setState({
    runId: null,
    workflowId: null,
    isRunning: false,
    stepStates: {},
    totalSteps: 0,
    durationMs: null,
    error: null,
    startedAt: null,
    completedAt: null,
  })
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
  handleWsEvent,
  reset,
}

export type { WorkflowExecutionState, StepExecutionState, StepExecutionStatus }
