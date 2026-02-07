import { createContext, useReducer, useEffect, useCallback, useRef, type ReactNode } from 'react'
import { ACTION } from '@/constants'
import { api } from '@/api'
import type { Task } from '@/types/task'

// ── State ────────────────────────────────────────────────────────────────────

type TaskState = {
  tasks: Task[]
  loading: boolean
  error: string | null
}

const initialState: TaskState = { tasks: [], loading: true, error: null }

// ── Actions ──────────────────────────────────────────────────────────────────

type TaskUpdate = {
  id: string
  status: string
  progress: number | null
  assigned_agent: string | null
  user_id?: string | null
}

type TaskAction =
  | { type: typeof ACTION.SET_ALL; tasks: Task[] }
  | { type: typeof ACTION.UPDATE_ONE; task: Task }
  | { type: typeof ACTION.UPDATE; update: TaskUpdate }
  | { type: typeof ACTION.REMOVE_ONE; id: string }
  | { type: typeof ACTION.SET_LOADING; loading: boolean }
  | { type: typeof ACTION.SET_ERROR; error: string }

const reducer = (state: TaskState, action: TaskAction): TaskState => {
  switch (action.type) {
    case ACTION.SET_ALL:
      return { tasks: action.tasks, loading: false, error: null }
    case ACTION.UPDATE_ONE:
      return {
        ...state,
        tasks: state.tasks.some((t) => t.id === action.task.id)
          ? state.tasks.map((t) => (t.id === action.task.id ? action.task : t))
          : [...state.tasks, action.task],
      }
    case ACTION.UPDATE:
      return {
        ...state,
        tasks: state.tasks.map((t) =>
          t.id === action.update.id
            ? { ...t, status: action.update.status as Task['status'], assigned_agent: action.update.assigned_agent }
            : t,
        ),
      }
    case ACTION.REMOVE_ONE:
      return { ...state, tasks: state.tasks.filter((t) => t.id !== action.id) }
    case ACTION.SET_LOADING:
      return { ...state, loading: action.loading }
    case ACTION.SET_ERROR:
      return { ...state, loading: false, error: action.error }
  }
}

// ── Context ──────────────────────────────────────────────────────────────────

type TaskContextValue = TaskState & { reload: () => void }

const TaskContext = createContext<TaskContextValue | null>(null)

// ── Provider ─────────────────────────────────────────────────────────────────

function TaskProvider({ children }: { children: ReactNode }) {
  const [state, dispatch] = useReducer(reducer, initialState)
  const mountedRef = useRef(true)

  const load = useCallback(async () => {
    dispatch({ type: ACTION.SET_LOADING, loading: true })
    try {
      const data = await api.tasks.list()
      const tasks = data.items
      if (mountedRef.current) dispatch({ type: ACTION.SET_ALL, tasks })
    } catch (e) {
      if (mountedRef.current) dispatch({ type: ACTION.SET_ERROR, error: e instanceof Error ? e.message : 'Failed to load tasks' })
    }
  }, [])

  useEffect(() => {
    mountedRef.current = true
    void load()
    return () => { mountedRef.current = false }
  }, [load])

  return (
    <TaskContext.Provider value={{ ...state, reload: () => { void load() } }}>
      {children}
    </TaskContext.Provider>
  )
}

export { TaskContext, TaskProvider }
export type { TaskContextValue }
