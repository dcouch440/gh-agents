import { API_BASE, LS_AUTH_TOKEN } from '@/constants'

type SSECallbacks = {
  onMessage: (data: string) => void
  onDone: () => void
  onError: (error: Error) => void
}

const createSSEStream = (path: string, callbacks: SSECallbacks): (() => void) => {
  const controller = new AbortController()
  const token = localStorage.getItem(LS_AUTH_TOKEN)

  const headers: Record<string, string> = {
    Accept: 'text/event-stream',
  }
  if (token) {
    headers['Authorization'] = `Bearer ${token}`
  }

  fetch(`${API_BASE}${path}`, {
    headers,
    signal: controller.signal,
  })
    .then(async (res) => {
      if (!res.ok) {
        callbacks.onError(new Error(`SSE request failed: ${res.status} ${res.statusText}`))
        return
      }

      const reader = res.body?.getReader()
      if (!reader) {
        callbacks.onError(new Error('Response body is not readable'))
        return
      }

      const decoder = new TextDecoder()
      let buffer = ''

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
          if (line.startsWith('data: ')) {
            const data = line.slice(6)
            if (data === '[DONE]') {
              callbacks.onDone()
              return
            }
            callbacks.onMessage(data)
          }
        }
      }
    })
    .catch((err: unknown) => {
      if (err instanceof DOMException && err.name === 'AbortError') return
      callbacks.onError(err instanceof Error ? err : new Error('SSE connection failed'))
    })

  return () => controller.abort()
}

export { createSSEStream }
export type { SSECallbacks }
