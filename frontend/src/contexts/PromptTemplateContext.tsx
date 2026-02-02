import { createContext, useReducer, useEffect, useCallback, useRef, type ReactNode } from 'react'
import { ACTION, USE_MOCK_DATA, API } from '@/constants'
import { api } from '@/api'
import type { PromptTemplate } from '@/types/template'

// ── State ────────────────────────────────────────────────────────────────────

type PromptTemplateState = {
  templates: PromptTemplate[]
  loading: boolean
  error: string | null
}

const initialState: PromptTemplateState = { templates: [], loading: true, error: null }

// ── Actions ──────────────────────────────────────────────────────────────────

type PromptTemplateAction =
  | { type: typeof ACTION.SET_ALL; templates: PromptTemplate[] }
  | { type: typeof ACTION.UPDATE_ONE; template: PromptTemplate }
  | { type: typeof ACTION.REMOVE_ONE; id: string }
  | { type: typeof ACTION.SET_LOADING; loading: boolean }
  | { type: typeof ACTION.SET_ERROR; error: string }

const reducer = (state: PromptTemplateState, action: PromptTemplateAction): PromptTemplateState => {
  switch (action.type) {
    case ACTION.SET_ALL:
      return { templates: action.templates, loading: false, error: null }
    case ACTION.UPDATE_ONE:
      return {
        ...state,
        templates: state.templates.some((t) => t.id === action.template.id)
          ? state.templates.map((t) => (t.id === action.template.id ? action.template : t))
          : [...state.templates, action.template],
      }
    case ACTION.REMOVE_ONE:
      return {
        ...state,
        templates: state.templates.filter((t) => t.id !== action.id),
      }
    case ACTION.SET_LOADING:
      return { ...state, loading: action.loading }
    case ACTION.SET_ERROR:
      return { ...state, loading: false, error: action.error }
  }
}

// ── Context ──────────────────────────────────────────────────────────────────

type PromptTemplateContextValue = PromptTemplateState & { reload: () => void }

const PromptTemplateContext = createContext<PromptTemplateContextValue | null>(null)

// ── Provider ─────────────────────────────────────────────────────────────────

function PromptTemplateProvider({ children }: { children: ReactNode }) {
  const [state, dispatch] = useReducer(reducer, initialState)
  const mountedRef = useRef(true)

  const load = useCallback(async () => {
    dispatch({ type: ACTION.SET_LOADING, loading: true })
    try {
      const templates = USE_MOCK_DATA
        ? []
        : await api.get<PromptTemplate[]>(API.PROMPT_TEMPLATES)
      if (mountedRef.current) dispatch({ type: ACTION.SET_ALL, templates })
    } catch (e) {
      if (mountedRef.current) dispatch({ type: ACTION.SET_ERROR, error: e instanceof Error ? e.message : 'Failed to load prompt templates' })
    }
  }, [])

  // Initial fetch
  useEffect(() => {
    mountedRef.current = true
    void load()
    return () => { mountedRef.current = false }
  }, [load])

  return (
    <PromptTemplateContext.Provider value={{ ...state, reload: () => { void load() } }}>
      {children}
    </PromptTemplateContext.Provider>
  )
}

export { PromptTemplateContext, PromptTemplateProvider }
export type { PromptTemplateContextValue }
