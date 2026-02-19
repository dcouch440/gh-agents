import { useEffect, useCallback, useRef } from 'react'
import { useStore } from '@/stores'
import { assistantSessionStore } from '@/stores/assistantSessionStore'
import { api, createSSEStream } from '@/api'
import { API } from '@/constants'
import type { SSEEvent } from '@/api'
import type { ChatMessageData } from '@/components/chat'
import type { MessageSegment } from '@/types'
import type { PanelState } from '@/stores/assistantSessionStore'

const STREAM_LOST_ERROR = 'Stream connection lost'
const SEND_FAILED_ERROR = 'Failed to send message'

// ---------------------------------------------------------------------------
// Module-level stream lifecycle management
//
// Stream abort functions and retry state live here (not in component state)
// so streams survive tab switches within the same workflow. Streams are only
// aborted when the user explicitly cancels or navigates to a different workflow.
// ---------------------------------------------------------------------------

type ActiveStream = {
  abort: () => void
  retryAbort: (() => void) | null
  receivedLength: number
  retried: boolean
  sessionId: string
  messageId: string
}

const activeStreams = new Map<string, ActiveStream>()

const abortStep = (stepId: string): void => {
  const stream = activeStreams.get(stepId)
  if (!stream) return
  stream.abort()
  stream.retryAbort?.()
  activeStreams.delete(stepId)
}

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

type SendMessageResponse = {
  message_id: string
  status: string
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
  const streaming = useStore(assistantSessionStore.store, assistantSessionStore.selectStreaming(stepId))

  // Mount tracking and session loading
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
          abortStep(stepId)
          assistantSessionStore.resetStep(stepId)
        }
      }
    }

    // Only fetch when the first consumer mounts for this stepId
    if (prev === 0) {
      void assistantSessionStore.loadSession(workflowId, stepId)
    }

    return () => {
      // Don't abort the stream on unmount — it persists across tab switches.
      // Stream state (segments, messages) stays in the store and will be
      // picked up when the component remounts.
      const next = (stepMountCounts.get(stepId) ?? 1) - 1
      stepMountCounts.set(stepId, next)
      if (next <= 0) {
        stepMountCounts.delete(stepId)
        // Defer reset to allow re-mount in the same render cycle
        queueMicrotask(() => {
          if ((stepMountCounts.get(stepId) ?? 0) === 0) {
            // All consumers gone and nobody re-mounted — clean up.
            // Only abort + reset if we're truly leaving (no re-mount).
            abortStep(stepId)
            assistantSessionStore.resetStep(stepId)
          }
        })
      }
    }
  }, [workflowId, stepId])

  // Abort streams when the workflow changes (user navigated away)
  const prevWorkflowIdRef = useRef(workflowId)
  useEffect(() => {
    if (prevWorkflowIdRef.current && prevWorkflowIdRef.current !== workflowId) {
      abortStep(stepId)
    }
    prevWorkflowIdRef.current = workflowId
  }, [workflowId, stepId])

  const sendMessage = useCallback(
    (content: string) => {
      if (!workflowId) return

      // Abort any existing stream for this step
      abortStep(stepId)

      assistantSessionStore.appendMessage(stepId, { id: crypto.randomUUID(), role: 'user', content })
      assistantSessionStore.appendMessage(stepId, { id: crypto.randomUUID(), role: 'assistant', content: '' })
      assistantSessionStore.setStreaming(stepId, true)

      const onEvent = (event: SSEEvent) => {
        const stream = activeStreams.get(stepId)
        if (stream) {
          stream.receivedLength += assistantSessionStore.handleSSEEvent(stepId, event)
        }
      }

      const onDone = () => {
        activeStreams.delete(stepId)
        assistantSessionStore.finalizeStream(stepId)
      }

      const doSend = async () => {
        try {
          let session = assistantSessionStore.store.getState().byStep[stepId]?.session ?? null
          if (!session) {
            session = await api.workflows.getOrCreateStepSession(workflowId, stepId)
            assistantSessionStore.setSessionCreated(stepId, session)
          }

          const { message_id: messageId } = await api.post<SendMessageResponse>(
            API.SESSION_CHAT(session.id),
            { message: content },
          )

          const sseAbort = createSSEStream(
            API.SESSION_CHAT_STREAM(session.id, messageId),
            {
              onEvent,
              onDone,
              onError: (err) => {
                const stream = activeStreams.get(stepId)
                if (!stream) return

                if (!stream.retried) {
                  stream.retried = true
                  const dedupeAfter = stream.receivedLength
                  const handler = dedupeAfter > 0
                    ? assistantSessionStore.buildDeduplicatingHandler(
                        stepId,
                        dedupeAfter,
                        onEvent,
                        (len) => {
                          const s = activeStreams.get(stepId)
                          if (s) s.receivedLength += len
                        },
                      )
                    : onEvent

                  stream.retryAbort = createSSEStream(
                    API.SESSION_CHAT_STREAM(session.id, messageId),
                    {
                      onEvent: handler,
                      onDone,
                      onError: () => {
                        activeStreams.delete(stepId)
                        assistantSessionStore.handleStreamError(stepId, STREAM_LOST_ERROR)
                      },
                    },
                  )
                } else {
                  activeStreams.delete(stepId)
                  assistantSessionStore.handleStreamError(stepId, err.message)
                }
              },
            },
          )

          activeStreams.set(stepId, {
            abort: sseAbort,
            retryAbort: null,
            receivedLength: 0,
            retried: false,
            sessionId: session.id,
            messageId,
          })
        } catch (e) {
          activeStreams.delete(stepId)
          assistantSessionStore.handleStreamError(stepId, e instanceof Error ? e.message : SEND_FAILED_ERROR)
        }
      }

      void doSend()
    },
    [workflowId, stepId],
  )

  const cancelGeneration = useCallback(() => {
    const stream = activeStreams.get(stepId)
    if (stream) {
      // Cancel on the backend with proper session/message IDs
      void api.sessions.cancelChat(stream.sessionId, stream.messageId).catch(() => {})
    }
    abortStep(stepId)
    assistantSessionStore.finalizeStream(stepId)
  }, [stepId])

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
