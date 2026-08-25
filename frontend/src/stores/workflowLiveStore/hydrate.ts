import { api, isRateLimitError } from '@/api'
import { Collections } from '@/utils/collections'
import type { LiveDispatchInfo, WorkflowLiveStateResponse } from '@/types'
import { extractError } from '../lib'
import { workflowExecutionStore } from '../workflowExecutionStore'
import { dispatchStore, MAX_TRACE_EVENTS } from '../dispatchStore'
import { agentTraceStore } from '../agentTraceStore'
import { store } from './_store'
import type { BaselineStepState, LiveDispatch } from './types'

/** Failed hydrations tolerated before optimistic flags are dropped. */
const UNCONFIRMED_LIMIT = 2

/** Fallback wait when the server throttles us without saying for how long. */
const DEFAULT_THROTTLE_MS = 10_000

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

/** A dispatch in one of these states will never produce another trace event. */
const TERMINAL_DISPATCH_STATUSES: ReadonlySet<string> = new Set([
  'completed',
  'failed',
  'cancelled',
])

/** A run in one of these states will never produce another timeline entry. */
const TERMINAL_RUN_STATUSES: ReadonlySet<string> = new Set([
  'completed',
  'failed',
  'cancelled',
])

/**
 * True when re-fetching this dispatch's trace could not tell us anything new.
 *
 * The poller runs for as long as the editor is open, and it used to re-download
 * every dispatch's full trace on every tick — a finished dispatch's trace is
 * immutable, so that was the same bytes over and over for the life of the page.
 *
 * Requires the execution id to match: a step's *latest* dispatch is what the
 * server reports, and a new one for the same step must not be mistaken for the
 * old one already being in hand.
 *
 * `trace_len` is compared against `MAX_TRACE_EVENTS` too, because a trace longer
 * than the view cap is stored truncated — without that the comparison could
 * never be satisfied and the fetch would repeat forever.
 */
const isTraceSettled = (dispatch: LiveDispatch): boolean => {
  if (!TERMINAL_DISPATCH_STATUSES.has(dispatch.status)) return false

  const entry = dispatchStore.selectByStep(dispatchStore.store.getState())[dispatch.stepId]
  if (entry?.executionId !== dispatch.executionId) return false

  return entry.trace.length >= Math.min(dispatch.traceLen, MAX_TRACE_EVENTS)
}

/**
 * Fetch a dispatch's trace from whichever store actually holds it.
 *
 * `registry` entries live in the server's in-memory task registry; `persisted`
 * ones only survive as an `agent_executions.trace` row, and their execution id
 * is not resolvable by the dispatch route.
 */
const fetchTrace = async (workflowId: string, dispatch: LiveDispatch): Promise<void> => {
  if (isTraceSettled(dispatch)) return

  try {
    const resp = dispatch.source === 'registry'
      ? await api.dispatch.trace(dispatch.executionId)
      : await api.workflows.getStepDispatchHistory(workflowId, dispatch.stepId)
    dispatchStore.hydrateFromApi(resp)
  } catch (e) {
    // Best-effort — a missing trace must not block the rest of the hydration.
    // A 429 is the exception worth recording: swallowing it renders as an empty
    // panel with no explanation, which is the failure this whole path exists to
    // avoid. Surface it so the next tick backs off instead of hammering.
    if (isRateLimitError(e)) {
      store.setState({
        throttledUntilMs: Date.now() + (e.retryAfterMs ?? DEFAULT_THROTTLE_MS),
      })
    }
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
    // Being throttled is not a failure. The view we already have is still
    // correct, just going stale, so keep it and wait the server out rather than
    // counting this against the failure budget and dropping the spinner.
    if (isRateLimitError(e)) {
      store.setState({
        loading: false,
        throttledUntilMs: Date.now() + (e.retryAfterMs ?? DEFAULT_THROTTLE_MS),
      })
      return
    }

    // `isGenerating` is set optimistically on click and only server truth can
    // confirm it. If we cannot reach the server we must not keep showing a
    // spinner we are unable to substantiate — a stuck spinner reads as a hung
    // system. The next successful tick restores it within one poll interval.
    const failures = store.getState().consecutiveFailures + 1
    store.setState({
      loading: false,
      error: extractError('workflowLive', e),
      consecutiveFailures: failures,
      ...(failures >= UNCONFIRMED_LIMIT ? { isGenerating: false, unconfirmedGenerating: 0 } : {}),
    })
    return
  }

  const run = live.active_run ?? live.latest_run
  // While the user is deliberately viewing a past run (via the Execution
  // panel's history selector), the poller must not touch `agentTraceStore` —
  // it would overwrite the traces of the run being viewed with the live run's.
  const isLive = workflowExecutionStore.selectViewMode(workflowExecutionStore.store.getState()) === 'live'

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

  if (isLive) {
    agentTraceStore.setHydratedRun(run?.id ?? null)
  }

  // The server saying "not generating" only overrides an optimistic flag once we
  // have spent its grace — see `setGenerating`. Server truth that agrees, or any
  // positive reading, settles it immediately.
  const grace = store.getState().unconfirmedGenerating
  const holdOptimistic = !live.generating && grace > 0
  const isGenerating = holdOptimistic ? true : live.generating
  const unconfirmedGenerating = holdOptimistic ? grace - 1 : 0

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
    isGenerating,
    unconfirmedGenerating,
    loading: false,
    error: null,
    consecutiveFailures: 0,
    throttledUntilMs: null,
    hydratedAt: live.server_time,
  })

  // Drop rows for steps the server no longer reports — view buffer only.
  dispatchStore.pruneToSteps(Collections.mapBy(dispatches, (d) => d.stepId))

  await Promise.all(Collections.mapBy(dispatches, (d) => fetchTrace(workflowId, d)))

  if (run === null) {
    await workflowExecutionStore.applyWorkshopFallback(workflowId)
    return
  }

  // Debug WS events (`NEXOR_DEBUG_STREAM`) are an opt-in dev flag and off by
  // default — `execution_messages` rows are written unconditionally, but
  // nothing pushes them to the client without that flag. This REST fetch is
  // therefore often the *only* source of trace data, so it has to keep
  // re-polling on every tick like the rest of this function, not just once —
  // a run that has nothing yet can absolutely have something on the next
  // tick. `hydrateFromTimeline`'s merge only ever keeps the richer version of
  // each agent's trace, so re-fetching never discards WS-delivered data.
  //
  // Once the run is finished and its timeline is in hand, that stops being true
  // in the only direction that matters: there is nothing left to arrive, so the
  // fetch can only ever return what we already have. This is the largest call in
  // the tick, so skipping it is most of the idle cost.
  const timelineSettled =
    TERMINAL_RUN_STATUSES.has(run.status) &&
    agentTraceStore.selectTimelineRunId(agentTraceStore.store.getState()) === run.id

  if (isLive && !timelineSettled) {
    try {
      await agentTraceStore.hydrateFromTimeline(run.id)
    } catch (e) {
      if (isRateLimitError(e)) {
        store.setState({
          throttledUntilMs: Date.now() + (e.retryAfterMs ?? DEFAULT_THROTTLE_MS),
        })
      }
    }
  }
}

/** Re-hydrate whichever workflow is currently loaded. No-op when there is none. */
const hydrateActive = async (): Promise<void> => {
  const { workflowId } = store.getState()
  if (workflowId === null) return
  await hydrateLiveState(workflowId)
}

export { hydrateLiveState, hydrateActive, toBaseline, toDispatch, UNCONFIRMED_LIMIT, DEFAULT_THROTTLE_MS }
