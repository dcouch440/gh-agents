import { useState, useCallback, useRef } from 'react'
import { api, createSSEStream } from '@/api'
import type { SSEEvent } from '@/api'
import { API } from '@/constants'
import type { SendMessageRequest } from '@/types'

type SendMessageResponse = {
  message_id: string
  status: string
}

const useSendSessionMessage = () => {
  const [loading, setLoading] = useState(false)
  const [error, setError] = useState<string | null>(null)
  const [streaming, setStreaming] = useState(false)
  const abortRef = useRef<(() => void) | null>(null)

  const send = useCallback(
    async (sessionId: string, body: SendMessageRequest, onEvent?: (event: SSEEvent) => void, onDone?: () => void): Promise<string> => {
      setLoading(true)
      setError(null)
      try {
        const { message_id } = await api.post<SendMessageResponse>(API.SESSION_CHAT(sessionId), body)

        if (onEvent) {
          setStreaming(true)
          abortRef.current = createSSEStream(API.SESSION_CHAT_STREAM(sessionId, message_id), {
            onEvent,
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
    },
    [],
  )

  const abort = useCallback(() => {
    abortRef.current?.()
    abortRef.current = null
    setStreaming(false)
  }, [])

  return { send, abort, loading, streaming, error }
}

export { useSendSessionMessage }
