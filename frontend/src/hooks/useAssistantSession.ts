import { useEffect, useCallback, useRef } from 'react'
import { useStore } from '@/stores'
import { assistantSessionStore } from '@/stores/assistantSessionStore'
import { api, createSSEStream } from '@/api'
import type { SSEEvent } from '@/api'
import { API } from '@/constants'
import { useSendSessionMessage } from './useChatMutations'
import type { ChatMessageData } from '@/components/chat'
import type { MessageSegment } from '@/types'
import type { PanelState } from '@/stores/assistantSessionStore'

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

  // On mount / stepId change: load existing session
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

      const userMsg: ChatMessageData = { id: crypto.randomUUID(), role: 'user', content }
      assistantSessionStore.appendMessage(stepId, userMsg)

      const assistantMsg: ChatMessageData = { id: crypto.randomUUID(), role: 'assistant', content: '' }
      assistantSessionStore.appendMessage(stepId, assistantMsg)

      const onEvent = (event: SSEEvent) => {
        switch (event.event) {
          case 'token':
          case 'message':
          case 'content': {
            const text = assistantSessionStore.parseTokenText(event.data)
            receivedLengthRef.current += text.length
            assistantSessionStore.streamToken(stepId, text)
            break
          }
          case 'tool_start': {
            const data = JSON.parse(event.data) as { name: string; id: string }
            assistantSessionStore.addTool(stepId, data.id, data.name)
            break
          }
          case 'tool_end': {
            const data = JSON.parse(event.data) as { name: string; id: string }
            assistantSessionStore.completeTool(stepId, data.id)
            break
          }
          case 'doc_update': {
            const data = JSON.parse(event.data) as { doc_id: string; title: string }
            assistantSessionStore.addDoc(stepId, data.doc_id, data.title)
            break
          }
          case 'panel_render': {
            const data = JSON.parse(event.data) as { content: string; submit_label: string }
            assistantSessionStore.setPanel(stepId, data.content, data.submit_label)
            break
          }
          case 'error': {
            assistantSessionStore.handleStreamError(stepId, event.data)
            break
          }
        }
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

                let replayedLength = 0
                const deduplicatingHandler = (evt: SSEEvent) => {
                  if (evt.event === 'token' || evt.event === 'message' || evt.event === 'content') {
                    const text = assistantSessionStore.parseTokenText(evt.data)
                    replayedLength += text.length
                    if (replayedLength <= dedupeAfter) return
                    const overlap = dedupeAfter - (replayedLength - text.length)
                    const newText = overlap > 0 ? text.slice(overlap) : text
                    if (newText) {
                      receivedLengthRef.current += newText.length
                      assistantSessionStore.streamToken(stepId, newText)
                    }
                  } else {
                    onEvent(evt)
                  }
                }

                retryAbortRef.current = createSSEStream(
                  API.SESSION_CHAT_STREAM(session.id, messageId),
                  {
                    onEvent: dedupeAfter > 0 ? deduplicatingHandler : onEvent,
                    onDone,
                    onError: () => {
                      assistantSessionStore.handleStreamError(stepId, 'Stream connection lost')
                    },
                  },
                )
              } else {
                assistantSessionStore.handleStreamError(stepId, err.message)
              }
            },
          )
          void messageId
        } catch (e) {
          assistantSessionStore.handleStreamError(
            stepId,
            e instanceof Error ? e.message : 'Failed to send message',
          )
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
