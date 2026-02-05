type DraftConfig = {
  system_prompt: string
  model_id: string
  model_max_tokens: number
  model_temperature: number
  tool_names?: string[]
}

type Session = {
  id: string
  mode_id: string
  agent_id: string | null
  draft_config: DraftConfig | null
  title: string
  created_at: string
  updated_at: string
}

type ChatMessage = {
  id: string
  role: 'user' | 'assistant'
  content: string
  timestamp: string
}

type Mode = {
  id: string
  name: string
  description: string
}

type CreateSessionRequest = {
  mode_id: string
  agent_id?: string
  title?: string
  draft_config?: DraftConfig
}

type UpdateSessionRequest = {
  title?: string
}

type SendMessageRequest = {
  message: string
}

type SaveAgentRequest = {
  name: string
  context_document_ids?: string[]
}

type SaveAgentResponse = {
  agent_id: string
}

export type {
  DraftConfig,
  Session,
  ChatMessage,
  Mode,
  CreateSessionRequest,
  UpdateSessionRequest,
  SendMessageRequest,
  SaveAgentRequest,
  SaveAgentResponse,
}
