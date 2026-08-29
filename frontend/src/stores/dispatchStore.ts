// ============================================================================
// dispatchStore — Tracks active dispatch tasks per step with execution trace
// ============================================================================

import { Collections } from '@/utils/collections'
import { createStore, memoFactory } from './lib'
import { SESSION_EVENT } from '@/types/ws'
import type { WsWireMessage } from '@/types/ws'
import type { ApiTraceEvent, DispatchTraceResponse } from '@/types/dispatch'

// ── Types ───────────────────────────────────────────────────────────────────

type DispatchStatus = 'running' | 'completed' | 'failed' | 'cancelled'

type DispatchTraceEvent =
  | { type: 'token'; content: string; ts: string }
  | { type: 'tool_start'; toolName: string; toolId: string; input: Record<string, unknown>; ts: string }
  | { type: 'tool_end'; toolName: string; toolId: string; result: unknown; ts: string }
  | { type: 'error'; error: string; ts: string }
  | { type: 'phase_marker'; label: string; ts: string }
  | { type: 'system_prompt'; content: string; agentName: string | null; ts: string }
  | { type: 'user_message'; content: string; agentName: string | null; ts: string }

type DispatchEntry = {
  executionId: string
  stepId: string
  status: DispatchStatus
  instruction: string
  message: string | null
  summary: string | null
  error: string | null
  startedAt: string
  trace: DispatchTraceEvent[]
  tokenBuffer: string
}

type DispatchState = {
  byStep: Record<string, DispatchEntry>
}

const MAX_TRACE_EVENTS = 2000

// ── Store ───────────────────────────────────────────────────────────────────

const store = createStore<DispatchState>(() => ({
  byStep: {},
}))

// ── Selectors ───────────────────────────────────────────────────────────────

const selectByStepId = memoFactory(
  (stepId: string) =>
  (s: DispatchState): DispatchEntry | null =>
    s.byStep[stepId] ?? null,
)

const selectActiveForStep = memoFactory(
  (stepId: string) =>
  (s: DispatchState): DispatchEntry | null => {
    const entry = s.byStep[stepId]
    return entry?.status === 'running' ? entry : null
  },
)

const selectTrace = memoFactory(
  (stepId: string) =>
  (s: DispatchState): DispatchTraceEvent[] =>
    s.byStep[stepId]?.trace ?? [],
)

const selectToolEvents = memoFactory(
  (stepId: string) =>
  (s: DispatchState): DispatchTraceEvent[] =>
    (s.byStep[stepId]?.trace ?? []).filter(
      (e) => e.type === 'tool_start' || e.type === 'tool_end'
    ),
)

const selectTokenBuffer = memoFactory(
  (stepId: string) =>
  (s: DispatchState): string =>
    s.byStep[stepId]?.tokenBuffer ?? '',
)

const selectAllStepIds = (s: DispatchState): readonly string[] =>
  Object.keys(s.byStep)

const selectByStep = (s: DispatchState): Readonly<Record<string, DispatchEntry>> => s.byStep

// ── Helpers ─────────────────────────────────────────────────────────────────

const makeDefaultEntry = (stepId: string, ts: string): DispatchEntry => ({
  executionId: '',
  stepId,
  status: 'running',
  instruction: '',
  message: null,
  summary: null,
  error: null,
  startedAt: ts,
  trace: [],
  tokenBuffer: '',
})

/**
 * Update a step's entry, creating one if a stream event arrived before the
 * `dispatch_started` event or before REST hydration. Dropping those events is
 * how the panel used to end up blank after a refresh mid-dispatch.
 */
const upsertEntry = (
  stepId: string,
  ts: string,
  updater: (entry: DispatchEntry) => DispatchEntry
): void => {
  store.setState((s) => {
    const existing = s.byStep[stepId] ?? makeDefaultEntry(stepId, ts)
    return { byStep: { ...s.byStep, [stepId]: updater(existing) } }
  })
}

/** Append a trace event, dropping the oldest events if the cap is exceeded. */
const appendTrace = (trace: DispatchTraceEvent[], event: DispatchTraceEvent): DispatchTraceEvent[] => {
  const next = [...trace, event]
  return next.length > MAX_TRACE_EVENTS ? next.slice(-MAX_TRACE_EVENTS) : next
}

// ── WS Handler ──────────────────────────────────────────────────────────────

