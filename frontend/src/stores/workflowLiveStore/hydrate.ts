import { api } from '@/api'
import { Collections } from '@/utils/collections'
import type { LiveDispatchInfo, WorkflowLiveStateResponse } from '@/types'
import { extractError } from '../lib'
import { workflowExecutionStore } from '../workflowExecutionStore'
import { dispatchStore } from '../dispatchStore'
import { agentTraceStore } from '../agentTraceStore'
import { store } from './_store'
import type { BaselineStepState, LiveDispatch } from './types'

/** Failed hydrations tolerated before optimistic flags are dropped. */
const UNCONFIRMED_LIMIT = 2

const toBaseline = (s: WorkflowLiveStateResponse['steps'][number]): BaselineStepState => ({
  stepId: s.step_id,
  name: s.name,
  executionMode: s.execution_mode,
  baselineStatus: s.baseline_status,
  pinned: s.pinned,
  hasRunSummary: s.has_run_summary,
  isRunningInActiveRun: s.is_running_in_active_run,
})

const toDispatch = (d: LiveDispatchInfo): LiveDispatch => ({
  stepId: d.step_id,
  executionId: d.execution_id,
  status: d.status,
  instruction: d.instruction,
  createdAt: d.created_at,
  result: d.result,
  traceLen: d.trace_len,
  source: d.source,
})

/**
 * Fetch a dispatch's trace from whichever store actually holds it.
 *
 * `registry` entries live in the server's in-memory task registry; `persisted`
 * ones only survive as an `agent_executions.trace` row, and their execution id
 * is not resolvable by the dispatch route.
 */
const fetchTrace = async (workflowId: string, dispatch: LiveDispatch): Promise<void> => {
  try {
    const resp = dispatch.source === 'registry'
      ? await api.dispatch.trace(dispatch.executionId)
      : await api.workflows.getStepDispatchHistory(workflowId, dispatch.stepId)
    dispatchStore.hydrateFromApi(resp)
  } catch {
    // Best-effort — a missing trace must not block the rest of the hydration.
  }
}

/**
 * Rebuild the editor's view of a workflow from a single server call.
 *
 * This is the REST counterpart of `WsStoreRouter`: everything a refresh would
 * otherwise lose — which run is current, per-step results, which nodes are
 * being generated, and each dispatch's trace — comes back from here.
 */
const hydrateLiveState = async (workflowId: string): Promise<void> => {
  store.setState({ workflowId, loading: true, error: null })

  let live: WorkflowLiveStateResponse
  try {
    live = await api.workflows.getLiveState(workflowId)
  } catch (e) {
    // `isGenerating` is set optimistically on click and only server truth can
    // confirm it. If we cannot reach the server we must not keep showing a
    // spinner we are unable to substantiate — a stuck spinner reads as a hung
    // system. The next successful tick restores it within one poll interval.
    const failures = store.getState().consecutiveFailures + 1
    store.setState({
      loading: false,
      error: extractError('workflowLive', e),
      consecutiveFailures: failures,
      ...(failures >= UNCONFIRMED_LIMIT ? { isGenerating: false } : {}),
    })
    return
  }

  const run = live.active_run ?? live.latest_run

  workflowExecutionStore.applyServerRun({
    runId: run?.id ?? null,
    workflowId,
    status: run?.status ?? null,
    startedAt: run?.started_at ?? null,
    completedAt: run?.completed_at ?? null,
    durationMs: null,
    error: run?.error ?? null,
    steps: live.run_steps,
  })

  agentTraceStore.setHydratedRun(run?.id ?? null)

  const dispatches = Collections.mapBy(live.dispatches, toDispatch)
  const baselineByStep: Record<string, BaselineStepState> = {}
  for (const step of live.steps) {
    baselineByStep[step.step_id] = toBaseline(step)
  }

  store.setState({
    workflowId,
    baselineByStep,
    dispatches,
    runSteps: live.run_steps,
    isGenerating: live.generating,
    loading: false,
    error: null,
    consecutiveFailures: 0,
    hydratedAt: live.server_time,
  })

  // Drop rows for steps the server no longer reports — view buffer only.
  dispatchStore.pruneToSteps(Collections.mapBy(dispatches, (d) => d.stepId))

  await Promise.all(Collections.mapBy(dispatches, (d) => fetchTrace(workflowId, d)))

  if (run === null) {
    await workflowExecutionStore.applyWorkshopFallback(workflowId)
    return
  }

  // Only worth fetching when we have nothing for this run — i.e. right after a
  // refresh. Once traces exist, WebSocket keeps them current.
  if (agentTraceStore.selectOrder(agentTraceStore.store.getState()).length === 0) {
    await agentTraceStore.hydrateFromTimeline(run.id)
  }
}

/** Re-hydrate whichever workflow is currently loaded. No-op when there is none. */
const hydrateActive = async (): Promise<void> => {
  const { workflowId } = store.getState()
  if (workflowId === null) return
  await hydrateLiveState(workflowId)
}

export { hydrateLiveState, hydrateActive, toBaseline, toDispatch }
