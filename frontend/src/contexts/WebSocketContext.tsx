import { createContext, useEffect, useRef, useCallback, useState, type ReactNode } from 'react'
import { WS_URL, WS_RECONNECT_BASE_MS, WS_RECONNECT_MAX_MS } from '@/constants'
import { useAuth } from '@/hooks/useAuth'
import { WS_STATUS, WS_MSG, WS_CONTROL } from '@/types/ws'
import type { WsTopic, WsWireMessage, WsClientMessage, WsControlMessage, WsStatus } from '@/types/ws'

type TopicHandler = (msg: WsWireMessage) => void

type WebSocketState = {
  status: WsStatus
  latency: number | null
  subscribe: (topic: WsTopic, handler: TopicHandler) => () => void
  subscribeRun: (runId: string, handler: TopicHandler) => () => void
  unsubscribeRun: (runId: string) => void
  sendJson: (data: WsClientMessage) => void
}

const HEARTBEAT_INTERVAL_MS = 30_000

const WebSocketContext = createContext<WebSocketState | null>(null)

function WebSocketProvider({ children }: { children: ReactNode }) {
  const { token } = useAuth()
  const wsRef = useRef<WebSocket | null>(null)
  const topicHandlersRef = useRef<Map<WsTopic, Set<TopicHandler>>>(new Map())
  const subscribedTopicsRef = useRef<Set<WsTopic>>(new Set())
  const runHandlersRef = useRef<Map<string, Set<TopicHandler>>>(new Map())
  const reconnectAttemptRef = useRef(0)
  const reconnectTimerRef = useRef<ReturnType<typeof setTimeout> | null>(null)
  const heartbeatTimerRef = useRef<ReturnType<typeof setInterval> | null>(null)
  const [status, setStatus] = useState<WsStatus>(WS_STATUS.DISCONNECTED)
  const [latency, setLatency] = useState<number | null>(null)
  const tokenRef = useRef(token)

  const sendJson = useCallback((data: WsClientMessage) => {
    const ws = wsRef.current
    if (ws?.readyState === WebSocket.OPEN) {
      ws.send(JSON.stringify(data))
    }
  }, [])

  const syncSubscriptions = useCallback(() => {
    const topics = Array.from(subscribedTopicsRef.current)
    if (topics.length > 0) {
      sendJson({ type: WS_MSG.SUBSCRIBE, topics })
    }
    for (const runId of runHandlersRef.current.keys()) {
      sendJson({ type: WS_MSG.SUBSCRIBE_RUN, run_id: runId })
    }
  }, [sendJson])

  useEffect(() => {
    tokenRef.current = token
  }, [token])

  useEffect(() => {
    if (!token) return

    const openConnection = () => {
      const currentToken = tokenRef.current
      if (!currentToken) return

      const url = `${WS_URL}?token=${encodeURIComponent(currentToken)}`
      const ws = new WebSocket(url)
      wsRef.current = ws

      setStatus(WS_STATUS.CONNECTING)

      ws.addEventListener('open', () => {
        setStatus(WS_STATUS.CONNECTED)
        reconnectAttemptRef.current = 0
        syncSubscriptions()

        // Start heartbeat
        if (heartbeatTimerRef.current) clearInterval(heartbeatTimerRef.current)
        heartbeatTimerRef.current = setInterval(() => {
          sendJson({ type: WS_MSG.PING, ts: new Date().toISOString() })
        }, HEARTBEAT_INTERVAL_MS)
      })

      ws.addEventListener('message', (event) => {
        try {
          const raw = JSON.parse(event.data as string) as Record<string, unknown>

          // Handle control messages (type-tagged: subscribed, error, pong)
          if ('type' in raw) {
            const ctrl = raw as WsControlMessage
            if (ctrl.type === WS_CONTROL.PONG) {
              const sent = new Date(ctrl.client_ts).getTime()
              setLatency(Date.now() - sent)
            }
            return
          }

          // Handle broadcast events (topic-tagged wire messages)
          if ('topic' in raw && 'event' in raw && 'data' in raw) {
            const msg = raw as unknown as WsWireMessage

            // Topic handlers
            const handlers = topicHandlersRef.current.get(msg.topic)
            if (handlers) {
              for (const handler of handlers) {
                handler(msg)
              }
            }

            // Run-scoped handlers
            if (msg.run_id) {
              const runSet = runHandlersRef.current.get(msg.run_id)
              if (runSet) {
                for (const handler of runSet) {
                  handler(msg)
                }
              }
            }
          }
        } catch {
          // ignore malformed messages
        }
      })

      ws.addEventListener('close', () => {
        setStatus(WS_STATUS.DISCONNECTED)
        wsRef.current = null
        if (heartbeatTimerRef.current) {
          clearInterval(heartbeatTimerRef.current)
          heartbeatTimerRef.current = null
        }

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
      if (heartbeatTimerRef.current) clearInterval(heartbeatTimerRef.current)
      wsRef.current?.close()
      wsRef.current = null
    }
  }, [token, syncSubscriptions, sendJson])

  const subscribe = useCallback((topic: WsTopic, handler: TopicHandler) => {
    if (!topicHandlersRef.current.has(topic)) {
      topicHandlersRef.current.set(topic, new Set())
    }
    topicHandlersRef.current.get(topic)!.add(handler)

    if (!subscribedTopicsRef.current.has(topic)) {
      subscribedTopicsRef.current.add(topic)
      sendJson({ type: WS_MSG.SUBSCRIBE, topics: [topic] })
    }

    return () => {
      const handlers = topicHandlersRef.current.get(topic)
      if (handlers) {
        handlers.delete(handler)
        if (handlers.size === 0) {
          topicHandlersRef.current.delete(topic)
          subscribedTopicsRef.current.delete(topic)
          sendJson({ type: WS_MSG.UNSUBSCRIBE, topics: [topic] })
        }
      }
    }
  }, [sendJson])

  const subscribeRun = useCallback((runId: string, handler: TopicHandler) => {
    if (!runHandlersRef.current.has(runId)) {
      runHandlersRef.current.set(runId, new Set())
      sendJson({ type: WS_MSG.SUBSCRIBE_RUN, run_id: runId })
    }
    runHandlersRef.current.get(runId)!.add(handler)

    return () => {
      const runSet = runHandlersRef.current.get(runId)
      if (runSet) {
        runSet.delete(handler)
        if (runSet.size === 0) {
          runHandlersRef.current.delete(runId)
          sendJson({ type: WS_MSG.UNSUBSCRIBE_RUN, run_id: runId })
        }
      }
    }
  }, [sendJson])

  const unsubscribeRun = useCallback((runId: string) => {
    runHandlersRef.current.delete(runId)
    sendJson({ type: WS_MSG.UNSUBSCRIBE_RUN, run_id: runId })
  }, [sendJson])

  return (
    <WebSocketContext.Provider value={{ status, latency, subscribe, subscribeRun, unsubscribeRun, sendJson }}>
      {children}
    </WebSocketContext.Provider>
  )
}

export { WebSocketContext, WebSocketProvider }
export type { WebSocketState, WsStatus, TopicHandler }
