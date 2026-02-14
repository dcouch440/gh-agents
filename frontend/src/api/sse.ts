import { API_BASE, LS_AUTH_TOKEN } from '@/constants'
import { ApiError } from './client'
import type { RequestConfig } from './client'

type SSEEvent = {
  readonly event: string
  readonly data: string
}

type SSECallbacks = {
  onEvent: (event: SSEEvent) => void
  onDone: () => void
  onError: (error: ApiError) => void
}

const createSSEStream = (
  path: string,
  callbacks: SSECallbacks,
  config?: Pick<RequestConfig, 'headers' | 'signal'>,
): (() => void) => {
  const controller = new AbortController()
  const token = localStorage.getItem(LS_AUTH_TOKEN)
  const url = `${API_BASE}${path}`

  const headers: Record<string, string> = {
    Accept: 'text/event-stream',
    ...config?.headers,
  }
  if (token) {
    headers['Authorization'] = `Bearer ${token}`
  }

  // Forward external signal
  if (config?.signal) {
    if (config.signal.aborted) {
      controller.abort()
    } else {
      config.signal.addEventListener('abort', () => controller.abort())
    }
  }

  fetch(url, {
    headers,
    signal: controller.signal,
  })
    .then(async (res) => {
      if (!res.ok) {
        callbacks.onError(ApiError.http(url, res.status, res.statusText, null))
        return
      }

      const reader = res.body?.getReader()
      if (!reader) {
        callbacks.onError(ApiError.network(url, new Error('Response body is not readable')))
        return
      }

      const decoder = new TextDecoder()
      let buffer = ''
      let currentEvent = 'message'

      for (;;) {
        const { done, value } = await reader.read()
        if (done) {
          callbacks.onDone()
          break
        }

        buffer += decoder.decode(value, { stream: true })
        const lines = buffer.split('\n')
        buffer = lines.pop() ?? ''

        for (const line of lines) {
          if (line.startsWith('event: ')) {
            currentEvent = line.slice(7).trim()
          } else if (line.startsWith('data: ')) {
            const data = line.slice(6)
            if (currentEvent === 'done' || data === '[DONE]') {
              callbacks.onDone()
              return
            }
            callbacks.onEvent(Object.freeze({ event: currentEvent, data }))
            currentEvent = 'message'
          } else if (line === '') {
            currentEvent = 'message'
          }
        }
      }
    })
    .catch((err: unknown) => {
      if (err instanceof DOMException && err.name === 'AbortError') return
      const original = err instanceof Error ? err : new Error('SSE connection failed')
      callbacks.onError(ApiError.network(url, original))
    })

  return () => controller.abort()
}

export { createSSEStream }
export type { SSECallbacks, SSEEvent }
