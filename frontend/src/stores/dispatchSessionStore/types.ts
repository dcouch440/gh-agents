import type { ChatMessageData } from '@/components/chat'

type DispatchStepSession = {
  sessionId: string | null
  messages: ChatMessageData[]
  isLoading: boolean
  error: string | null
}

type DispatchSessionState = {
  byStep: Record<string, DispatchStepSession>
}

export type { DispatchStepSession, DispatchSessionState }
