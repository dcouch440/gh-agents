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

/**
 * The most recent dispatch for one step, or null.
 *
 * `dispatches` is server-ordered newest-first with at most one row per step, so
 * the first match is the current one.
 */
const selectDispatchForStep = memoFactory(
  (stepId: string) =>
  (s: WorkflowLiveState): LiveDispatch | null =>
    s.dispatches.find((d) => d.stepId === stepId) ?? null,
)

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
  selectDispatchForStep,
  selectRunSteps,
  selectIsGenerating,
  selectLoading,
  selectError,
  selectConsecutiveFailures,
  selectHydratedAt,
}
