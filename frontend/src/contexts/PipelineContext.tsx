import { createContext, useReducer, useEffect, useCallback, useRef, type ReactNode } from 'react'
import { useWebSocket } from '@/hooks/useWebSocket'
import { ACTION, WS_CHANNEL } from '@/constants'
import { api } from '@/api'
import type { Pipeline, PipelineRun } from '@/types/pipeline'

// ── State ────────────────────────────────────────────────────────────────────

type PipelineState = {
  pipelines: Pipeline[]
  runs: PipelineRun[]
  loading: boolean
  error: string | null
}

const initialState: PipelineState = { pipelines: [], runs: [], loading: true, error: null }

// ── Actions ──────────────────────────────────────────────────────────────────

type PipelineAction =
  | { type: typeof ACTION.SET_PIPELINES; pipelines: Pipeline[] }
  | { type: typeof ACTION.SET_RUNS; runs: PipelineRun[] }
  | { type: typeof ACTION.UPDATE_RUN; run: PipelineRun }
  | { type: typeof ACTION.SET_LOADING; loading: boolean }
  | { type: typeof ACTION.SET_ERROR; error: string }

const reducer = (state: PipelineState, action: PipelineAction): PipelineState => {
  switch (action.type) {
    case ACTION.SET_PIPELINES:
      return { ...state, pipelines: action.pipelines, loading: false, error: null }
    case ACTION.SET_RUNS:
      return { ...state, runs: action.runs }
    case ACTION.UPDATE_RUN:
      return {
        ...state,
        runs: state.runs.some((r) => r.id === action.run.id)
          ? state.runs.map((r) => (r.id === action.run.id ? action.run : r))
          : [...state.runs, action.run],
      }
    case ACTION.SET_LOADING:
      return { ...state, loading: action.loading }
    case ACTION.SET_ERROR:
      return { ...state, loading: false, error: action.error }
  }
}

// ── Context ──────────────────────────────────────────────────────────────────

type PipelineContextValue = PipelineState & { reload: () => void }

const PipelineContext = createContext<PipelineContextValue | null>(null)

// ── Provider ─────────────────────────────────────────────────────────────────

function PipelineProvider({ children }: { children: ReactNode }) {
  const [state, dispatch] = useReducer(reducer, initialState)
  const { subscribe } = useWebSocket()
  const mountedRef = useRef(true)

  const load = useCallback(async () => {
    dispatch({ type: ACTION.SET_LOADING, loading: true })
    try {
      const data = await api.pipelines.list()
      const pipelines = data.items
      if (mountedRef.current) dispatch({ type: ACTION.SET_PIPELINES, pipelines })
    } catch (e) {
      if (mountedRef.current) dispatch({ type: ACTION.SET_ERROR, error: e instanceof Error ? e.message : 'Failed to load pipelines' })
    }
  }, [])

  const loadRuns = useCallback(async () => {
    try {
      const data = await api.pipelineRuns.list()
      const runs = data.items
      if (mountedRef.current) dispatch({ type: ACTION.SET_RUNS, runs })
    } catch {
      // Runs reload is best-effort
    }
  }, [])

  useEffect(() => {
    mountedRef.current = true
    void load()
    void loadRuns()
    return () => { mountedRef.current = false }
  }, [load, loadRuns])

  // Pipeline WS events are status notifications (not full PipelineRun objects).
  // Reload runs list when we receive any pipeline event.
  useEffect(() => {
    const unsub = subscribe(WS_CHANNEL.PIPELINES, () => {
      void loadRuns()
    })
    return unsub
  }, [subscribe, loadRuns])

  return (
    <PipelineContext.Provider value={{ ...state, reload: () => { void load(); void loadRuns() } }}>
      {children}
    </PipelineContext.Provider>
  )
}

export { PipelineContext, PipelineProvider }
export type { PipelineContextValue }
