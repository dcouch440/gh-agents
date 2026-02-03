type AgentTier = 'orchestrator' | 'worker' | 'utility'

type AgentStatus = 'idle' | 'working' | 'waiting_for_context' | 'waiting_for_approval'

type TierStats = {
  total: number
  available: number
  max: number
}

type AgentPoolStats = {
  orchestrators: TierStats
  workers: TierStats
  utilities: TierStats
}

type Agent = {
  id: string
  name: string
  system_prompt: string
  model_provider: string
  model_id: string
  model_max_tokens: number
  model_temperature: number
  created_at: string
  updated_at: string
  // Dashboard fields (from WS updates)
  status?: AgentStatus
  tier?: AgentTier
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
}

type UpdateAgentRequest = Partial<CreateAgentRequest>

// Avoid circular import — these are lightweight forward references
import type { Tool } from './tool'
import type { DocumentListItem } from './document'

export type {
  Agent,
  AgentTier,
  AgentStatus,
  TierStats,
  AgentPoolStats,
  AgentsResponse,
  AgentToolsResponse,
  AgentContextResponse,
  CreateAgentRequest,
  UpdateAgentRequest,
}
