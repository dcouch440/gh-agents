// ============================================================================
// activityStore — Flight recorder: independent, append-only event log
//
// Captures every parsed WebSocket event as ground truth for debugging.
// Compare this log against domain store state to find mismatches.
// In dev mode, exposed on window.__activityStore for console inspection.
// ============================================================================

import { createStore } from '../lib'
import { parseWsEvent } from './parseWsEvent'
import { activityMessage } from './activityMessages'
import type { WsWireMessage } from '@/types/ws'
import { ACTIVITY } from '@/types/activity'
import type { ActivityEvent, ActivityTopic } from '@/types/activity'

// ── Types ────────────────────────────────────────────────────────────────────

type ActivityEntry = {
  id: string
  seq: number
  event: ActivityEvent
  ts: string
  runId: string | null
  userId: string | null
  receivedAt: number
}

type ActivityState = {
  entries: ActivityEntry[]
  maxSize: number
}

// ── Constants ────────────────────────────────────────────────────────────────

const ACTIVITY_LOG_MAX_SIZE = 500

// ── Store ────────────────────────────────────────────────────────────────────

let seqCounter = 0

const initialState: ActivityState = {
  entries: [],
  maxSize: ACTIVITY_LOG_MAX_SIZE,
}

const store = createStore<ActivityState>(() => ({ ...initialState }))

// ── Selectors ────────────────────────────────────────────────────────────────

const selectAll = (s: ActivityState): ActivityEntry[] => s.entries

const selectByRunId =
  (runId: string) =>
  (s: ActivityState): ActivityEntry[] =>
    s.entries.filter((e) => e.runId === runId)

const selectByTopic =
  (topic: ActivityTopic) =>
  (s: ActivityState): ActivityEntry[] =>
    s.entries.filter((e) => e.event.type.startsWith(`${topic}:`))

const ERROR_TYPES = new Set<ActivityEvent['type']>([ACTIVITY.WORKFLOW_STEP_FAILED, ACTIVITY.WORKFLOW_FAILED])

const selectErrors = (s: ActivityState): ActivityEntry[] => s.entries.filter((e) => ERROR_TYPES.has(e.event.type))

const selectLatest =
  (n: number) =>
  (s: ActivityState): ActivityEntry[] =>
    s.entries.slice(-n)

const selectCount = (s: ActivityState): number => s.entries.length

// ── Mutations ────────────────────────────────────────────────────────────────

const append = (entry: ActivityEntry): void => {
  store.setState((s) => {
    const next = [...s.entries, entry]
    if (next.length > s.maxSize) {
      return { entries: next.slice(next.length - s.maxSize) }
    }
    return { entries: next }
  })
}

const handleWsEvent = (msg: WsWireMessage): void => {
  const event = parseWsEvent(msg)
  if (event === null) return

  const entry: ActivityEntry = {
    id: `act_${seqCounter}`,
    seq: seqCounter++,
    event,
    ts: msg.ts,
    runId: msg.run_id ?? null,
    userId: msg.user_id ?? null,
    receivedAt: Date.now(),
  }

  append(entry)
}

const reset = (): void => {
  seqCounter = 0
  store.setState({ ...initialState })
}

// ── Debug Utilities ──────────────────────────────────────────────────────────

const dump = (): void => {
  const { entries } = store.getState()
  const rows = entries.map((e) => ({
    seq: e.seq,
    type: e.event.type,
    message: activityMessage(e.event),
    ts: e.ts,
    runId: e.runId,
    receivedAt: new Date(e.receivedAt).toISOString(),
  }))
  // eslint-disable-next-line no-console -- intentional debug utility for console inspection
  console.table(rows)
}

const entries = (): ActivityEntry[] => store.getState().entries

const errors = (): ActivityEntry[] => selectErrors(store.getState())

const forRun = (runId: string): ActivityEntry[] => selectByRunId(runId)(store.getState())

// ── Dev Tools Exposure ───────────────────────────────────────────────────────

if (import.meta.env.DEV) {
  ;(window as Record<string, unknown>).__activityStore = { dump, entries, errors, forRun, reset }
}

// ── Export ────────────────────────────────────────────────────────────────────

export const activityStore = {
  store,
  selectAll,
  selectByRunId,
  selectByTopic,
  selectErrors,
  selectLatest,
  selectCount,
  handleWsEvent,
  append,
  dump,
  reset,
}

export type { ActivityEntry, ActivityState }
