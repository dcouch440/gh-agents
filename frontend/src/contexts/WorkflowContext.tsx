import { createContext, useReducer, useEffect, useCallback, useRef, type ReactNode } from 'react'
import { ACTION } from '@/constants'
import { api } from '@/api'
import type { Workflow, WorkflowStep, WorkflowStepEdge, StepDocument } from '@/types/workflow'

// ── State ────────────────────────────────────────────────────────────────────

type WorkflowCurrent = {
  workflow: Workflow
  steps: WorkflowStep[]
  edges: WorkflowStepEdge[]
  stepDocuments: StepDocument[]
}

type WorkflowState = {
  workflows: Workflow[]
  current: WorkflowCurrent | null
  loading: boolean
  error: string | null
}

const initialState: WorkflowState = { workflows: [], current: null, loading: true, error: null }

// ── Actions ──────────────────────────────────────────────────────────────────

type WorkflowAction =
  | { type: typeof ACTION.SET_ALL; workflows: Workflow[] }
  | { type: typeof ACTION.SET_CURRENT; current: WorkflowCurrent }
  | { type: typeof ACTION.CLEAR_CURRENT }
  | { type: typeof ACTION.UPDATE_ONE; workflow: Workflow }
  | { type: typeof ACTION.REMOVE_ONE; id: string }
  | { type: typeof ACTION.SET_LOADING; loading: boolean }
  | { type: typeof ACTION.SET_ERROR; error: string }

const reducer = (state: WorkflowState, action: WorkflowAction): WorkflowState => {
  switch (action.type) {
    case ACTION.SET_ALL:
      return { ...state, workflows: action.workflows, loading: false, error: null }
    case ACTION.SET_CURRENT:
      return { ...state, current: action.current }
    case ACTION.CLEAR_CURRENT:
      return { ...state, current: null }
    case ACTION.UPDATE_ONE:
      return {
        ...state,
        workflows: state.workflows.some((w) => w.id === action.workflow.id)
          ? state.workflows.map((w) => (w.id === action.workflow.id ? action.workflow : w))
          : [...state.workflows, action.workflow],
      }
    case ACTION.REMOVE_ONE:
      return {
        ...state,
        workflows: state.workflows.filter((w) => w.id !== action.id),
        current: state.current?.workflow.id === action.id ? null : state.current,
      }
    case ACTION.SET_LOADING:
      return { ...state, loading: action.loading }
    case ACTION.SET_ERROR:
      return { ...state, loading: false, error: action.error }
  }
}

// ── Context ──────────────────────────────────────────────────────────────────

type WorkflowContextValue = WorkflowState & {
  reload: () => void
  loadWorkflow: (id: string) => Promise<void>
}

const WorkflowContext = createContext<WorkflowContextValue | null>(null)

// ── Provider ─────────────────────────────────────────────────────────────────

function WorkflowProvider({ children }: { children: ReactNode }) {
  const [state, dispatch] = useReducer(reducer, initialState)
  const mountedRef = useRef(true)

  const load = useCallback(async () => {
    dispatch({ type: ACTION.SET_LOADING, loading: true })
    try {
      const data = await api.workflows.list()
      const workflows = data.items
      if (mountedRef.current) dispatch({ type: ACTION.SET_ALL, workflows })
    } catch (e) {
      if (mountedRef.current) dispatch({ type: ACTION.SET_ERROR, error: e instanceof Error ? e.message : 'Failed to load workflows' })
    }
  }, [])

  const loadWorkflow = useCallback(async (id: string) => {
    try {
      const [workflow, steps, edges] = await Promise.all([
        api.workflows.get(id),
        api.workflows.listSteps(id),
        api.workflows.listEdges(id),
      ])

      // Collect step documents for all steps concurrently
      const docArrays = await Promise.all(
        steps.map((s) => api.workflows.listStepDocuments(id, s.id)),
      )
      const stepDocuments = docArrays.flat()

      if (mountedRef.current) {
        dispatch({ type: ACTION.SET_CURRENT, current: { workflow, steps, edges, stepDocuments } })
      }
    } catch (e) {
      if (mountedRef.current) dispatch({ type: ACTION.SET_ERROR, error: e instanceof Error ? e.message : 'Failed to load workflow' })
    }
  }, [])

  useEffect(() => {
    mountedRef.current = true
    void load()
    return () => { mountedRef.current = false }
  }, [load])

  return (
    <WorkflowContext.Provider value={{ ...state, reload: () => { void load() }, loadWorkflow }}>
      {children}
    </WorkflowContext.Provider>
  )
}

export { WorkflowContext, WorkflowProvider }
export type { WorkflowContextValue }
