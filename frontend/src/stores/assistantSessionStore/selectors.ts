import type { ChatMessageData } from '@/components/chat'
import type { Session, MessageSegment } from '@/types'
import type { AssistantSessionState, PanelState } from './types'
import { emptySession } from './_store'

const EMPTY_MESSAGES: ChatMessageData[] = []
const EMPTY_SEGMENTS: MessageSegment[] = []

const selectSession =
  (stepId: string) =>
  (s: AssistantSessionState): Session | null =>
    s.byStep[stepId]?.session ?? null

const selectMessages =
  (stepId: string) =>
  (s: AssistantSessionState): ChatMessageData[] =>
    s.byStep[stepId]?.messages ?? EMPTY_MESSAGES

const selectSegments =
  (stepId: string) =>
  (s: AssistantSessionState): MessageSegment[] =>
    s.byStep[stepId]?.streamingSegments ?? EMPTY_SEGMENTS

const selectPanel =
  (stepId: string) =>
  (s: AssistantSessionState): PanelState | null =>
    s.byStep[stepId]?.activePanel ?? null

const selectLoading =
  (stepId: string) =>
  (s: AssistantSessionState): boolean =>
    s.byStep[stepId]?.isLoading ?? emptySession.isLoading

const selectError =
  (stepId: string) =>
  (s: AssistantSessionState): string | null =>
    s.byStep[stepId]?.error ?? null

const selectStreaming =
  (stepId: string) =>
  (s: AssistantSessionState): boolean =>
    s.byStep[stepId]?.streaming ?? false

export {
  selectSession,
  selectMessages,
  selectSegments,
  selectPanel,
  selectLoading,
  selectError,
  selectStreaming,
}
