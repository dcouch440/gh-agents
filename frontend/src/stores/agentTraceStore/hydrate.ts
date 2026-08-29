import { api, isRateLimitError } from '@/api'
import { Collections } from '@/utils/collections'
import type { TimelineEntry } from '@/types'
import { store } from './_store'
import type { AgentTrace, AgentTraceEvent } from './types'

/** How far back to rebuild the Run tab. Matches roughly one long run. */
const TIMELINE_LIMIT = 200

/**
 * `tool_call` content is a serialized payload, not a structured object. Parse it
 * when we can and keep the raw text when we cannot, so nothing is lost.
 */
const parseToolInput = (content: string): Record<string, unknown> => {
  try {
    const parsed: unknown = JSON.parse(content)
    if (typeof parsed === 'object' && parsed !== null && !Array.isArray(parsed)) {
      return parsed as Record<string, unknown>
    }
  } catch {
    // Fall through to the raw form.
  }
  return { raw: content }
}

/**
 * Fallback pairing, by position within one agent execution.
 *
 * Used only for rows written before the engine recorded `tool_call_id` on the
 * assistant side. It is approximate by nature: an agent can issue several tool
 * calls in one turn, so the Nth call is not reliably the Nth result. Anything
 * with an id pairs by id instead — see `toolIdFor`.
 *
 * A trailing call with no result keeps a distinct key and stays "running" —
 * which is correct: it genuinely never returned.
 */
const pairKey = (agentExecutionId: string, index: number): string =>
  `${agentExecutionId}#${String(index)}`

/** Pair by the provider's tool-call id, which both sides of a pair share. */
const idKey = (agentExecutionId: string, toolCallId: string): string =>
  `${agentExecutionId}#id:${toolCallId}`

const toEvent = (entry: TimelineEntry, toolId: string): AgentTraceEvent | null => {
  switch (entry.kind) {
    case 'system_prompt':
      return { type: 'system_prompt', content: entry.content, ts: entry.ts }
    case 'user_message':
      return { type: 'user_message', content: entry.content, ts: entry.ts }
    case 'assistant_message':
      return { type: 'assistant_message', content: entry.content, ts: entry.ts }
    case 'tool_call':
      return {
        type: 'tool_call',
        toolName: entry.tool_name ?? '',
        toolId,
        input: parseToolInput(entry.content),
        ts: entry.ts,
      }
    case 'tool_result':
      return {
        type: 'tool_result',
        toolName: entry.tool_name ?? '',
        toolId,
        result: entry.content,
        ts: entry.ts,
      }
  }
}

/**
 * Rebuild agent traces for a finished or in-flight run.
 *
 * The Run tab was previously WebSocket-only, so it was always empty after a
 * refresh. The timeline endpoint joins `execution_messages` to
 * `agent_executions`, which maps onto `AgentTrace` one-for-one.
 */
const hydrateFromTimeline = async (executionId: string): Promise<void> => {
  let entries: readonly TimelineEntry[]
  try {
    const resp = await api.workflows.getExecutionTimeline(executionId, TIMELINE_LIMIT)
    entries = resp.entries
  } catch (e) {
    // Throttling is the caller's problem — it owns the poll cadence and needs to
    // know to back off. Anything else is best-effort and must not block the rest
    // of the hydration; the caller polls repeatedly, so a failed attempt is
    // simply retried on the next tick.
    if (isRateLimitError(e)) throw e
    return
  }

  if (entries.length === 0) return

  const traces: Record<string, AgentTrace> = {}
  const order: string[] = []
  const callCounts = new Map<string, number>()
  const resultCounts = new Map<string, number>()

  // Results always carry a `tool_call_id`; calls only carry one if the engine
  // that wrote the run recorded it. So the calls decide which scheme an
  // execution uses, and it must be decided per execution before pairing —
  // keying results by id while their calls fall back to position would pair
  // nothing at all.
  const pairsById = new Set(
    Collections.filterMap(entries, (e) =>
      e.kind === 'tool_call' && e.tool_call_id !== null ? e.agent_execution_id : null,
    ),
  )

  // Entries arrive oldest-first, which is also the order we want to display.
  for (const entry of entries) {
    const agentId = entry.agent_execution_id
    let toolId = entry.id
    const callId = entry.tool_call_id
    const byId = pairsById.has(agentId) && callId !== null
    if (entry.kind === 'tool_call') {
      const n = callCounts.get(agentId) ?? 0
      callCounts.set(agentId, n + 1)
      toolId = byId ? idKey(agentId, callId) : pairKey(agentId, n)
    } else if (entry.kind === 'tool_result') {
      const n = resultCounts.get(agentId) ?? 0
      resultCounts.set(agentId, n + 1)
      toolId = byId ? idKey(agentId, callId) : pairKey(agentId, n)
    }

    const event = toEvent(entry, toolId)
    if (event === null) continue

    const existing = traces[entry.agent_execution_id]
    if (existing === undefined) {
      order.push(entry.agent_execution_id)
      traces[entry.agent_execution_id] = {
        agentExecutionId: entry.agent_execution_id,
        agentName: entry.agent_name,
        stepId: entry.step_id ?? '',
        events: [event],
      }
      continue
    }
    existing.events.push(event)
  }

  store.setState((s) => {
    // Keep whichever version of each agent's trace has more events. WS
    // delivers events in real time and can be ahead of what is committed to
    // the DB yet, but this fetch is polled repeatedly and needs to be free to
    // keep growing a trace across ticks — a version with fewer events is
    // never allowed to overwrite a richer one, regardless of which side it
    // came from.
    const merged: Record<string, AgentTrace> = { ...s.traces }
    for (const [id, fresh] of Object.entries(traces)) {
      const existing = merged[id]
      if (existing === undefined || fresh.events.length > existing.events.length) {
        merged[id] = fresh
      }
    }
    const seen = Collections.toSet(s.order)
    const mergedOrder = [...s.order, ...order.filter((id) => !seen.has(id))]
    // Stamped only on the success path, so a caller can skip re-fetching a
    // finished run's timeline. A failure above returns early and leaves this
    // unset, which is what makes the next tick retry.
    return { traces: merged, order: mergedOrder, timelineRunId: executionId }
  })
}

/**
 * Record which run the current traces belong to, so the Run tab can tell
 * "nothing has run yet" apart from "these traces predate this reload".
 */
const setHydratedRun = (runId: string | null): void => {
  store.setState((s) =>
    s.hydratedRunId === runId
      ? s
      : { hydratedRunId: runId, traces: {}, order: [], timelineRunId: null },
  )
}

export { hydrateFromTimeline, setHydratedRun, parseToolInput, toEvent, pairKey }
