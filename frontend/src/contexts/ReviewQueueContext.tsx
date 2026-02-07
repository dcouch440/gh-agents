import { createContext, useReducer, useEffect, useCallback, useRef, type ReactNode } from 'react'
import { ACTION } from '@/constants'
import { api } from '@/api'
import { NotificationSnackbar } from '@/components/primitives/NotificationSnackbar'
import type { AgentExecution } from '@/types/execution'

// ── State ────────────────────────────────────────────────────────────────────

type ReviewQueueState = {
  executions: AgentExecution[]
  loading: boolean
  error: string | null
  notification: { id: string; message: string } | null
}

const initialState: ReviewQueueState = {
  executions: [],
  loading: true,
  error: null,
  notification: null,
}

// ── Actions ──────────────────────────────────────────────────────────────────

type ReviewQueueAction =
  | { type: typeof ACTION.SET_QUEUE; executions: AgentExecution[] }
  | { type: typeof ACTION.ADD_TO_QUEUE; execution: AgentExecution }
  | { type: typeof ACTION.REMOVE_FROM_QUEUE; id: string }
  | { type: typeof ACTION.SET_LOADING; loading: boolean }
  | { type: typeof ACTION.SET_ERROR; error: string }
  | { type: typeof ACTION.DISMISS_NOTIFICATION }

const reducer = (state: ReviewQueueState, action: ReviewQueueAction): ReviewQueueState => {
  switch (action.type) {
    case ACTION.SET_QUEUE:
      return { ...state, executions: action.executions, loading: false, error: null }
    case ACTION.ADD_TO_QUEUE: {
      const exists = state.executions.some((e) => e.id === action.execution.id)
      if (exists) return state
      return {
        ...state,
        executions: [action.execution, ...state.executions],
        notification: {
          id: action.execution.id,
          message: 'New review awaiting your approval',
        },
      }
    }
    case ACTION.REMOVE_FROM_QUEUE:
      return {
        ...state,
        executions: state.executions.filter((e) => e.id !== action.id),
      }
    case ACTION.SET_LOADING:
      return { ...state, loading: action.loading }
    case ACTION.SET_ERROR:
      return { ...state, loading: false, error: action.error }
    case ACTION.DISMISS_NOTIFICATION:
      return { ...state, notification: null }
  }
}

// ── Context ──────────────────────────────────────────────────────────────────

type ReviewQueueContextValue = ReviewQueueState & {
  pendingCount: number
  dismissNotification: () => void
  reload: () => void
}

const ReviewQueueContext = createContext<ReviewQueueContextValue | null>(null)

// ── Provider ─────────────────────────────────────────────────────────────────

function ReviewQueueProvider({ children }: { children: ReactNode }) {
  const [state, dispatch] = useReducer(reducer, initialState)
  const mountedRef = useRef(true)

  const load = useCallback(async () => {
    dispatch({ type: ACTION.SET_LOADING, loading: true })
    try {
      const executions = await api.agentExecutions.list({ status: 'awaiting_user' })
      if (mountedRef.current) dispatch({ type: ACTION.SET_QUEUE, executions })
    } catch (e) {
      if (mountedRef.current) {
        dispatch({
          type: ACTION.SET_ERROR,
          error: e instanceof Error ? e.message : 'Failed to load review queue',
        })
      }
    }
  }, [])

  // Initial fetch
  useEffect(() => {
    mountedRef.current = true
    void load()
    return () => {
      mountedRef.current = false
    }
  }, [load])

  const dismissNotification = useCallback(() => {
    dispatch({ type: ACTION.DISMISS_NOTIFICATION })
  }, [])

  const value: ReviewQueueContextValue = {
    ...state,
    pendingCount: state.executions.length,
    dismissNotification,
    reload: () => {
      void load()
    },
  }

  return (
    <ReviewQueueContext.Provider value={value}>
      {children}
      <NotificationSnackbar
        open={state.notification !== null}
        message={state.notification?.message ?? ''}
        onClose={dismissNotification}
        severity="warning"
      />
    </ReviewQueueContext.Provider>
  )
}

export { ReviewQueueContext, ReviewQueueProvider }
export type { ReviewQueueContextValue }
