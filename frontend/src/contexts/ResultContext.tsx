import { createContext, useReducer, useEffect, useCallback, useRef, type ReactNode } from 'react'
import { ACTION } from '@/constants'
import { api } from '@/api'
import type { Result } from '@/types/result'

// ── State ────────────────────────────────────────────────────────────────────

type ResultState = {
  results: Result[]
  loading: boolean
  error: string | null
}

const initialState: ResultState = { results: [], loading: true, error: null }

// ── Actions ──────────────────────────────────────────────────────────────────

type ResultAction =
  | { type: typeof ACTION.SET_ALL; results: Result[] }
  | { type: typeof ACTION.SET_LOADING; loading: boolean }
  | { type: typeof ACTION.SET_ERROR; error: string }
  | { type: typeof ACTION.REMOVE_ONE; id: string }

const reducer = (state: ResultState, action: ResultAction): ResultState => {
  switch (action.type) {
    case ACTION.SET_ALL:
      return { results: action.results, loading: false, error: null }
    case ACTION.SET_LOADING:
      return { ...state, loading: action.loading }
    case ACTION.SET_ERROR:
      return { ...state, loading: false, error: action.error }
    case ACTION.REMOVE_ONE:
      return { ...state, results: state.results.filter((r) => r.id !== action.id) }
  }
}

// ── Context ──────────────────────────────────────────────────────────────────

type ResultContextValue = ResultState & { reload: () => void }

const ResultContext = createContext<ResultContextValue | null>(null)

// ── Provider ─────────────────────────────────────────────────────────────────

function ResultProvider({ children }: { children: ReactNode }) {
  const [state, dispatch] = useReducer(reducer, initialState)
  const mountedRef = useRef(true)

  const load = useCallback(async () => {
    dispatch({ type: ACTION.SET_LOADING, loading: true })
    try {
      const data = await api.results.list()
      const results = data.items
      if (mountedRef.current) dispatch({ type: ACTION.SET_ALL, results })
    } catch (e) {
      if (mountedRef.current) dispatch({ type: ACTION.SET_ERROR, error: e instanceof Error ? e.message : 'Failed to load results' })
    }
  }, [])

  useEffect(() => {
    mountedRef.current = true
    void load()
    return () => { mountedRef.current = false }
  }, [load])

  return (
    <ResultContext.Provider value={{ ...state, reload: () => { void load() } }}>
      {children}
    </ResultContext.Provider>
  )
}

export { ResultContext, ResultProvider }
export type { ResultContextValue }
