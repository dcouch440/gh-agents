import type { ChatMessageData } from '@/components/chat'
import type { Session, MessageSegment } from '@/types'
import { memoFactory } from '../lib'
import type { AssistantSessionState } from './types'
import { emptySession } from './_store'

const EMPTY_MESSAGES: ChatMessageData[] = []
const EMPTY_SEGMENTS: MessageSegment[] = []

const selectSession = memoFactory(
  (stepId: string) =>
  (s: AssistantSessionState): Session | null =>
    s.byStep[stepId]?.session ?? null,
)

const selectMessages = memoFactory(
  (stepId: string) =>
  (s: AssistantSessionState): ChatMessageData[] =>
    s.byStep[stepId]?.messages ?? EMPTY_MESSAGES,
)

const selectSegments = memoFactory(
  (stepId: string) =>
  (s: AssistantSessionState): MessageSegment[] =>
    s.byStep[stepId]?.streamingSegments ?? EMPTY_SEGMENTS,
)

const selectLoading = memoFactory(
  (stepId: string) =>
  (s: AssistantSessionState): boolean =>
    s.byStep[stepId]?.isLoading ?? emptySession.isLoading,
)

const selectError = memoFactory(
  (stepId: string) =>
  (s: AssistantSessionState): string | null =>
    s.byStep[stepId]?.error ?? null,
)

const selectStreaming = memoFactory(
  (stepId: string) =>
  (s: AssistantSessionState): boolean =>
    s.byStep[stepId]?.streaming ?? false,
)

export {
  selectSession,
  selectMessages,
  selectSegments,
  selectLoading,
  selectError,
  selectStreaming,
}
