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
import { hydrateLatestRun, hydrateWorkshop, hydrateAgentSources } from './hydrate'

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
  hydrateLatestRun,
  hydrateWorkshop,
  hydrateAgentSources,
  reset,
}

export type { WorkflowExecutionState, StepExecutionState, StepExecutionStatus, StepTimelineEvent, ViewMode } from './types'
