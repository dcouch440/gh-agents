import { createContext, useReducer, useEffect, useCallback, useRef, type ReactNode } from 'react'
import { ACTION, USE_MOCK_DATA, API } from '@/constants'
import { api } from '@/api'
import type { OutputSchema } from '@/types/schema'

// ── State ────────────────────────────────────────────────────────────────────

type OutputSchemaState = {
  schemas: OutputSchema[]
  loading: boolean
  error: string | null
}

const initialState: OutputSchemaState = { schemas: [], loading: true, error: null }

// ── Actions ──────────────────────────────────────────────────────────────────

type OutputSchemaAction =
  | { type: typeof ACTION.SET_ALL; schemas: OutputSchema[] }
  | { type: typeof ACTION.UPDATE_ONE; schema: OutputSchema }
  | { type: typeof ACTION.REMOVE_ONE; id: string }
  | { type: typeof ACTION.SET_LOADING; loading: boolean }
  | { type: typeof ACTION.SET_ERROR; error: string }

const reducer = (state: OutputSchemaState, action: OutputSchemaAction): OutputSchemaState => {
  switch (action.type) {
    case ACTION.SET_ALL:
      return { schemas: action.schemas, loading: false, error: null }
    case ACTION.UPDATE_ONE:
      return {
        ...state,
        schemas: state.schemas.some((s) => s.id === action.schema.id)
          ? state.schemas.map((s) => (s.id === action.schema.id ? action.schema : s))
          : [...state.schemas, action.schema],
      }
    case ACTION.REMOVE_ONE:
      return {
        ...state,
        schemas: state.schemas.filter((s) => s.id !== action.id),
      }
    case ACTION.SET_LOADING:
      return { ...state, loading: action.loading }
    case ACTION.SET_ERROR:
      return { ...state, loading: false, error: action.error }
  }
}

// ── Context ──────────────────────────────────────────────────────────────────

type OutputSchemaContextValue = OutputSchemaState & { reload: () => void }

const OutputSchemaContext = createContext<OutputSchemaContextValue | null>(null)

// ── Provider ─────────────────────────────────────────────────────────────────

function OutputSchemaProvider({ children }: { children: ReactNode }) {
  const [state, dispatch] = useReducer(reducer, initialState)
  const mountedRef = useRef(true)

  const load = useCallback(async () => {
    dispatch({ type: ACTION.SET_LOADING, loading: true })
    try {
      const schemas = USE_MOCK_DATA
        ? []
        : await api.get<OutputSchema[]>(API.OUTPUT_SCHEMAS)
      if (mountedRef.current) dispatch({ type: ACTION.SET_ALL, schemas })
    } catch (e) {
      if (mountedRef.current) dispatch({ type: ACTION.SET_ERROR, error: e instanceof Error ? e.message : 'Failed to load output schemas' })
    }
  }, [])

  useEffect(() => {
    mountedRef.current = true
    void load()
    return () => { mountedRef.current = false }
  }, [load])

  return (
    <OutputSchemaContext.Provider value={{ ...state, reload: () => { void load() } }}>
      {children}
    </OutputSchemaContext.Provider>
  )
}

export { OutputSchemaContext, OutputSchemaProvider }
export type { OutputSchemaContextValue }
