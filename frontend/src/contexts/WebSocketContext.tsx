import { createContext, useEffect, useRef, useCallback, useState, type ReactNode } from 'react'
import { WS_URL, WS_RECONNECT_BASE_MS, WS_RECONNECT_MAX_MS, type WsChannel, type WsEvent } from '@/constants'
import { useAuth } from '@/hooks/useAuth'

type WsStatus = 'connecting' | 'connected' | 'disconnected'

type MessageHandler = (data: unknown) => void

type WebSocketState = {
  status: WsStatus
  subscribe: (channel: WsChannel, handler: MessageHandler) => () => void
  subscribeRun: (runId: string, event: WsEvent, handler: MessageHandler) => () => void
  unsubscribeRun: (runId: string) => void
  send: (msg: Record<string, unknown>) => void
}

const WebSocketContext = createContext<WebSocketState | null>(null)

function WebSocketProvider({ children }: { children: ReactNode }) {
  const { token } = useAuth()
  const wsRef = useRef<WebSocket | null>(null)
  const handlersRef = useRef<Map<WsChannel, Set<MessageHandler>>>(new Map())
  const subscribedChannelsRef = useRef<Set<WsChannel>>(new Set())
  const runHandlersRef = useRef<Map<string, Map<WsEvent, Set<MessageHandler>>>>(new Map())
  const reconnectAttemptRef = useRef(0)
  const reconnectTimerRef = useRef<ReturnType<typeof setTimeout> | null>(null)
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
    // Re-subscribe to any active runs
    for (const runId of runHandlersRef.current.keys()) {
      sendJson({ action: 'subscribe_run', run_id: runId })
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
          const msg = JSON.parse(event.data as string) as Record<string, unknown>

          // Route run-scoped events (have `event` and `run_id` fields)
          const eventType = msg.event as string | undefined
          const runId = msg.run_id as string | undefined
          if (eventType && runId) {
            const runMap = runHandlersRef.current.get(runId)
            if (runMap) {
              const handlers = runMap.get(eventType as WsEvent)
              if (handlers) {
                for (const handler of handlers) {
                  handler(msg)
                }
              }
            }
          }

          // Route channel-based messages (have `type` field, data in `data`)
          const channel = (msg.channel ?? msg.type) as string | undefined
          if (channel) {
            const handlers = handlersRef.current.get(channel as WsChannel)
            if (handlers) {
              // Pass the `data` payload to handlers, not the full envelope
              const payload = msg.data ?? msg
              for (const handler of handlers) {
                handler(payload)
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
      if (reconnectTimerRef.current) clearTimeout(reconnectTimerRef.current)
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

  const subscribeRun = useCallback((runId: string, event: WsEvent, handler: MessageHandler) => {
    if (!runHandlersRef.current.has(runId)) {
      runHandlersRef.current.set(runId, new Map())
      sendJson({ action: 'subscribe_run', run_id: runId })
    }
    const runMap = runHandlersRef.current.get(runId)!
    if (!runMap.has(event)) {
      runMap.set(event, new Set())
    }
    runMap.get(event)!.add(handler)

    return () => {
      const rm = runHandlersRef.current.get(runId)
      if (!rm) return
      const handlers = rm.get(event)
      if (handlers) {
        handlers.delete(handler)
        if (handlers.size === 0) rm.delete(event)
      }
      // If no more handlers for this run, unsubscribe
      if (rm.size === 0) {
        runHandlersRef.current.delete(runId)
        sendJson({ action: 'unsubscribe_run', run_id: runId })
      }
    }
  }, [sendJson])

  const unsubscribeRun = useCallback((runId: string) => {
    runHandlersRef.current.delete(runId)
    sendJson({ action: 'unsubscribe_run', run_id: runId })
  }, [sendJson])

  const send = useCallback((msg: Record<string, unknown>) => {
    sendJson(msg)
  }, [sendJson])

  return (
    <WebSocketContext.Provider value={{ status, subscribe, subscribeRun, unsubscribeRun, send }}>
      {children}
    </WebSocketContext.Provider>
  )
}

export { WebSocketContext, WebSocketProvider }
export type { WebSocketState, WsStatus, MessageHandler }
