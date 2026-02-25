import { store, reset } from './_store'
import { handleWsEvent } from './wsHandler'
import type { AgentTraceState, AgentTrace } from './types'

const selectTraces = (s: AgentTraceState): Record<string, AgentTrace> => s.traces

const selectOrder = (s: AgentTraceState): string[] => s.order

const selectTraceById = (id: string) => (s: AgentTraceState): AgentTrace | null => s.traces[id] ?? null

export const agentTraceStore = {
  store,
  handleWsEvent,
  reset,
  selectTraces,
  selectOrder,
  selectTraceById,
}

export type { AgentTraceState, AgentTrace, AgentTraceEvent } from './types'
