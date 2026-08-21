import { store } from './_store'
import {
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
} from './selectors'
import { fetchRuns, viewHistoricalRun, returnToLive, reset } from './history'
import { handleWsEvent } from './wsHandler'
import {
  setActiveRun,
  beginRun,
  applyServerRun,
  applyWorkshopFallback,
  hydrateAgentSources,
} from './hydrate'

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
  setActiveRun,
  beginRun,
  applyServerRun,
  applyWorkshopFallback,
  hydrateAgentSources,
  reset,
}

export type { ServerRunSnapshot } from './hydrate'
export type { WorkflowExecutionState, StepExecutionState, StepExecutionStatus, StepTimelineEvent, ViewMode } from './types'
