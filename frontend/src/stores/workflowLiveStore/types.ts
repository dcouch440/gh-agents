import type { BaselineStatus, DispatchSource, RunStepResult } from '@/types'

/**
 * Design-time truth for one node — pinned state, run summaries, whether it has
 * been configured. Deliberately independent of any run, so it survives a new
 * run starting and is never cleared when the run overlay is swapped.
 */
type BaselineStepState = {
  stepId: string
  name: string | null
  executionMode: string
  baselineStatus: BaselineStatus
  pinned: boolean
  hasRunSummary: boolean
  isRunningInActiveRun: boolean
}

type LiveDispatch = {
  stepId: string
  executionId: string
  status: string
  instruction: string
  createdAt: string
  result: string | null
  traceLen: number
  source: DispatchSource
}

type WorkflowLiveState = {
  workflowId: string | null
  baselineByStep: Readonly<Record<string, BaselineStepState>>
  /** Server order: newest-first, at most one per step. */
  dispatches: readonly LiveDispatch[]
  /** Per-step results for the run currently on screen. */
  runSteps: readonly RunStepResult[]
  isGenerating: boolean
  loading: boolean
  error: string | null
  /** Consecutive failed hydrations. Drives backoff and optimistic-flag expiry. */
  consecutiveFailures: number
  hydratedAt: string | null
}

export type { BaselineStepState, LiveDispatch, WorkflowLiveState }
