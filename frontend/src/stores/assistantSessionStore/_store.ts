import { createStore } from '../lib'
import type { AssistantSessionState, StepSession } from './types'

const emptySession: StepSession = {
  session: null,
  messages: [],
  streamingSegments: [],
  isLoading: true,
  error: null,
  activePanel: null,
}

const store = createStore<AssistantSessionState>(() => ({
  byStep: {},
}))

const getStep = (stepId: string): StepSession =>
  store.getState().byStep[stepId] ?? emptySession

const updateStep = (stepId: string, patch: Partial<StepSession>): void => {
  store.setState((s) => ({
    byStep: {
      ...s.byStep,
      [stepId]: { ...(s.byStep[stepId] ?? emptySession), ...patch },
    },
  }))
}

export { store, emptySession, getStep, updateStep }
