import { createContext, useReducer, useEffect, useCallback, useRef, type ReactNode } from 'react'
import { ACTION } from '@/constants'
import { api } from '@/api'
import type { CostResponse } from '@/types/cost'

// ── State ────────────────────────────────────────────────────────────────────

type CostState = {
  costs: CostResponse | null
  loading: boolean
  error: string | null
}

const initialState: CostState = { costs: null, loading: true, error: null }

// ── Actions ──────────────────────────────────────────────────────────────────

type CostAction =
  | { type: typeof ACTION.SET_ALL; costs: CostResponse }
  | { type: typeof ACTION.SET_LOADING; loading: boolean }
  | { type: typeof ACTION.SET_ERROR; error: string }

const reducer = (state: CostState, action: CostAction): CostState => {
  switch (action.type) {
    case ACTION.SET_ALL:
      return { costs: action.costs, loading: false, error: null }
    case ACTION.SET_LOADING:
      return { ...state, loading: action.loading }
    case ACTION.SET_ERROR:
      return { ...state, loading: false, error: action.error }
  }
}

// ── Context ──────────────────────────────────────────────────────────────────

type CostContextValue = CostState & { reload: () => void }

const CostContext = createContext<CostContextValue | null>(null)

// ── Provider ─────────────────────────────────────────────────────────────────

function CostProvider({ children }: { children: ReactNode }) {
  const [state, dispatch] = useReducer(reducer, initialState)
  const mountedRef = useRef(true)

  const load = useCallback(async () => {
    try {
      const costs = await api.costs.list()
      if (mountedRef.current) dispatch({ type: ACTION.SET_ALL, costs })
    } catch (e) {
      if (mountedRef.current) dispatch({ type: ACTION.SET_ERROR, error: e instanceof Error ? e.message : 'Failed to load costs' })
    }
  }, [])

  useEffect(() => {
    mountedRef.current = true
    void load()
    return () => { mountedRef.current = false }
  }, [load])

  return (
    <CostContext.Provider value={{ ...state, reload: () => { void load() } }}>
      {children}
    </CostContext.Provider>
  )
}

export { CostContext, CostProvider }
export type { CostContextValue }
