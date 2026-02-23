import type { ChatMessageData } from '@/components/chat'
import type { DispatchSessionState } from './types'

const EMPTY_MESSAGES: ChatMessageData[] = []

const selectMessages =
  (stepId: string) =>
  (s: DispatchSessionState): ChatMessageData[] =>
    s.byStep[stepId]?.messages ?? EMPTY_MESSAGES

const selectLoading =
  (stepId: string) =>
  (s: DispatchSessionState): boolean =>
    s.byStep[stepId]?.isLoading ?? false

const selectError =
  (stepId: string) =>
  (s: DispatchSessionState): string | null =>
    s.byStep[stepId]?.error ?? null

export { selectMessages, selectLoading, selectError }
