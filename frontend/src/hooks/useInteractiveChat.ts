import { useState, useEffect, useCallback, useRef } from 'react'
import { api } from '@/api'
import { API } from '@/constants'
import type { ExecutionMessage, ApproveExecutionRequest } from '@/types/execution'

const useInteractiveChat = (executionId: string) => {
  const [messages, setMessages] = useState<ExecutionMessage[]>([])
  const [loading, setLoading] = useState(true)
  const [sending, setSending] = useState(false)
  const [error, setError] = useState<string | null>(null)
  const mountedRef = useRef(true)

  const fetchMessages = useCallback(async () => {
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
    setLoading(true)
    void fetchMessages().finally(() => {
      if (mountedRef.current) setLoading(false)
    })
    return () => { mountedRef.current = false }
  }, [fetchMessages])

  const sendMessage = useCallback(async (content: string) => {
    setSending(true)
    try {
      await api.post(API.EXECUTION_MESSAGES(executionId), { content, role: 'user' })
      await fetchMessages()
    } catch (e) {
      if (mountedRef.current) {
        setError(e instanceof Error ? e.message : 'Failed to send message')
      }
    } finally {
      if (mountedRef.current) setSending(false)
    }
  }, [executionId, fetchMessages])

  const approve = useCallback(async (structuredOutput?: Record<string, unknown>) => {
    setSending(true)
    try {
      const body: ApproveExecutionRequest = structuredOutput ? { structured_output: structuredOutput } : {}
      await api.post(API.EXECUTION_APPROVE(executionId), body)
      await fetchMessages()
    } catch (e) {
      if (mountedRef.current) {
        setError(e instanceof Error ? e.message : 'Failed to approve execution')
      }
    } finally {
      if (mountedRef.current) setSending(false)
    }
  }, [executionId, fetchMessages])

  const reload = useCallback(() => {
    void fetchMessages()
  }, [fetchMessages])

  return { messages, loading, sending, error, sendMessage, approve, reload }
}

export { useInteractiveChat }
