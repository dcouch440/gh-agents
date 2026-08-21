import { createStore } from '../lib'
import type { WorkflowLiveState } from './types'

const initialState: WorkflowLiveState = {
  workflowId: null,
  baselineByStep: {},
  dispatches: [],
  runSteps: [],
  isGenerating: false,
  loading: false,
  error: null,
  consecutiveFailures: 0,
  hydratedAt: null,
}

const store = createStore<WorkflowLiveState>(() => ({ ...initialState }))

export { store, initialState }
