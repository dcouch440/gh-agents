import type { ChatMessageData } from '@/components/chat'
import { memoFactory } from '../lib'
import type { DispatchSessionState } from './types'

const EMPTY_MESSAGES: ChatMessageData[] = []

const selectMessages = memoFactory(
  (stepId: string) =>
  (s: DispatchSessionState): ChatMessageData[] =>
    s.byStep[stepId]?.messages ?? EMPTY_MESSAGES,
)

const selectLoading = memoFactory(
  (stepId: string) =>
  (s: DispatchSessionState): boolean =>
    s.byStep[stepId]?.isLoading ?? false,
)

const selectError = memoFactory(
  (stepId: string) =>
  (s: DispatchSessionState): string | null =>
    s.byStep[stepId]?.error ?? null,
)

export { selectMessages, selectLoading, selectError }
