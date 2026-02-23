import type { ChatMessageData } from '@/components/chat'
import type { Session, MessageSegment } from '@/types'

type StepSession = {
  session: Session | null
  messages: ChatMessageData[]
  streamingSegments: MessageSegment[]
  isLoading: boolean
  error: string | null
  streaming: boolean
}

type AssistantSessionState = {
  byStep: Record<string, StepSession>
}

export type { StepSession, AssistantSessionState }