const handleWsEvent = (msg: WsWireMessage): void => {
  const data = msg.data
  const stepId = data.step_id as string | undefined
  if (!stepId) return

  switch (msg.event) {
    case SESSION_EVENT.DISPATCH_STARTED: {
      const entry: DispatchEntry = {
        executionId: data.execution_id as string,
        stepId,
        status: 'running',
        instruction: (data.instruction as string | undefined) ?? '',
        message: null,
        summary: null,
        error: null,
        startedAt: msg.ts,
        trace: [],
        tokenBuffer: '',
      }
      store.setState((s) => ({ byStep: { ...s.byStep, [stepId]: entry } }))
      break
    }
    case SESSION_EVENT.DISPATCH_PROGRESS: {
      const message = (data.message as string | undefined) ?? null
      upsertEntry(stepId, msg.ts, (e) => ({
        ...e,
        message,
        trace: message !== null
          ? appendTrace(e.trace, { type: 'phase_marker' as const, label: message, ts: msg.ts })
          : e.trace,
      }))
      break
    }
    // The three terminal events upsert for the same reason the stream events do:
    // a socket that connects after `dispatch_started` (or reconnects across it)
    // still has to be able to report how the dispatch ended. Dropping them left
    // a row that had failed showing no status and no reason.
    case SESSION_EVENT.DISPATCH_COMPLETED: {
      upsertEntry(stepId, msg.ts, (e) => ({
        ...e,
        status: 'completed',
        summary: (data.summary as string | undefined) ?? null,
      }))

      break
    }
    case SESSION_EVENT.DISPATCH_FAILED: {
      upsertEntry(stepId, msg.ts, (e) => ({
        ...e,
        status: 'failed',
        error: (data.error as string | undefined) ?? null,
      }))

      break
    }
    case SESSION_EVENT.DISPATCH_CANCELLED: {
      upsertEntry(stepId, msg.ts, (e) => ({ ...e, status: 'cancelled' }))

      break
    }
    // ── Dispatch streaming events ─────────────────────────────────────
    case SESSION_EVENT.DISPATCH_STREAM_TOKEN: {
      const content = data.content as string
      upsertEntry(stepId, msg.ts, (e) => ({
        ...e,
        tokenBuffer: e.tokenBuffer + content,
        trace: appendTrace(e.trace, { type: 'token', content, ts: msg.ts }),
      }))
      break
    }
    case SESSION_EVENT.DISPATCH_STREAM_TOOL_START: {
      upsertEntry(stepId, msg.ts, (e) => ({
        ...e,
        trace: appendTrace(e.trace, {
          type: 'tool_start' as const,
          toolName: data.tool_name as string,
          toolId: data.tool_id as string,
          input: data.input as Record<string, unknown>,
          ts: msg.ts,
        }),
      }))
      break
    }
    case SESSION_EVENT.DISPATCH_STREAM_TOOL_END: {
      upsertEntry(stepId, msg.ts, (e) => ({
        ...e,
        trace: appendTrace(e.trace, {
          type: 'tool_end' as const,
          toolName: data.tool_name as string,
          toolId: data.tool_id as string,
          result: data.result,
          ts: msg.ts,
        }),
      }))
      break
    }
    case SESSION_EVENT.DISPATCH_STREAM_ERROR: {
      upsertEntry(stepId, msg.ts, (e) => ({
        ...e,
        trace: appendTrace(e.trace, { type: 'error' as const, error: data.error as string, ts: msg.ts }),
      }))
      break
    }
    case SESSION_EVENT.DISPATCH_STREAM_SYSTEM_PROMPT: {
      upsertEntry(stepId, msg.ts, (e) => ({
        ...e,
        trace: appendTrace(e.trace, {
          type: 'system_prompt' as const,
          content: data.content as string,
          agentName: (data.agent_name as string | null) ?? null,
          ts: msg.ts,
        }),
      }))
      break
    }
    case SESSION_EVENT.DISPATCH_STREAM_USER_MESSAGE: {
      upsertEntry(stepId, msg.ts, (e) => ({
        ...e,
        trace: appendTrace(e.trace, {
          type: 'user_message' as const,
          content: data.content as string,
          agentName: (data.agent_name as string | null) ?? null,
          ts: msg.ts,
        }),
      }))
      break
    }
  }
}

// ── Hydration ────────────────────────────────────────────────────────────

