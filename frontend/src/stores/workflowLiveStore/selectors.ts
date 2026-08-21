import { memoFactory } from '../lib'
import type { RunStepResult } from '@/types'
import type { BaselineStepState, LiveDispatch, WorkflowLiveState } from './types'

const selectWorkflowId = (s: WorkflowLiveState): string | null => s.workflowId

const selectBaselineByStep = (s: WorkflowLiveState): Readonly<Record<string, BaselineStepState>> =>
  s.baselineByStep

const selectBaselineForStep = memoFactory(
  (stepId: string) =>
  (s: WorkflowLiveState): BaselineStepState | null =>
    s.baselineByStep[stepId] ?? null,
)

const selectDispatches = (s: WorkflowLiveState): readonly LiveDispatch[] => s.dispatches

const selectRunSteps = (s: WorkflowLiveState): readonly RunStepResult[] => s.runSteps

const selectIsGenerating = (s: WorkflowLiveState): boolean => s.isGenerating

const selectLoading = (s: WorkflowLiveState): boolean => s.loading

const selectError = (s: WorkflowLiveState): string | null => s.error

const selectConsecutiveFailures = (s: WorkflowLiveState): number => s.consecutiveFailures

const selectHydratedAt = (s: WorkflowLiveState): string | null => s.hydratedAt

export {
  selectWorkflowId,
  selectBaselineByStep,
  selectBaselineForStep,
  selectDispatches,
  selectRunSteps,
  selectIsGenerating,
  selectLoading,
  selectError,
  selectConsecutiveFailures,
  selectHydratedAt,
}
