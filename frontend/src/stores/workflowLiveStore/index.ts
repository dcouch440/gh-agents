import { store, initialState } from './_store'
import {
  selectWorkflowId,
  selectBaselineByStep,
  selectBaselineForStep,
  selectDispatches,
  selectDispatchForStep,
  selectRunSteps,
  selectIsGenerating,
  selectLoading,
  selectError,
  selectHydratedAt,
} from './selectors'
import { hydrateLiveState, hydrateActive, UNCONFIRMED_LIMIT } from './hydrate'
import { startLiveSync, stopLiveSync, rescheduleLiveSync } from './sync'
import { viewHistoricalRun, returnToLive } from './history'

const reset = (): void => {
  stopLiveSync()
  store.setState({ ...initialState })
}

/**
 * Optimistic flag while a Generate request is in flight.
 *
 * Held across `UNCONFIRMED_LIMIT` reads that disagree, because the server
 * genuinely does not know about the work yet — `POST /generate` spawns its
 * pipeline and returns immediately. Without the grace the spinner flicks off
 * the moment we ask, then back on once the first task registers.
 */
const setGenerating = (isGenerating: boolean): void => {
  store.setState({ isGenerating, unconfirmedGenerating: isGenerating ? UNCONFIRMED_LIMIT : 0 })
  // Switching to generating changes what "busy" means, and the poll may be
  // sitting on the long idle delay. Re-arm it so server truth arrives in one
  // active interval rather than up to fifteen seconds later.
  if (isGenerating) rescheduleLiveSync()
}

export const workflowLiveStore = {
  store,
  selectWorkflowId,
  selectBaselineByStep,
  selectBaselineForStep,
  selectDispatches,
  selectDispatchForStep,
  selectRunSteps,
  selectIsGenerating,
  selectLoading,
  selectError,
  selectHydratedAt,
  hydrateLiveState,
  hydrateActive,
  startLiveSync,
  stopLiveSync,
  rescheduleLiveSync,
  setGenerating,
  viewHistoricalRun,
  returnToLive,
  reset,
}

export type { BaselineStepState, LiveDispatch, WorkflowLiveState } from './types'
