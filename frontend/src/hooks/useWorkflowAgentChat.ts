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
          history.map((m) => {
            if (m.source_type === 'tool') {
              const sepIdx = m.content.indexOf('\n---\n')
              const header = sepIdx >= 0 ? m.content.slice(0, sepIdx) : m.content
              const result = sepIdx >= 0 ? m.content.slice(sepIdx + 5) : ''
              const colonIdx = header.indexOf(': ')
              const toolName = colonIdx >= 0 ? header.slice(0, colonIdx) : 'tool'
              const toolInput = colonIdx >= 0 ? header.slice(colonIdx + 2) : header
              return { id: m.id, role: 'tool' as const, content: toolInput, toolName, toolResult: result }
            }
            return { id: m.id, role: m.role, content: m.content }
          }),
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
        if (event.event === 'tool_start') {
          try {
            const data = JSON.parse(event.data) as { name: string; id: string; input: string }
            // Parse the input to extract the command
            let inputText = data.input
            try {
              const parsed: unknown = JSON.parse(data.input)
              if (parsed !== null && typeof parsed === 'object' && 'command' in parsed) {
                inputText = String((parsed as Record<string, unknown>).command)
              }
            } catch { /* use raw */ }
            setMessages((prev) => [...prev, {
              id: `tool-${data.id}`,
              role: 'tool' as const,
              content: inputText,
              toolName: data.name,
            }])
          } catch { /* ignore parse errors */ }
          return
        }
        if (event.event === 'tool_end') {
          try {
            const data = JSON.parse(event.data) as { name: string; id: string; result: string }
            setMessages((prev) => {
              const msgs = [...prev]
              const toolIdx = msgs.findIndex((m) => m.id === `tool-${data.id}`)
              if (toolIdx >= 0) {
                msgs[toolIdx] = { ...msgs[toolIdx], toolResult: data.result }
              }
              return msgs
            })
          } catch { /* ignore parse errors */ }
          return
        }
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
