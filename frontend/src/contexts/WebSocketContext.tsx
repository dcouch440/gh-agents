import { createContext, useEffect, useRef, useCallback, useState, type ReactNode } from 'react'
import { WS_URL, WS_RECONNECT_BASE_MS, WS_RECONNECT_MAX_MS, type WsChannel } from '../constants'
import { useAuth } from '../hooks/useAuth'

type WsStatus = 'connecting' | 'connected' | 'disconnected'

type MessageHandler = (data: unknown) => void

type WebSocketState = {
  status: WsStatus
  subscribe: (channel: WsChannel, handler: MessageHandler) => () => void
}

const WebSocketContext = createContext<WebSocketState | null>(null)

function WebSocketProvider({ children }: { children: ReactNode }) {
  const { token } = useAuth()
  const wsRef = useRef<WebSocket | null>(null)
  const handlersRef = useRef<Map<WsChannel, Set<MessageHandler>>>(new Map())
  const subscribedChannelsRef = useRef<Set<WsChannel>>(new Set())
  const reconnectAttemptRef = useRef(0)
  const reconnectTimerRef = useRef<ReturnType<typeof setTimeout>>()
  const [status, setStatus] = useState<WsStatus>('disconnected')
  const tokenRef = useRef(token)

  const sendJson = useCallback((data: unknown) => {
    const ws = wsRef.current
    if (ws?.readyState === WebSocket.OPEN) {
      ws.send(JSON.stringify(data))
    }
  }, [])

  const syncSubscriptions = useCallback(() => {
    const channels = Array.from(subscribedChannelsRef.current)
    if (channels.length > 0) {
      sendJson({ type: 'subscribe', channels })
    }
  }, [sendJson])

  // Keep tokenRef current so the WS event handlers can read it
  useEffect(() => {
    tokenRef.current = token
  }, [token])

  // Manage WS connection lifecycle as external system sync
  useEffect(() => {
    if (!token) return

    const openConnection = () => {
      const currentToken = tokenRef.current
      if (!currentToken) return

      const url = `${WS_URL}?token=${encodeURIComponent(currentToken)}`
      const ws = new WebSocket(url)
      wsRef.current = ws

      setStatus('connecting')

      ws.addEventListener('open', () => {
        setStatus('connected')
        reconnectAttemptRef.current = 0
        syncSubscriptions()
      })

      ws.addEventListener('message', (event) => {
        try {
          const msg = JSON.parse(event.data as string) as { type?: string; channel?: string }
          const channel = msg.channel ?? msg.type
          if (channel) {
            const handlers = handlersRef.current.get(channel as WsChannel)
            if (handlers) {
              for (const handler of handlers) {
                handler(msg)
              }
            }
          }
        } catch {
          // ignore malformed messages
        }
      })

      ws.addEventListener('close', () => {
        setStatus('disconnected')
        wsRef.current = null

        const delay = Math.min(
          WS_RECONNECT_BASE_MS * 2 ** reconnectAttemptRef.current,
          WS_RECONNECT_MAX_MS,
        )
        reconnectAttemptRef.current += 1
        reconnectTimerRef.current = setTimeout(openConnection, delay)
      })

      ws.addEventListener('error', () => {
        ws.close()
      })
    }

    openConnection()

    return () => {
      clearTimeout(reconnectTimerRef.current)
      wsRef.current?.close()
      wsRef.current = null
    }
  }, [token, syncSubscriptions])

  const subscribe = useCallback((channel: WsChannel, handler: MessageHandler) => {
    if (!handlersRef.current.has(channel)) {
      handlersRef.current.set(channel, new Set())
    }
    handlersRef.current.get(channel)!.add(handler)

    if (!subscribedChannelsRef.current.has(channel)) {
      subscribedChannelsRef.current.add(channel)
      sendJson({ type: 'subscribe', channels: [channel] })
    }

    return () => {
      const handlers = handlersRef.current.get(channel)
      if (handlers) {
        handlers.delete(handler)
        if (handlers.size === 0) {
          handlersRef.current.delete(channel)
          subscribedChannelsRef.current.delete(channel)
          sendJson({ type: 'unsubscribe', channels: [channel] })
        }
      }
    }
  }, [sendJson])

  return (
    <WebSocketContext.Provider value={{ status, subscribe }}>
      {children}
    </WebSocketContext.Provider>
  )
}

export { WebSocketContext, WebSocketProvider }
export type { WebSocketState, WsStatus, MessageHandler }
