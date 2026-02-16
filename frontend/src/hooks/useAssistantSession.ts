import { useEffect, useCallback, useRef } from 'react'
import { useStore } from '@/stores'
import { assistantSessionStore } from '@/stores/assistantSessionStore'
import { api, createSSEStream } from '@/api'
import { API } from '@/constants'
import { useSendSessionMessage } from './useChatMutations'
import type { ChatMessageData } from '@/components/chat'
import type { MessageSegment } from '@/types'
import type { PanelState } from '@/stores/assistantSessionStore'

const STREAM_LOST_ERROR = 'Stream connection lost'
const SEND_FAILED_ERROR = 'Failed to send message'

// Track how many hook instances are mounted per stepId so we only
// reset store state when the *last* consumer unmounts (e.g. full-screen
// modal closing while the canvas ChatTab stays mounted).
const stepMountCounts = new Map<string, number>()

type UseAssistantSessionReturn = {
  messages: ChatMessageData[]
  streamingSegments: MessageSegment[]
  isLoading: boolean
  error: string | null
  streaming: boolean
  activePanel: PanelState | null
  sendMessage: (content: string) => void
  cancelGeneration: () => void
  clearHistory: () => void
  dismissPanel: () => void
  submitPanelSelections: (selections: string) => void
}

const useAssistantSession = (
  workflowId: string | null,
  stepId: string,
): UseAssistantSessionReturn => {
  const messages = useStore(assistantSessionStore.store, assistantSessionStore.selectMessages(stepId))
  const streamingSegments = useStore(assistantSessionStore.store, assistantSessionStore.selectSegments(stepId))
  const isLoading = useStore(assistantSessionStore.store, assistantSessionStore.selectLoading(stepId))
  const error = useStore(assistantSessionStore.store, assistantSessionStore.selectError(stepId))
  const activePanel = useStore(assistantSessionStore.store, assistantSessionStore.selectPanel(stepId))

  const { send, abort, cancelChat, streaming } = useSendSessionMessage()
  const receivedLengthRef = useRef(0)
  const retriedRef = useRef(false)
  const retryAbortRef = useRef<(() => void) | null>(null)

  useEffect(() => {
    const prev = stepMountCounts.get(stepId) ?? 0
    stepMountCounts.set(stepId, prev + 1)

    if (!workflowId) {
      if (prev === 0) assistantSessionStore.initEmpty(stepId)
      return () => {
        const next = (stepMountCounts.get(stepId) ?? 1) - 1
        stepMountCounts.set(stepId, next)
        if (next <= 0) {
          stepMountCounts.delete(stepId)
          assistantSessionStore.resetStep(stepId)
        }
      }
    }

    // Only fetch when the first consumer mounts for this stepId
    if (prev === 0) {
      void assistantSessionStore.loadSession(workflowId, stepId)
    }

    return () => {
      abort()
      retryAbortRef.current?.()
      retryAbortRef.current = null

      const next = (stepMountCounts.get(stepId) ?? 1) - 1
      stepMountCounts.set(stepId, next)
      if (next <= 0) {
        stepMountCounts.delete(stepId)
        // Defer reset to allow re-mount in the same render cycle
        // (e.g. effect re-fire due to dependency change)
        queueMicrotask(() => {
          if ((stepMountCounts.get(stepId) ?? 0) === 0) {
            assistantSessionStore.resetStep(stepId)
          }
        })
      }
    }
  }, [workflowId, stepId, abort])

  const sendMessage = useCallback(
    (content: string) => {
      if (!workflowId) return

      receivedLengthRef.current = 0
      retriedRef.current = false
      retryAbortRef.current?.()
      retryAbortRef.current = null

      assistantSessionStore.appendMessage(stepId, { id: crypto.randomUUID(), role: 'user', content })
      assistantSessionStore.appendMessage(stepId, { id: crypto.randomUUID(), role: 'assistant', content: '' })

      const onEvent = (event: Parameters<typeof assistantSessionStore.handleSSEEvent>[1]) => {
        receivedLengthRef.current += assistantSessionStore.handleSSEEvent(stepId, event)
      }

      const onDone = () => {
        assistantSessionStore.finalizeStream(stepId)
      }

      const doSend = async () => {
        try {
          let session = assistantSessionStore.store.getState().byStep[stepId]?.session ?? null
          if (!session) {
            session = await api.workflows.getOrCreateStepSession(workflowId, stepId)
            assistantSessionStore.setSessionCreated(stepId, session)
          }

          const messageId = await send(
            session.id,
            { message: content },
            onEvent,
            onDone,
            (err: Error) => {
              if (!retriedRef.current) {
                retriedRef.current = true
                const dedupeAfter = receivedLengthRef.current
                const handler = dedupeAfter > 0
                  ? assistantSessionStore.buildDeduplicatingHandler(
                      stepId,
                      dedupeAfter,
                      onEvent,
                      (len) => { receivedLengthRef.current += len },
                    )
                  : onEvent

                retryAbortRef.current = createSSEStream(
                  API.SESSION_CHAT_STREAM(session.id, messageId),
                  {
                    onEvent: handler,
                    onDone,
                    onError: () => { assistantSessionStore.handleStreamError(stepId, STREAM_LOST_ERROR) },
                  },
                )
              } else {
                assistantSessionStore.handleStreamError(stepId, err.message)
              }
            },
          )
          void messageId
        } catch (e) {
          assistantSessionStore.handleStreamError(stepId, e instanceof Error ? e.message : SEND_FAILED_ERROR)
        }
      }

      void doSend()
    },
    [workflowId, stepId, send],
  )

  const cancelGeneration = useCallback(() => {
    cancelChat()
    retryAbortRef.current?.()
    retryAbortRef.current = null
    assistantSessionStore.finalizeStream(stepId)
  }, [cancelChat, stepId])

  const dismissPanel = useCallback(() => {
    assistantSessionStore.dismissPanel(stepId)
  }, [stepId])

  const submitPanelSelections = useCallback(
    (selections: string) => {
      assistantSessionStore.dismissPanel(stepId)
      sendMessage(selections)
    },
    [stepId, sendMessage],
  )

  const clearHistory = useCallback(() => {
    if (!workflowId) return
    void assistantSessionStore.clearMessages(workflowId, stepId)
  }, [workflowId, stepId])

  return {
    messages,
    streamingSegments,
    isLoading,
    error,
    streaming,
    activePanel,
    sendMessage,
    cancelGeneration,
    clearHistory,
    dismissPanel,
    submitPanelSelections,
  }
}

export { useAssistantSession }
export type { UseAssistantSessionReturn, PanelState }
