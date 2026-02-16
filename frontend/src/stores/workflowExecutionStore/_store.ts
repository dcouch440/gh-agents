import { createStore } from '../lib'
import type { WorkflowExecutionState, StepExecutionState } from './types'

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
  subWorkflowProgress: null,
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

export { store, initialState, makeDefaultStepState, updateStep }
