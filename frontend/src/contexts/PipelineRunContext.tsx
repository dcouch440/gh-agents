import { createContext, useReducer, useEffect, useCallback, useRef, type ReactNode } from 'react'
import { useWebSocket } from '@/hooks/useWebSocket'
import { ACTION, WS_EVENT, API } from '@/constants'
import { api } from '@/api'
import type { ExecutionTree, TreeAgentExecution } from '@/types/execution'

// ── State ────────────────────────────────────────────────────────────────────

type PipelineRunState = {
  tree: ExecutionTree | null
  loading: boolean
  error: string | null
}

const initialState: PipelineRunState = { tree: null, loading: true, error: null }

// ── Actions ──────────────────────────────────────────────────────────────────

type RunUpdate = {
  status: string
  current_stage: number
  completed_at: string | null
}

type StageExecutionUpdate = {
  id: string
  status: string
  completed_at: string | null
}

type AgentExecutionUpdate = {
  id: string
  status: string
  output: string | null
  structured_output: Record<string, unknown> | null
  input_tokens: number
  output_tokens: number
  completed_at: string | null
}

type ForEachNodes = {
  stage_execution_id: string
  executions: TreeAgentExecution[]
}

type PipelineRunAction =
  | { type: typeof ACTION.SET_TREE; tree: ExecutionTree }
  | { type: typeof ACTION.SET_LOADING; loading: boolean }
  | { type: typeof ACTION.SET_ERROR; error: string }
  | { type: typeof ACTION.UPDATE_RUN; update: RunUpdate }
  | { type: typeof ACTION.UPDATE_STAGE_EXECUTION; update: StageExecutionUpdate }
  | { type: typeof ACTION.UPDATE_AGENT_EXECUTION; update: AgentExecutionUpdate }
  | { type: typeof ACTION.ADD_FOR_EACH_NODES; payload: ForEachNodes }

const reducer = (state: PipelineRunState, action: PipelineRunAction): PipelineRunState => {
  switch (action.type) {
    case ACTION.SET_TREE:
      return { tree: action.tree, loading: false, error: null }

    case ACTION.SET_LOADING:
      return { ...state, loading: action.loading }

    case ACTION.SET_ERROR:
      return { ...state, loading: false, error: action.error }

    case ACTION.UPDATE_RUN: {
      if (!state.tree) return state
      return {
        ...state,
        tree: {
          ...state.tree,
          run: { ...state.tree.run, ...action.update },
        },
      }
    }

    case ACTION.UPDATE_STAGE_EXECUTION: {
      if (!state.tree) return state
      return {
        ...state,
        tree: {
          ...state.tree,
          stages: state.tree.stages.map((s) => ({
            ...s,
            stage_executions: s.stage_executions.map((se) =>
              se.id === action.update.id ? { ...se, ...action.update } : se,
            ),
          })),
        },
      }
    }

    case ACTION.UPDATE_AGENT_EXECUTION: {
      if (!state.tree) return state
      return {
        ...state,
        tree: {
          ...state.tree,
          stages: state.tree.stages.map((s) => ({
            ...s,
            stage_executions: s.stage_executions.map((se) => ({
              ...se,
              agent_executions: se.agent_executions.map((ae) =>
                ae.id === action.update.id ? { ...ae, ...action.update } : ae,
              ),
            })),
          })),
        },
      }
    }

    case ACTION.ADD_FOR_EACH_NODES: {
      if (!state.tree) return state
      return {
        ...state,
        tree: {
          ...state.tree,
          stages: state.tree.stages.map((s) => ({
            ...s,
            stage_executions: s.stage_executions.map((se) =>
              se.id === action.payload.stage_execution_id
                ? { ...se, agent_executions: [...se.agent_executions, ...action.payload.executions] }
                : se,
            ),
          })),
        },
      }
    }
  }
}

// ── Context ──────────────────────────────────────────────────────────────────

const PipelineRunContext = createContext<PipelineRunState | null>(null)

// ── Provider ─────────────────────────────────────────────────────────────────

function PipelineRunProvider({ runId, children }: { runId: string; children: ReactNode }) {
  const [state, dispatch] = useReducer(reducer, initialState)
  const { subscribeRun, unsubscribeRun } = useWebSocket()
  const mountedRef = useRef(true)

  const load = useCallback(async () => {
    dispatch({ type: ACTION.SET_LOADING, loading: true })
    try {
      const tree = await api.get<ExecutionTree>(API.PIPELINE_RUN_TREE(runId))
      if (mountedRef.current) dispatch({ type: ACTION.SET_TREE, tree })
    } catch (e) {
      if (mountedRef.current) dispatch({ type: ACTION.SET_ERROR, error: e instanceof Error ? e.message : 'Failed to load execution tree' })
    }
  }, [runId])

  // Initial fetch
  useEffect(() => {
    mountedRef.current = true
    void load()
    return () => { mountedRef.current = false }
  }, [load])

  // WS subscriptions
  useEffect(() => {
    const unsubs: (() => void)[] = []

    unsubs.push(
      subscribeRun(runId, WS_EVENT.PIPELINE_RUN_UPDATE, (data) => {
        const msg = data as { status?: string; current_stage?: number; completed_at?: string | null }
        dispatch({
          type: ACTION.UPDATE_RUN,
          update: {
            status: msg.status ?? '',
            current_stage: msg.current_stage ?? 0,
            completed_at: msg.completed_at ?? null,
          },
        })
      }),
    )

    unsubs.push(
      subscribeRun(runId, WS_EVENT.STAGE_EXECUTION_UPDATE, (data) => {
        const msg = data as { stage_execution_id: string; status: string; completed_at?: string | null }
        dispatch({
          type: ACTION.UPDATE_STAGE_EXECUTION,
          update: {
            id: msg.stage_execution_id,
            status: msg.status,
            completed_at: msg.completed_at ?? null,
          },
        })
      }),
    )

    unsubs.push(
      subscribeRun(runId, WS_EVENT.AGENT_EXECUTION_UPDATE, (data) => {
        const msg = data as {
          agent_execution_id: string
          status: string
          output?: string | null
          structured_output?: Record<string, unknown> | null
          input_tokens?: number
          output_tokens?: number
          completed_at?: string | null
        }
        dispatch({
          type: ACTION.UPDATE_AGENT_EXECUTION,
          update: {
            id: msg.agent_execution_id,
            status: msg.status,
            output: msg.output ?? null,
            structured_output: msg.structured_output ?? null,
            input_tokens: msg.input_tokens ?? 0,
            output_tokens: msg.output_tokens ?? 0,
            completed_at: msg.completed_at ?? null,
          },
        })
      }),
    )

    unsubs.push(
      subscribeRun(runId, WS_EVENT.FOR_EACH_SPAWNED, (data) => {
        const msg = data as { stage_execution_id: string; executions: TreeAgentExecution[] }
        dispatch({
          type: ACTION.ADD_FOR_EACH_NODES,
          payload: { stage_execution_id: msg.stage_execution_id, executions: msg.executions },
        })
      }),
    )

    return () => {
      for (const unsub of unsubs) unsub()
      unsubscribeRun(runId)
    }
  }, [runId, subscribeRun, unsubscribeRun])

  return (
    <PipelineRunContext.Provider value={state}>
      {children}
    </PipelineRunContext.Provider>
  )
}

export { PipelineRunContext, PipelineRunProvider }
export type { PipelineRunState }
