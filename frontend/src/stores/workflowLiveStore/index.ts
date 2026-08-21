import { store, initialState } from './_store'
import {
  selectWorkflowId,
  selectBaselineByStep,
  selectBaselineForStep,
  selectDispatches,
  selectRunSteps,
  selectIsGenerating,
  selectLoading,
  selectError,
  selectHydratedAt,
} from './selectors'
import { hydrateLiveState, hydrateActive } from './hydrate'
import { startLiveSync, stopLiveSync } from './sync'

const reset = (): void => {
  stopLiveSync()
  store.setState({ ...initialState })
}

/** Optimistic flag while a Generate request is in flight, until the next tick. */
const setGenerating = (isGenerating: boolean): void => {
  store.setState({ isGenerating })
}

export const workflowLiveStore = {
  store,
  selectWorkflowId,
  selectBaselineByStep,
  selectBaselineForStep,
  selectDispatches,
  selectRunSteps,
  selectIsGenerating,
  selectLoading,
  selectError,
  selectHydratedAt,
  hydrateLiveState,
  hydrateActive,
  startLiveSync,
  stopLiveSync,
  setGenerating,
  reset,
}

export type { BaselineStepState, LiveDispatch, WorkflowLiveState } from './types'
