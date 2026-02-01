import { createContext, useReducer, useEffect, useCallback, useRef, type ReactNode } from 'react'
import { ACTION, STATS_POLL_INTERVAL_MS, USE_MOCK_DATA, API } from '@/constants'
import { api } from '@/api'
import { mock } from '@/mock'
import type { UsageSummary } from '@/types/stats'

// ── State ────────────────────────────────────────────────────────────────────

type StatsState = {
  stats: UsageSummary[]
  loading: boolean
  error: string | null
}

const initialState: StatsState = { stats: [], loading: true, error: null }

// ── Actions ──────────────────────────────────────────────────────────────────

type StatsAction =
  | { type: typeof ACTION.SET_ALL; stats: UsageSummary[] }
  | { type: typeof ACTION.SET_LOADING; loading: boolean }
  | { type: typeof ACTION.SET_ERROR; error: string }

const reducer = (state: StatsState, action: StatsAction): StatsState => {
  switch (action.type) {
    case ACTION.SET_ALL:
      return { stats: action.stats, loading: false, error: null }
    case ACTION.SET_LOADING:
      return { ...state, loading: action.loading }
    case ACTION.SET_ERROR:
      return { ...state, loading: false, error: action.error }
  }
}

// ── Context ──────────────────────────────────────────────────────────────────

type StatsContextValue = StatsState

const StatsContext = createContext<StatsContextValue | null>(null)

// ── Provider ─────────────────────────────────────────────────────────────────

function StatsProvider({ children }: { children: ReactNode }) {
  const [state, dispatch] = useReducer(reducer, initialState)
  const mountedRef = useRef(true)

  const load = useCallback(async () => {
    try {
      const stats = USE_MOCK_DATA
        ? await mock.getStats()
        : await api.get<UsageSummary[]>(API.STATS)
      if (mountedRef.current) dispatch({ type: ACTION.SET_ALL, stats })
    } catch (e) {
      if (mountedRef.current) dispatch({ type: ACTION.SET_ERROR, error: e instanceof Error ? e.message : 'Failed to load stats' })
    }
  }, [])

  useEffect(() => {
    mountedRef.current = true
    load()
    const interval = setInterval(load, STATS_POLL_INTERVAL_MS)
    return () => {
      mountedRef.current = false
      clearInterval(interval)
    }
  }, [load])

  return (
    <StatsContext.Provider value={state}>
      {children}
    </StatsContext.Provider>
  )
}

export { StatsContext, StatsProvider }
export type { StatsContextValue }
