import { useState, useCallback, useRef } from 'react'
import { api, createSSEStream } from '@/api'
import type { SendMessageRequest } from '@/types'

type SendMessageResponse = {
  message_id: string
}

const useSendMessage = () => {
  const [loading, setLoading] = useState(false)
  const [error, setError] = useState<string | null>(null)
  const [streaming, setStreaming] = useState(false)
  const abortRef = useRef<(() => void) | null>(null)

  const send = useCallback(async (
    body: SendMessageRequest,
    onChunk?: (data: string) => void,
    onDone?: () => void,
  ): Promise<string> => {
    setLoading(true)
    setError(null)
    try {
      const { message_id } = await api.post<SendMessageResponse>('/chat', body)

      if (onChunk) {
        setStreaming(true)
        abortRef.current = createSSEStream(`/chat/${message_id}/stream`, {
          onMessage: onChunk,
          onDone: () => {
            setStreaming(false)
            abortRef.current = null
            onDone?.()
          },
          onError: (e) => {
            setStreaming(false)
            abortRef.current = null
            setError(e.message)
          },
        })
      }

      return message_id
    } catch (e) {
      const msg = e instanceof Error ? e.message : 'Failed to send message'
      setError(msg)
      throw e
    } finally {
      setLoading(false)
    }
  }, [])

  const abort = useCallback(() => {
    abortRef.current?.()
    abortRef.current = null
    setStreaming(false)
  }, [])

  return { send, abort, loading, streaming, error }
}

const useSendSessionMessage = () => {
  const [loading, setLoading] = useState(false)
  const [error, setError] = useState<string | null>(null)
  const [streaming, setStreaming] = useState(false)
  const abortRef = useRef<(() => void) | null>(null)

  const send = useCallback(async (
    sessionId: string,
    body: SendMessageRequest,
    onChunk?: (data: string) => void,
    onDone?: () => void,
  ): Promise<string> => {
    setLoading(true)
    setError(null)
    try {
      const { message_id } = await api.post<SendMessageResponse>(`/sessions/${sessionId}/chat`, body)

      if (onChunk) {
        setStreaming(true)
        abortRef.current = createSSEStream(`/sessions/${sessionId}/chat/${message_id}/stream`, {
          onMessage: onChunk,
          onDone: () => {
            setStreaming(false)
            abortRef.current = null
            onDone?.()
          },
          onError: (e) => {
            setStreaming(false)
            abortRef.current = null
            setError(e.message)
          },
        })
      }

      return message_id
    } catch (e) {
      const msg = e instanceof Error ? e.message : 'Failed to send message'
      setError(msg)
      throw e
    } finally {
      setLoading(false)
    }
  }, [])

  const abort = useCallback(() => {
    abortRef.current?.()
    abortRef.current = null
    setStreaming(false)
  }, [])

  return { send, abort, loading, streaming, error }
}

const useClearHistory = () => {
  const [loading, setLoading] = useState(false)
  const [error, setError] = useState<string | null>(null)

  const mutate = useCallback(async (): Promise<void> => {
    setLoading(true)
    setError(null)
    try {
      await api.del('/chat/history')
    } catch (e) {
      const msg = e instanceof Error ? e.message : 'Failed to clear history'
      setError(msg)
      throw e
    } finally {
      setLoading(false)
    }
  }, [])

  return { mutate, loading, error }
}

export { useSendMessage, useSendSessionMessage, useClearHistory }
