import { useState, useCallback, useEffect, useRef } from 'react'
import { api } from '@/api'
import type { SSEEvent } from '@/api'
import type { ChatMessageData } from '@/components/chat/ChatPanel'
import { useSendSessionMessage } from './useChatMutations'

/**
 * Hook for the workflow agent chat. Manages session creation, message history,
 * and SSE token streaming. Drop-in for ChatPanel.
 */
const useWorkflowAgentChat = (workflowId: string | null) => {
  const [sessionId, setSessionId] = useState<string | null>(null)
  const [messages, setMessages] = useState<ChatMessageData[]>([])
  const [streaming, setStreaming] = useState(false)
  const contentRef = useRef('')
  const pendingFrameRef = useRef<number | null>(null)
  const { send, cancelChat } = useSendSessionMessage()

  // Create/get session on mount
  useEffect(() => {
    if (!workflowId) return
    const init = async () => {
      try {
        const res = await api.workflows.getOrCreateAgentSession(workflowId)
        setSessionId(res.session_id)
        const history = await api.sessions.getHistory(res.session_id)
        setMessages(
          history.map((m) => ({
            id: m.id,
            role: m.role,
            content: m.content,
          })),
        )
      } catch (err) {
        console.error('[useWorkflowAgentChat] Failed to init session:', err)
      }
    }
    void init()
  }, [workflowId])

  const sendMessage = useCallback(
    (message: string) => {
      if (!sessionId) return

      // Add user message
      const userMsgId = `msg-${Date.now()}`
      setMessages((prev) => [...prev, { id: userMsgId, role: 'user', content: message }])

      // Add empty assistant placeholder
      const assistantMsgId = `msg-${Date.now() + 1}`
      setMessages((prev) => [...prev, { id: assistantMsgId, role: 'assistant', content: '' }])
      setStreaming(true)
      contentRef.current = ''

      const onEvent = (event: SSEEvent) => {
        if (event.event === 'token' || event.event === 'message' || event.event === 'content') {
          let text = event.data
          try {
            const parsed = JSON.parse(text) as unknown
            if (typeof parsed === 'string') text = parsed
          } catch {
            // use raw
          }
          contentRef.current += text
          pendingFrameRef.current ??= requestAnimationFrame(() => {
            pendingFrameRef.current = null
            const content = contentRef.current
            setMessages((prev) => {
              const msgs = [...prev]
              const lastIdx = msgs.length - 1
              if (lastIdx >= 0 && msgs[lastIdx].role === 'assistant') {
                msgs[lastIdx] = { ...msgs[lastIdx], content }
              }
              return msgs
            })
          })
        }
      }

      const onDone = () => {
        if (pendingFrameRef.current !== null) {
          cancelAnimationFrame(pendingFrameRef.current)
          pendingFrameRef.current = null
          const content = contentRef.current
          setMessages((prev) => {
            const msgs = [...prev]
            const lastIdx = msgs.length - 1
            if (lastIdx >= 0 && msgs[lastIdx].role === 'assistant') {
              msgs[lastIdx] = { ...msgs[lastIdx], content }
            }
            return msgs
          })
        }
        setStreaming(false)
      }

      void send(sessionId, { message }, onEvent, onDone)
    },
    [sessionId, send],
  )

  return { messages, sendMessage, streaming, cancelChat, sessionId }
}

export { useWorkflowAgentChat }
