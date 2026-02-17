type AgentStatus = 'idle' | 'working' | 'waiting_for_context' | 'waiting_for_approval'

type AgentPoolStats = {
  total: number
  available: number
  max: number
}

type Agent = {
  id: string
  name: string
  system_prompt: string
  model_provider: string
  model_id: string
  model_max_tokens: number
  model_temperature: number
  status: string
  output_schema_id: string | null
  version: number
}

type AgentsResponse = {
  agents: Agent[]
  stats: AgentPoolStats
}

type AgentToolsResponse = {
  agent_id: string
  tools: Tool[]
}

type AgentContextResponse = {
  agent_id: string
  documents: DocumentListItem[]
}

type CreateAgentRequest = {
  name: string
  system_prompt?: string
  model_provider?: string
  model_id?: string
  model_max_tokens?: number
  model_temperature?: number
  output_schema_id?: string
}

type UpdateAgentRequest = Partial<CreateAgentRequest>

// Avoid circular import — these are lightweight forward references
import type { Tool } from './tool'
import type { DocumentListItem } from './document'

export type {
  Agent,
  AgentStatus,
  AgentPoolStats,
  AgentsResponse,
  AgentToolsResponse,
  AgentContextResponse,
  CreateAgentRequest,
  UpdateAgentRequest,
}
