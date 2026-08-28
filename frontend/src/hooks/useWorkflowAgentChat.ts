import { useState, useCallback, useEffect, useRef } from 'react'
import { api } from '@/api'
import type { SSEEvent } from '@/api'
import type { ChatMessageData } from '@/components/chat/ChatPanel'
import { useSendSessionMessage } from './useChatMutations'

/**
 * Write streamed content into the assistant message for the current round.
 *
 * Tool messages are inserted between LLM rounds, so the assistant message
 * being written to is always the last one after the last tool message.
 */
const applyAssistantContent = (msgs: ChatMessageData[], content: string): ChatMessageData[] => {
  const next = [...msgs]
  let lastToolIdx = -1
  for (let i = next.length - 1; i >= 0; i--) {
    if (next[i].role === 'tool') { lastToolIdx = i; break }
  }
  let assistantIdx = -1
  for (let i = next.length - 1; i > lastToolIdx; i--) {
    if (next[i].role === 'assistant') { assistantIdx = i; break }
  }
  if (assistantIdx >= 0) {
    next[assistantIdx] = { ...next[assistantIdx], content }
  } else {
    next.push({ id: `msg-${Date.now()}`, role: 'assistant', content })
  }
  return next
}

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
  // Ends the in-flight turn. Held in a ref so cancelling can settle the turn
  // the same way a normal completion does — aborting the SSE stream never
  // fires onDone, so without this the UI would stay "streaming" forever.
  const settleRef = useRef<((errorMessage?: string) => void) | null>(null)
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

              // Reconstruct render_panel as an interactive panel (already submitted)
              if (toolName === 'render_panel') {
                try {
                  const parsed = JSON.parse(toolInput) as { content?: string; submit_label?: string }
                  return {
                    id: m.id,
                    role: 'assistant' as const,
                    content: parsed.content ?? '',
                    source_type: 'panel_render',
                    panelMeta: { submitLabel: parsed.submit_label ?? 'Submit', submitted: true },
                  }
                } catch { /* fall through to normal tool display */ }
              }

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
            if (data.name === 'render_panel') return
            let inputText = data.input
            try {
              const parsed: unknown = JSON.parse(data.input)
              if (parsed !== null && typeof parsed === 'object' && 'command' in parsed) {
                inputText = String((parsed as Record<string, unknown>).command)
              }
            } catch { /* use raw */ }
            // Reset content accumulator — next tokens will be a new assistant response
            contentRef.current = ''
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
            if (data.name === 'render_panel') return
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
        if (event.event === 'panel_render') {
          try {
            const data = JSON.parse(event.data) as { content: string; submit_label: string }
            const panelMsg: ChatMessageData = {
              id: `panel-${Date.now()}`,
              role: 'assistant',
              content: data.content,
              source_type: 'panel_render',
              panelMeta: { submitLabel: data.submit_label, submitted: false },
            }
            setMessages((prev) => {
              const msgs = [...prev]
              const lastIdx = msgs.length - 1
              if (lastIdx >= 0 && msgs[lastIdx].role === 'assistant' && msgs[lastIdx].source_type !== 'panel_render') {
                msgs.splice(lastIdx, 0, panelMsg)
              } else {
                msgs.push(panelMsg)
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
            setMessages((prev) => applyAssistantContent(prev, content))
          })
        }
      }

      // Single exit for the turn: flush whatever was streamed, drop the
      // placeholder if nothing arrived, and record a failure on the message
      // that started the turn so it survives a reload.
      const settle = (errorMessage?: string) => {
        if (pendingFrameRef.current !== null) {
          cancelAnimationFrame(pendingFrameRef.current)
          pendingFrameRef.current = null
        }
        const content = contentRef.current
        setMessages((prev) => {
          const flushed = content ? applyAssistantContent(prev, content) : prev
          const pruned = flushed.filter((m) => !(m.role === 'assistant' && m.content === ''))
          return errorMessage
            ? pruned.map((m) => (m.id === userMsgId ? { ...m, error: errorMessage } : m))
            : pruned
        })
        settleRef.current = null
        setStreaming(false)
      }
      settleRef.current = settle

      send(sessionId, { message }, onEvent, () => { settle() }, (err) => { settle(err.message) })
        .catch((err: unknown) => {
          settle(err instanceof Error ? err.message : 'Failed to send message')
        })
    },
    [sessionId, send],
  )

  const submitPanel = useCallback(
    (messageId: string, selections: string) => {
      setMessages((prev) =>
        prev.map((m) =>
          m.id === messageId && m.panelMeta
            ? { ...m, panelMeta: { ...m.panelMeta, submitted: true } }
            : m,
        ),
      )
      sendMessage(selections)
    },
    [sendMessage],
  )

  const cancelGeneration = useCallback(() => {
    cancelChat()
    settleRef.current?.()
  }, [cancelChat])

  return { messages, sendMessage, streaming, cancelChat: cancelGeneration, sessionId, submitPanel }
}

export { useWorkflowAgentChat }
