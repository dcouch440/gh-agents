// ============================================================================
// reviewQueueStore — Store for pending review executions + notifications
// ============================================================================

import { createStore } from './lib'
import { api } from '@/api'
import type { AgentExecution } from '@/types/execution'

// ── State ────────────────────────────────────────────────────────────────────

type ReviewQueueState = {
  executions: AgentExecution[]
  notification: { id: string; message: string } | null
  loading: boolean
  error: string | null
}

// ── Store ────────────────────────────────────────────────────────────────────

const store = createStore<ReviewQueueState>(() => ({
  executions: [],
  notification: null,
  loading: false,
  error: null,
}))

// ── Helpers ──────────────────────────────────────────────────────────────────

const extractError = (e: unknown): string =>
  e instanceof Error ? e.message : 'reviewQueue: unknown error'

// ── Selectors ────────────────────────────────────────────────────────────────

const selectExecutions = (s: ReviewQueueState): AgentExecution[] => s.executions

const selectPendingCount = (s: ReviewQueueState): number => s.executions.length

const selectNotification = (s: ReviewQueueState): ReviewQueueState['notification'] =>
  s.notification

const selectLoading = (s: ReviewQueueState): boolean => s.loading

const selectError = (s: ReviewQueueState): string | null => s.error

// ── Actions ──────────────────────────────────────────────────────────────────

const fetchPending = async (): Promise<void> => {
  store.setState({ loading: true, error: null })
  try {
    const executions = await api.agentExecutions.list({ status: 'awaiting_user' })
    store.setState({ executions, loading: false, error: null })
  } catch (e) {
    store.setState({ loading: false, error: extractError(e) })
  }
}

const addExecution = (execution: AgentExecution): void => {
  store.setState((s) => {
    const exists = s.executions.some((e) => e.id === execution.id)
    if (exists) return s
    return {
      executions: [execution, ...s.executions],
      notification: {
        id: execution.id,
        message: 'New review awaiting your approval',
      },
    }
  })
}

const removeExecution = (id: string): void => {
  store.setState((s) => ({
    executions: s.executions.filter((e) => e.id !== id),
  }))
}

const dismissNotification = (): void => {
  store.setState({ notification: null })
}

// ── Export ────────────────────────────────────────────────────────────────────

export const reviewQueueStore = {
  store,
  selectExecutions,
  selectPendingCount,
  selectNotification,
  selectLoading,
  selectError,
  fetchPending,
  addExecution,
  removeExecution,
  dismissNotification,
}

export type { ReviewQueueState }