const mapApiTraceEvent = (e: ApiTraceEvent): DispatchTraceEvent => {
  switch (e.type) {
    case 'token':
      return { type: 'token', content: e.content, ts: e.ts }
    case 'tool_start':
      return { type: 'tool_start', toolName: e.tool_name, toolId: e.tool_id, input: e.input, ts: e.ts }
    case 'tool_end':
      return { type: 'tool_end', toolName: e.tool_name, toolId: e.tool_id, result: e.result, ts: e.ts }
    case 'error':
      return { type: 'error', error: e.error, ts: e.ts }
    case 'system_prompt':
      return { type: 'system_prompt', content: e.content, agentName: e.agent_name, ts: e.ts }
    case 'user_message':
      return { type: 'user_message', content: e.content, agentName: e.agent_name, ts: e.ts }
  }
}

const TERMINAL_STATUSES: ReadonlySet<DispatchStatus> = new Set<DispatchStatus>([
  'completed',
  'failed',
  'cancelled',
])

/**
 * Merge a REST trace into the view.
 *
 * WebSocket and REST race in both directions, so neither wins outright: keep
 * whichever trace is longer, but always accept a terminal status from the
 * server. Refusing to touch a `running` entry (the old behaviour) meant a
 * dispatch that finished while the socket was down stayed spinning forever.
 */
const hydrateFromApi = (resp: DispatchTraceResponse): void => {
  const stepId = resp.step_id
  const existing = store.getState().byStep[stepId]

  const allTrace = Collections.mapBy(resp.trace, mapApiTraceEvent)
  const incomingTrace = allTrace.length > MAX_TRACE_EVENTS ? allTrace.slice(-MAX_TRACE_EVENTS) : allTrace

  const incomingStatus = resp.status as DispatchStatus
  const keepLocalTrace =
    existing !== undefined && existing.trace.length > incomingTrace.length
  const trace = keepLocalTrace ? existing.trace : incomingTrace

  const tokenBuffer = keepLocalTrace
    ? existing.tokenBuffer
    : Collections.filterMap(trace, (e) => (e.type === 'token' ? e.content : null)).join('')

  // A terminal server status is authoritative; otherwise let a live local
  // 'running' survive a snapshot taken before the dispatch started.
  const status = TERMINAL_STATUSES.has(incomingStatus)
    ? incomingStatus
    : existing?.status ?? incomingStatus

  // The registry stores a failure's text in the task's `result`, so a failed
  // dispatch's result is an error message rather than a summary. Filing it as
  // a summary is how the reason for a failure used to vanish on refresh: the
  // only copy arrived over the socket, and nothing rendered it afterwards.
  const failed = status === 'failed'
  const result = resp.result ?? null
  const summary = failed ? existing?.summary ?? null : result ?? existing?.summary ?? null
  const error = failed ? result ?? existing?.error ?? null : existing?.error ?? null

  const entry: DispatchEntry = {
    executionId: resp.execution_id,
    stepId,
    status,
    instruction: resp.instruction || (existing?.instruction ?? ''),
    message: existing?.message ?? null,
    summary,
    error,
    startedAt: existing?.startedAt ?? '',
    trace,
    tokenBuffer,
  }

  store.setState((s) => ({ byStep: { ...s.byStep, [stepId]: entry } }))
}

/**
 * Drop view entries for steps the server no longer reports a dispatch for.
 *
 * View buffer only — this never deletes anything server-side.
 */
const pruneToSteps = (stepIds: readonly string[]): void => {
  const keep = Collections.toSet(stepIds)
  store.setState((s) => {
    const next: Record<string, DispatchEntry> = {}
    let changed = false
    for (const [stepId, entry] of Object.entries(s.byStep)) {
      if (keep.has(stepId)) {
        next[stepId] = entry
      } else {
        changed = true
      }
    }
    return changed ? { byStep: next } : s
  })
}

// ── Export ───────────────────────────────────────────────────────────────────

export const dispatchStore = {
  store,
  selectByStep,
  selectByStepId,
  selectActiveForStep,
  selectTrace,
  selectToolEvents,
  selectTokenBuffer,
  selectAllStepIds,
  handleWsEvent,
  hydrateFromApi,
  pruneToSteps,
}

export { MAX_TRACE_EVENTS }
export type { DispatchEntry, DispatchState, DispatchStatus, DispatchTraceEvent }
