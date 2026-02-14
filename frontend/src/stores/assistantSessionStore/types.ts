import type { ChatMessageData } from '@/components/chat'
import type { Session, MessageSegment } from '@/types'

type PanelState = {
  content: string
  submitLabel: string
}

type StepSession = {
  session: Session | null
  messages: ChatMessageData[]
  streamingSegments: MessageSegment[]
  isLoading: boolean
  error: string | null
  activePanel: PanelState | null
}

type AssistantSessionState = {
  byStep: Record<string, StepSession>
}

export type { PanelState, StepSession, AssistantSessionState }
