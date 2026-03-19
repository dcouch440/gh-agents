// ============================================================================
// dispatchStore — Tracks active dispatch tasks per step with execution trace
// ============================================================================

import { Collections } from '@/utils/collections'
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

const selectAllStepIds = (s: DispatchState): readonly string[] =>
  Object.keys(s.byStep)

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
      const message = (data.message as string | undefined) ?? null
      updateEntry(stepId, (e) => ({
        ...e,
        message,
        trace: message !== null
          ? [...e.trace, { type: 'phase_marker' as const, label: message, ts: msg.ts }]
          : e.trace,
      }))
      break
    }
    case SESSION_EVENT.DISPATCH_COMPLETED: {
      updateEntry(stepId, (e) => ({
        ...e,
        status: 'completed',
        summary: (data.summary as string | undefined) ?? null,
      }))

      break
    }
    case SESSION_EVENT.DISPATCH_FAILED: {
      updateEntry(stepId, (e) => ({
        ...e,
        status: 'failed',
        error: (data.error as string | undefined) ?? null,
      }))

      break
    }
    case SESSION_EVENT.DISPATCH_CANCELLED: {
      updateEntry(stepId, (e) => ({ ...e, status: 'cancelled' }))

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
    case SESSION_EVENT.DISPATCH_STREAM_SYSTEM_PROMPT: {
      updateEntry(stepId, (e) => ({
        ...e,
        trace: [
          ...e.trace,
          {
            type: 'system_prompt' as const,
            content: data.content as string,
            agentName: (data.agent_name as string | null) ?? null,
            ts: msg.ts,
          },
        ],
      }))
      break
    }
    case SESSION_EVENT.DISPATCH_STREAM_USER_MESSAGE: {
      updateEntry(stepId, (e) => ({
        ...e,
        trace: [
          ...e.trace,
          {
            type: 'user_message' as const,
            content: data.content as string,
            agentName: (data.agent_name as string | null) ?? null,
            ts: msg.ts,
          },
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
    case 'system_prompt':
      return { type: 'system_prompt', content: e.content, agentName: e.agent_name, ts: e.ts }
    case 'user_message':
      return { type: 'user_message', content: e.content, agentName: e.agent_name, ts: e.ts }
  }
}

const hydrateFromApi = (resp: DispatchTraceResponse): void => {
  const stepId = resp.step_id
  // Don't overwrite a running entry with stale API data
  const existing = store.getState().byStep[stepId]
  if (existing?.status === 'running') return

  const trace = Collections.mapBy(resp.trace, mapApiTraceEvent)
  const tokenBuffer = Collections.filterMap(trace, (e) =>
    e.type === 'token' ? e.content : null,
  ).join('')

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
}

// ── Export ───────────────────────────────────────────────────────────────────

export const dispatchStore = {
  store,
  selectByStepId,
  selectActiveForStep,
  selectTrace,
  selectToolEvents,
  selectTokenBuffer,
  selectAllStepIds,
  handleWsEvent,
  hydrateFromApi,
}

export type { DispatchEntry, DispatchState, DispatchStatus, DispatchTraceEvent }
