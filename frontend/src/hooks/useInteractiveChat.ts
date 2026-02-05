import { useState, useEffect, useCallback, useRef } from 'react'
import { api } from '@/api'
import { API } from '@/constants'
import { createSSEStream } from '@/api/sse'
import type { ExecutionMessage } from '@/types/execution'

const useInteractiveChat = (executionId: string) => {
  const [messages, setMessages] = useState<ExecutionMessage[]>([])
  const [loading, setLoading] = useState(true)
  const [sending, setSending] = useState(false)
  const [streaming, setStreaming] = useState(false)
  const [error, setError] = useState<string | null>(null)
  const mountedRef = useRef(true)
  const abortRef = useRef<(() => void) | null>(null)

  const fetchMessages = useCallback(async () => {
    if (!executionId) return
    try {
      const data = await api.get<ExecutionMessage[]>(API.EXECUTION_MESSAGES(executionId))
      if (mountedRef.current) {
        setMessages(data)
        setError(null)
      }
    } catch (e) {
      if (mountedRef.current) {
        setError(e instanceof Error ? e.message : 'Failed to load messages')
      }
    }
  }, [executionId])

  useEffect(() => {
    mountedRef.current = true
    if (!executionId) {
      setMessages([])
      setLoading(false)
      return
    }
    setLoading(true)
    void fetchMessages().finally(() => {
      if (mountedRef.current) setLoading(false)
    })
    return () => {
      mountedRef.current = false
      if (abortRef.current) {
        abortRef.current()
        abortRef.current = null
      }
    }
  }, [fetchMessages, executionId])

  const sendMessage = useCallback(async (content: string) => {
    if (!executionId) return
    setSending(true)
    setError(null)
    try {
      const response = await api.agentExecutions.sendMessage(executionId, { content })

      if (!mountedRef.current) return

      // Optimistically append user message
      setMessages((prev) => [...prev, response.message])

      // Create temp assistant message for streaming
      const tempId = `streaming-${Date.now()}`
      let accumulated = ''

      setMessages((prev) => [
        ...prev,
        {
          id: tempId,
          agent_execution_id: executionId,
          role: 'assistant' as const,
          content: '',
          tool_call_id: null,
          input_tokens: 0,
          output_tokens: 0,
          created_at: new Date().toISOString(),
        },
      ])

      setStreaming(true)

      // Open SSE stream
      const abort = createSSEStream(
        API.EXECUTION_MESSAGE_STREAM(executionId, response.stream_id),
        {
          onEvent: (event) => {
            if (event.event === 'token') {
              const tokenText = JSON.parse(event.data) as string
              accumulated += tokenText
              const current = accumulated
              if (mountedRef.current) {
                setMessages((prev) =>
                  prev.map((m) => (m.id === tempId ? { ...m, content: current } : m)),
                )
              }
            }
          },
          onDone: () => {
            if (mountedRef.current) {
              setStreaming(false)
              // Refetch to get the final recorded messages
              void fetchMessages()
            }
          },
          onError: (err) => {
            if (mountedRef.current) {
              setStreaming(false)
              setError(err.message)
              void fetchMessages()
            }
          },
        },
      )

      abortRef.current = abort
    } catch (e) {
      if (mountedRef.current) {
        setError(e instanceof Error ? e.message : 'Failed to send message')
      }
    } finally {
      if (mountedRef.current) setSending(false)
    }
  }, [executionId, fetchMessages])

  const approve = useCallback(async (structuredOutput?: Record<string, unknown>) => {
    if (!executionId) return
    setSending(true)
    try {
      await api.agentExecutions.approve(executionId, structuredOutput ? { structured_output: structuredOutput } : undefined)
      await fetchMessages()
    } catch (e) {
      if (mountedRef.current) {
        setError(e instanceof Error ? e.message : 'Failed to approve execution')
      }
    } finally {
      if (mountedRef.current) setSending(false)
    }
  }, [executionId, fetchMessages])

  const abort = useCallback(() => {
    if (abortRef.current) {
      abortRef.current()
      abortRef.current = null
      setStreaming(false)
    }
  }, [])

  const reload = useCallback(() => {
    void fetchMessages()
  }, [fetchMessages])

  return { messages, loading, sending, streaming, error, sendMessage, approve, abort, reload }
}

export { useInteractiveChat }
