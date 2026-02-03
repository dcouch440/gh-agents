type Session = {
  id: string
  mode_id: string
  agent_id: string | null
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
}

type UpdateSessionRequest = {
  title?: string
}

type SendMessageRequest = {
  message: string
}

export type { Session, ChatMessage, Mode, CreateSessionRequest, UpdateSessionRequest, SendMessageRequest }
