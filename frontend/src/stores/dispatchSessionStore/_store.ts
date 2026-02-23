import { createStore } from '../lib'
import type { DispatchSessionState, DispatchStepSession } from './types'

const emptySession: DispatchStepSession = {
  sessionId: null,
  messages: [],
  isLoading: true,
  error: null,
}

const store = createStore<DispatchSessionState>(() => ({
  byStep: {},
}))

const getStep = (stepId: string): DispatchStepSession =>
  store.getState().byStep[stepId] ?? emptySession

const updateStep = (stepId: string, patch: Partial<DispatchStepSession>): void => {
  store.setState((s) => ({
    byStep: {
      ...s.byStep,
      [stepId]: { ...(s.byStep[stepId] ?? emptySession), ...patch },
    },
  }))
}

export { store, emptySession, getStep, updateStep }
