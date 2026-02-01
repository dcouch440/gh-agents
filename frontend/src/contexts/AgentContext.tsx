import { createContext, useReducer, useEffect, useCallback, useRef, type ReactNode } from 'react'
import { useWebSocket } from '@/hooks/useWebSocket'
import { ACTION, WS_CHANNEL, USE_MOCK_DATA } from '@/constants'
import { api } from '@/api'
import { mock } from '@/mock'
import type { Agent } from '@/types/agent'

// ── State ────────────────────────────────────────────────────────────────────

type AgentState = {
  agents: Agent[]
  loading: boolean
  error: string | null
}

const initialState: AgentState = { agents: [], loading: true, error: null }

// ── Actions ──────────────────────────────────────────────────────────────────

type AgentAction =
  | { type: typeof ACTION.SET_ALL; agents: Agent[] }
  | { type: typeof ACTION.UPDATE_ONE; agent: Agent }
  | { type: typeof ACTION.SET_LOADING; loading: boolean }
  | { type: typeof ACTION.SET_ERROR; error: string }

const reducer = (state: AgentState, action: AgentAction): AgentState => {
  switch (action.type) {
    case ACTION.SET_ALL:
      return { agents: action.agents, loading: false, error: null }
    case ACTION.UPDATE_ONE:
      return {
        ...state,
        agents: state.agents.some((a) => a.id === action.agent.id)
          ? state.agents.map((a) => (a.id === action.agent.id ? action.agent : a))
          : [...state.agents, action.agent],
      }
    case ACTION.SET_LOADING:
      return { ...state, loading: action.loading }
    case ACTION.SET_ERROR:
      return { ...state, loading: false, error: action.error }
  }
}

// ── Context ──────────────────────────────────────────────────────────────────

type AgentContextValue = AgentState & { reload: () => void }

const AgentContext = createContext<AgentContextValue | null>(null)

// ── Provider ─────────────────────────────────────────────────────────────────

type AgentsResponse = {
  stats: { orchestrators: number; workers: number; utilities: number }
  agents: Agent[]
}

function AgentProvider({ children }: { children: ReactNode }) {
  const [state, dispatch] = useReducer(reducer, initialState)
  const { subscribe } = useWebSocket()
  const mountedRef = useRef(true)

  const load = useCallback(async () => {
    dispatch({ type: ACTION.SET_LOADING, loading: true })
    try {
      const agents = USE_MOCK_DATA
        ? await mock.getAgents()
        : (await api.get<AgentsResponse>('/agents')).agents
      if (mountedRef.current) dispatch({ type: ACTION.SET_ALL, agents })
    } catch (e) {
      if (mountedRef.current) dispatch({ type: ACTION.SET_ERROR, error: e instanceof Error ? e.message : 'Failed to load agents' })
    }
  }, [])

  // Initial fetch
  useEffect(() => {
    mountedRef.current = true
    load()
    return () => { mountedRef.current = false }
  }, [load])

  // WS subscription
  useEffect(() => {
    const unsub = subscribe(WS_CHANNEL.AGENTS, (data) => {
      const msg = data as { agent?: Agent }
      if (msg.agent) {
        dispatch({ type: ACTION.UPDATE_ONE, agent: msg.agent })
      }
    })
    return unsub
  }, [subscribe])

  return (
    <AgentContext.Provider value={{ ...state, reload: load }}>
      {children}
    </AgentContext.Provider>
  )
}

export { AgentContext, AgentProvider }
export type { AgentContextValue }
