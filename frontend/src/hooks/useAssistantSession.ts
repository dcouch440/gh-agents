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

type UseAssistantSessionReturn = {
  messages: ChatMessageData[]
  streamingSegments: MessageSegment[]
  isLoading: boolean
  error: string | null
  streaming: boolean
  activePanel: PanelState | null
  sendMessage: (content: string) => void
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

  const { send, abort, streaming } = useSendSessionMessage()
  const receivedLengthRef = useRef(0)
  const retriedRef = useRef(false)
  const retryAbortRef = useRef<(() => void) | null>(null)

  useEffect(() => {
    if (!workflowId) {
      assistantSessionStore.initEmpty(stepId)
      return
    }

    void assistantSessionStore.loadSession(workflowId, stepId)

    return () => {
      abort()
      retryAbortRef.current?.()
      retryAbortRef.current = null
      assistantSessionStore.resetStep(stepId)
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
    clearHistory,
    dismissPanel,
    submitPanelSelections,
  }
}

export { useAssistantSession }
export type { UseAssistantSessionReturn, PanelState }
