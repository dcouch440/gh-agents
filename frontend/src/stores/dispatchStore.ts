// ============================================================================
// dispatchStore — Tracks active dispatch tasks per step with execution trace
// ============================================================================

import { createStore } from './lib'
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

// ── Store ───────────────────────────────────────────────────────────────────

const store = createStore<DispatchState>(() => ({
  byStep: {},
}))

// ── Selectors ───────────────────────────────────────────────────────────────

const selectByStepId =
  (stepId: string) =>
  (s: DispatchState): DispatchEntry | null =>
    s.byStep[stepId] ?? null

const selectActiveForStep =
  (stepId: string) =>
  (s: DispatchState): DispatchEntry | null => {
    const entry = s.byStep[stepId]
    return entry?.status === 'running' ? entry : null
  }

const selectTrace =
  (stepId: string) =>
  (s: DispatchState): DispatchTraceEvent[] =>
    s.byStep[stepId]?.trace ?? []

const selectToolEvents =
  (stepId: string) =>
  (s: DispatchState): DispatchTraceEvent[] =>
    (s.byStep[stepId]?.trace ?? []).filter(
      (e) => e.type === 'tool_start' || e.type === 'tool_end'
    )

const selectTokenBuffer =
  (stepId: string) =>
  (s: DispatchState): string =>
    s.byStep[stepId]?.tokenBuffer ?? ''

// ── Cleanup ─────────────────────────────────────────────────────────────────

const CLEANUP_DELAY = 30_000

const scheduleCleanup = (stepId: string): void => {
  setTimeout(() => {
    store.setState((s) => {
      const entry = s.byStep[stepId]
      if (!entry || entry.status === 'running') return s
      // eslint-disable-next-line @typescript-eslint/no-unused-vars
      const { [stepId]: _removed, ...rest } = s.byStep
      return { byStep: rest }
    })
  }, CLEANUP_DELAY)
}

// ── Helpers ─────────────────────────────────────────────────────────────────

