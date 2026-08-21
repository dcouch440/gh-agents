import type { BaselineStepState, LiveDispatch } from '@/stores/workflowLiveStore'
import type { StepExecutionState, StepExecutionStatus } from '@/stores/workflowExecutionStore'
import type { SourceStreamStatus } from '@/stores/stepStreamStore'

type ResolveNodeStatusInput = {
  /** Design-time truth. Survives run changes — this is what keeps pinned green. */
  readonly baseline: BaselineStepState | null
  /** The current run's overlay for this step, if the run has reached it. */
  readonly runState: StepExecutionState | undefined
  /** Most recent dispatch (generation) for this step. */
  readonly dispatch: LiveDispatch | null
}

type ResolvedNode = {
  readonly status: StepExecutionStatus
  readonly designStatus: SourceStreamStatus | null
  readonly pinned: boolean
}

/**
 * Collapse the baseline and live-run layers into what one node should show.
 *
 * Mirrors the server's `resolve_node_status` priority so the sidebar and the
 * manager agent never describe a node differently.
 *
 * The layering is the point: the run overlay is scoped to one run id and is
 * cleared whenever the run changes, while the baseline is re-fetched from the
 * server and is never cleared. A pinned node therefore keeps reading as
 * completed across a new run and across a refresh — not because anything was
 * copied or preserved, but because it was never in the overlay to begin with.
 */
const resolveNodeStatus = ({ baseline, runState, dispatch }: ResolveNodeStatusInput): ResolvedNode => {
  const pinned = baseline?.pinned ?? false

  // 1. Being generated right now. Sourced from the live-state endpoint rather
  //    than a WebSocket event, which is why it survives a refresh.
  if (dispatch !== null && dispatch.status === 'running') {
    return { status: 'idle', designStatus: 'running', pinned }
  }

  // 2/3. The current run has reached this step — its state wins over any baseline.
  if (runState !== undefined && runState.status !== 'idle' && runState.status !== 'pending') {
    return { status: runState.status, designStatus: null, pinned }
  }

  // 4. Persisted completion: pinned, or a stored run summary.
  if (pinned || baseline?.hasRunSummary === true) {
    return { status: 'success', designStatus: null, pinned }
  }

  // 5. The last generation for this node failed.
  if (dispatch !== null && dispatch.status === 'failed') {
    return { status: 'error', designStatus: 'failed', pinned }
  }

  // 6. Agents have been designed, but nothing has run.
  if (baseline?.baselineStatus === 'configured') {
    return { status: 'idle', designStatus: 'completed', pinned }
  }

  if (baseline?.baselineStatus === 'error') {
    return { status: 'error', designStatus: null, pinned }
  }

  // 7. Drawn but not yet configured — fall back to the run overlay if present.
  return { status: runState?.status ?? 'idle', designStatus: null, pinned }
}

export { resolveNodeStatus }
export type { ResolveNodeStatusInput, ResolvedNode }
