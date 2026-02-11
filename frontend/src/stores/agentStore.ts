// ============================================================================
// agentStore — Hand-written store for agents + sub-resources
// ============================================================================

import { createStore, createNormalizedMap, nmFromArray, nmSet, nmDelete, toArray, nmGet, logger, extractError } from './lib'
import type { NormalizedMap } from './lib'
import { api } from '@/api'
import type { Agent, AgentPoolStats, CreateAgentRequest, UpdateAgentRequest } from '@/types/agent'
import type { Tool } from '@/types/tool'
import type { DocumentListItem } from '@/types/document'

// ── State ────────────────────────────────────────────────────────────────────

type AgentState = {
  items: NormalizedMap<Agent>
  stats: AgentPoolStats | null
  toolsByAgent: Record<string, Tool[]>
  contextByAgent: Record<string, DocumentListItem[]>
  loading: boolean
  error: string | null
}

// ── Store ────────────────────────────────────────────────────────────────────

const store = logger(
  'agentStore',
  createStore<AgentState>(() => ({
    items: createNormalizedMap<Agent>(),
    stats: null,
    toolsByAgent: {},
    contextByAgent: {},
    loading: false,
    error: null,
  })),
)

// ── Selectors ────────────────────────────────────────────────────────────────

const selectAll = (s: AgentState): Agent[] => toArray(s.items)

const selectById =
  (id: string) =>
  (s: AgentState): Agent | undefined =>
    nmGet(s.items, id)

const selectStats = (s: AgentState): AgentPoolStats | null => s.stats

const selectTools =
  (agentId: string) =>
  (s: AgentState): Tool[] =>
    s.toolsByAgent[agentId] ?? []

const selectContext =
  (agentId: string) =>
  (s: AgentState): DocumentListItem[] =>
    s.contextByAgent[agentId] ?? []

const selectLoading = (s: AgentState): boolean => s.loading

const selectError = (s: AgentState): string | null => s.error

const selectToolsByAgent = (s: AgentState): Record<string, Tool[]> => s.toolsByAgent

// ── Async Actions ────────────────────────────────────────────────────────────

const fetchAll = async (): Promise<void> => {
  store.setState({ loading: true, error: null })
  try {
    const data = await api.agents.list()
    store.setState({ items: nmFromArray(data.agents), stats: data.stats, loading: false })
  } catch (e) {
    store.setState({ loading: false, error: extractError('agents', e) })
  }
}

const fetchOne = async (id: string): Promise<Agent> => {
  const agent = await api.agents.get(id)
  store.setState((s) => ({ items: nmSet(s.items, agent.id, agent) }))
  return agent
}

const create = async (body: CreateAgentRequest): Promise<Agent> => {
  const agent = await api.agents.create(body)
  store.setState((s) => ({ items: nmSet(s.items, agent.id, agent) }))
  return agent
}

const update = async (id: string, body: UpdateAgentRequest): Promise<Agent> => {
  const agent = await api.agents.update(id, body)
  store.setState((s) => ({ items: nmSet(s.items, agent.id, agent) }))
  return agent
}

const remove = async (id: string): Promise<void> => {
  const prev = store.getState()
  store.setState((s) => ({ items: nmDelete(s.items, id) }))
  try {
    await api.agents.delete(id)
  } catch (e) {
    store.setState({ items: prev.items, error: extractError('agents', e) })
    throw e
  }
}

// ── Sub-resource: Tools ──────────────────────────────────────────────────────

const fetchTools = async (agentId: string): Promise<Tool[]> => {
  const res = await api.agents.getTools(agentId)
  store.setState((s) => ({
    toolsByAgent: { ...s.toolsByAgent, [agentId]: res.tools },
  }))
  return res.tools
}

const setTools = async (agentId: string, toolIds: string[]): Promise<void> => {
  await api.agents.setTools(agentId, toolIds)
  await fetchTools(agentId)
}

// ── Sub-resource: Context (Documents) ────────────────────────────────────────

const fetchContext = async (agentId: string): Promise<DocumentListItem[]> => {
  const res = await api.agents.getContext(agentId)
  store.setState((s) => ({
    contextByAgent: { ...s.contextByAgent, [agentId]: res.documents },
  }))
  return res.documents
}

const setContext = async (agentId: string, docIds: string[]): Promise<void> => {
  await api.agents.setContext(agentId, docIds)
  await fetchContext(agentId)
}

// ── Export ────────────────────────────────────────────────────────────────────

export const agentStore = {
  store,
  selectAll,
  selectById,
  selectStats,
  selectTools,
  selectContext,
  selectLoading,
  selectError,
  selectToolsByAgent,
  fetchAll,
  fetchOne,
  create,
  update,
  remove,
  fetchTools,
  setTools,
  fetchContext,
  setContext,
}

export type { AgentState }
