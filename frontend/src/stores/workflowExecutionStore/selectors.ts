import type { WorkflowExecutionSummary } from '@/types'
import { memoFactory } from '../lib'
import type { WorkflowExecutionState, StepExecutionState, StepTimelineEvent, ViewMode } from './types'

const selectIsRunning = (s: WorkflowExecutionState): boolean => s.isRunning

const selectRunId = (s: WorkflowExecutionState): string | null => s.runId

const selectWorkflowId = (s: WorkflowExecutionState): string | null => s.workflowId

const selectTotalSteps = (s: WorkflowExecutionState): number => s.totalSteps

const selectError = (s: WorkflowExecutionState): string | null => s.error

const selectStepState = memoFactory(
  (stepId: string) =>
  (s: WorkflowExecutionState): StepExecutionState | undefined =>
    s.stepStates[stepId],
)

const selectCompletedStepCount = (s: WorkflowExecutionState): number => s.completedStepCount

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

export {
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
}