const updateEntry = (
  stepId: string,
  updater: (entry: DispatchEntry) => DispatchEntry
): void => {
  store.setState((s) => {
    const existing = s.byStep[stepId]
    if (!existing) return s
    return { byStep: { ...s.byStep, [stepId]: updater(existing) } }
  })
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
      updateEntry(stepId, (e) => ({
        ...e,
        message: (data.message as string | undefined) ?? null,
      }))
      break
    }
    case SESSION_EVENT.DISPATCH_COMPLETED: {
      updateEntry(stepId, (e) => ({
        ...e,
        status: 'completed',
        summary: (data.summary as string | undefined) ?? null,
      }))
      scheduleCleanup(stepId)
      break
    }
    case SESSION_EVENT.DISPATCH_FAILED: {
      updateEntry(stepId, (e) => ({
        ...e,
        status: 'failed',
        error: (data.error as string | undefined) ?? null,
      }))
      scheduleCleanup(stepId)
      break
    }
    case SESSION_EVENT.DISPATCH_CANCELLED: {
      updateEntry(stepId, (e) => ({ ...e, status: 'cancelled' }))
      scheduleCleanup(stepId)
      break
    }
    // ── Dispatch streaming events ─────────────────────────────────────
    case SESSION_EVENT.DISPATCH_STREAM_TOKEN: {
      const content = data.content as string
      updateEntry(stepId, (e) => ({
        ...e,
        tokenBuffer: e.tokenBuffer + content,
        trace: [...e.trace, { type: 'token', content, ts: msg.ts }],
      }))
      break
    }
    case SESSION_EVENT.DISPATCH_STREAM_TOOL_START: {
      updateEntry(stepId, (e) => ({
        ...e,
        trace: [
          ...e.trace,
          {
            type: 'tool_start' as const,
            toolName: data.tool_name as string,
            toolId: data.tool_id as string,
            input: data.input as Record<string, unknown>,
            ts: msg.ts,
          },
        ],
      }))
      break
    }
    case SESSION_EVENT.DISPATCH_STREAM_TOOL_END: {
      updateEntry(stepId, (e) => ({
        ...e,
        trace: [
          ...e.trace,
          {
            type: 'tool_end' as const,
            toolName: data.tool_name as string,
            toolId: data.tool_id as string,
            result: data.result,
            ts: msg.ts,
          },
        ],
      }))
      break
    }
    case SESSION_EVENT.DISPATCH_STREAM_ERROR: {
      updateEntry(stepId, (e) => ({
        ...e,
        trace: [
          ...e.trace,
          { type: 'error' as const, error: data.error as string, ts: msg.ts },
        ],
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
  }
}

const hydrateFromApi = (resp: DispatchTraceResponse): void => {
  const stepId = resp.step_id
  // Don't overwrite a running entry with stale API data
  const existing = store.getState().byStep[stepId]
  if (existing?.status === 'running') return

  const trace = resp.trace.map(mapApiTraceEvent)
  const tokenBuffer = trace
    .filter((e): e is DispatchTraceEvent & { type: 'token' } => e.type === 'token')
    .map((e) => e.content)
    .join('')

  const entry: DispatchEntry = {
    executionId: resp.execution_id,
    stepId,
    status: resp.status as DispatchStatus,
    instruction: resp.instruction,
    message: null,
    summary: resp.result,
    error: null,
    startedAt: '',
    trace,
    tokenBuffer,
  }

  store.setState((s) => ({ byStep: { ...s.byStep, [stepId]: entry } }))

  // If the dispatch is already done, schedule cleanup
  if (resp.status !== 'running') {
    scheduleCleanup(stepId)
  }
}

// ── History hydration (from builder session messages) ──────────────────────

type HistoryMessage = {
  id: string
  role: 'user' | 'assistant'
  content: string
  timestamp: string
  source_type: string | null
}

/**
 * Hydrate a DispatchEntry from the builder session's persisted messages.
 * Used when the in-memory trace is gone (server restart / cleanup).
 * Creates a synthetic entry with the last user instruction and assistant response.
 */
const hydrateFromHistory = (stepId: string, messages: HistoryMessage[]): void => {
  if (messages.length === 0) return

  // Don't overwrite a running entry with historical data
  const existing = store.getState().byStep[stepId]
  if (existing?.status === 'running') return

  // Find the last user instruction and its following assistant response
  let lastUserIdx = -1
  for (let i = messages.length - 1; i >= 0; i--) {
    if (messages[i]?.role === 'user') {
      lastUserIdx = i
      break
    }
  }

  if (lastUserIdx === -1) return

  const userMsg = messages[lastUserIdx]
  if (!userMsg) return

  // Collect assistant response(s) that follow the last user message
  let responseContent = ''
  for (let i = lastUserIdx + 1; i < messages.length; i++) {
    const msg = messages[i]
    if (msg?.role === 'assistant') {
      responseContent += (responseContent.length > 0 ? '\n' : '') + msg.content
    }
  }

  const entry: DispatchEntry = {
    executionId: '',
    stepId,
    status: 'completed',
    instruction: userMsg.content,
    message: null,
    summary: responseContent.length > 0 ? responseContent : null,
    error: null,
    startedAt: userMsg.timestamp,
    trace: responseContent.length > 0
      ? [{ type: 'token', content: responseContent, ts: userMsg.timestamp }]
      : [],
    tokenBuffer: responseContent,
  }

  store.setState((s) => ({ byStep: { ...s.byStep, [stepId]: entry } }))
  scheduleCleanup(stepId)
}

// ── Export ───────────────────────────────────────────────────────────────────

export const dispatchStore = {
  store,
  selectByStepId,
  selectActiveForStep,
  selectTrace,
  selectToolEvents,
  selectTokenBuffer,
  handleWsEvent,
  hydrateFromApi,
  hydrateFromHistory,
}

export type { DispatchEntry, DispatchState, DispatchStatus, DispatchTraceEvent }
