type AgentTier = 'orchestrator' | 'worker' | 'utility'

type AgentStatus = 'idle' | 'working' | 'waiting_for_context' | 'waiting_for_approval'

type AgentPersona = {
  name: string
  system_prompt: string
  style: string
}

type ModelConfig = {
  provider: string
  model_id: string
  max_tokens: number
  temperature: number
}

type Agent = {
  id: string
  tier: AgentTier
  persona: AgentPersona
  model_config: ModelConfig
  status: AgentStatus
  current_task: string | null
  router_mode: boolean
}

type CreateAgentRequest = {
  tier: AgentTier
  persona: AgentPersona
  model_config: ModelConfig
  router_mode?: boolean
}

type UpdateAgentRequest = Partial<CreateAgentRequest>

export type { Agent, AgentTier, AgentStatus, AgentPersona, ModelConfig, CreateAgentRequest, UpdateAgentRequest }
