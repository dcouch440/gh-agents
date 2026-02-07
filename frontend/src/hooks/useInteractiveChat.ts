import { useState, useEffect, useCallback, useRef } from 'react'
import { useStore, executionStore } from '@/stores'
import type { ExecutionMessage } from '@/types/execution'

const useInteractiveChat = (executionId: string) => {
  const messages: ExecutionMessage[] = useStore(
    executionStore.store,
    executionStore.selectMessages(executionId),
  )

  const [loading, setLoading] = useState(true)
  const [sending, setSending] = useState(false)
  const [streaming, setStreaming] = useState(false)
  const [error, setError] = useState<string | null>(null)
  const mountedRef = useRef(true)

  useEffect(() => {
    mountedRef.current = true
    if (!executionId) {
      setLoading(false)
      return
    }
    setLoading(true)
    setError(null)
    void executionStore.fetchMessages(executionId).finally(() => {
      if (mountedRef.current) setLoading(false)
    })
    return () => {
      mountedRef.current = false
      executionStore.stopStream(executionId)
    }
  }, [executionId])

  // Track streaming state from store's activeStreams
  useEffect(() => {
    if (!executionId) return
    return executionStore.store.subscribe(() => {
      const streams = executionStore.store.getState().activeStreams
      const isStreaming = streams[executionId] !== null && streams[executionId] !== undefined
      if (mountedRef.current) setStreaming(isStreaming)
    })
  }, [executionId])

  const sendMessage = useCallback(async (content: string) => {
    if (!executionId) return
    setSending(true)
    setError(null)
    try {
      await executionStore.sendMessage(executionId, content)
    } catch (e) {
      if (mountedRef.current) {
        setError(e instanceof Error ? e.message : 'Failed to send message')
      }
    } finally {
      if (mountedRef.current) setSending(false)
    }
  }, [executionId])

  const approve = useCallback(async (structuredOutput?: Record<string, unknown>) => {
    if (!executionId) return
    setSending(true)
    try {
      await executionStore.approve(executionId, structuredOutput)
    } catch (e) {
      if (mountedRef.current) {
        setError(e instanceof Error ? e.message : 'Failed to approve execution')
      }
    } finally {
      if (mountedRef.current) setSending(false)
    }
  }, [executionId])

  const abort = useCallback(() => {
    executionStore.stopStream(executionId)
  }, [executionId])

  const reload = useCallback(() => {
    void executionStore.fetchMessages(executionId)
  }, [executionId])

  return { messages, loading, sending, streaming, error, sendMessage, approve, abort, reload }
}

export { useInteractiveChat }
