// ============================================================================
// dispatchStore — Tracks active dispatch tasks per step
// ============================================================================

import { createStore } from './lib'
import { SESSION_EVENT } from '@/types/ws'
import type { WsWireMessage } from '@/types/ws'

// ── Types ───────────────────────────────────────────────────────────────────

type DispatchStatus = 'running' | 'completed' | 'failed' | 'cancelled'

type DispatchEntry = {
  executionId: string
  stepId: string
  status: DispatchStatus
  instruction: string
  message: string | null
  summary: string | null
  error: string | null
  startedAt: string
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
      }
      store.setState((s) => ({ byStep: { ...s.byStep, [stepId]: entry } }))
      break
    }
    case SESSION_EVENT.DISPATCH_PROGRESS: {
      store.setState((s) => {
        const existing = s.byStep[stepId]
        if (!existing) return s
        return { byStep: { ...s.byStep, [stepId]: { ...existing, message: (data.message as string | undefined) ?? null } } }
      })
      break
    }
    case SESSION_EVENT.DISPATCH_COMPLETED: {
      store.setState((s) => {
        const existing = s.byStep[stepId]
        if (!existing) return s
        return { byStep: { ...s.byStep, [stepId]: { ...existing, status: 'completed', summary: (data.summary as string | undefined) ?? null } } }
      })
      scheduleCleanup(stepId)
      break
    }
    case SESSION_EVENT.DISPATCH_FAILED: {
      store.setState((s) => {
        const existing = s.byStep[stepId]
        if (!existing) return s
        return { byStep: { ...s.byStep, [stepId]: { ...existing, status: 'failed', error: (data.error as string | undefined) ?? null } } }
      })
      scheduleCleanup(stepId)
      break
    }
    case SESSION_EVENT.DISPATCH_CANCELLED: {
      store.setState((s) => {
        const existing = s.byStep[stepId]
        if (!existing) return s
        return { byStep: { ...s.byStep, [stepId]: { ...existing, status: 'cancelled' } } }
      })
      scheduleCleanup(stepId)
      break
    }
  }
}

// ── Export ───────────────────────────────────────────────────────────────────

export const dispatchStore = {
  store,
  selectByStepId,
  selectActiveForStep,
  handleWsEvent,
}

export type { DispatchEntry, DispatchState, DispatchStatus }
