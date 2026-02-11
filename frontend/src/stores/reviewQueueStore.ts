// ============================================================================
// reviewQueueStore — Store for pending review executions + notifications
// ============================================================================

import { createStore, extractError } from './lib'
import { api } from '@/api'
import type { AgentExecution } from '@/types/execution'

// ── State ────────────────────────────────────────────────────────────────────

type ReviewQueueState = {
  executions: AgentExecution[]
  executionIds: ReadonlySet<string>
  notification: { id: string; message: string } | null
  loading: boolean
  error: string | null
}

// ── Store ────────────────────────────────────────────────────────────────────

const store = createStore<ReviewQueueState>(() => ({
  executions: [],
  executionIds: new Set<string>(),
  notification: null,
  loading: false,
  error: null,
}))

// ── Selectors ────────────────────────────────────────────────────────────────

const selectExecutions = (s: ReviewQueueState): AgentExecution[] => s.executions

const selectPendingCount = (s: ReviewQueueState): number => s.executions.length

const selectNotification = (s: ReviewQueueState): ReviewQueueState['notification'] => s.notification

const selectLoading = (s: ReviewQueueState): boolean => s.loading

const selectError = (s: ReviewQueueState): string | null => s.error

// ── Actions ──────────────────────────────────────────────────────────────────

const fetchPending = async (): Promise<void> => {
  store.setState({ loading: true, error: null })
  try {
    const executions = await api.agentExecutions.list({ status: 'awaiting_user' })
    store.setState({
      executions,
      executionIds: new Set(executions.map((e) => e.id)),
      loading: false,
      error: null,
    })
  } catch (e) {
    store.setState({ loading: false, error: extractError('reviewQueue', e) })
  }
}

const addExecution = (execution: AgentExecution): void => {
  store.setState((s) => {
    if (s.executionIds.has(execution.id)) return s
    const nextIds = new Set(s.executionIds)
    nextIds.add(execution.id)
    return {
      executions: [execution, ...s.executions],
      executionIds: nextIds,
      notification: {
        id: execution.id,
        message: 'New review awaiting your approval',
      },
    }
  })
}

const removeExecution = (id: string): void => {
  store.setState((s) => {
    const nextIds = new Set(s.executionIds)
    nextIds.delete(id)
    return {
      executions: s.executions.filter((e) => e.id !== id),
      executionIds: nextIds,
    }
  })
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
