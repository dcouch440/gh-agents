import { createContext, useCallback, useEffect, useMemo, useRef } from 'react'
import type { ReactNode } from 'react'
import { WS_URL, WS_RECONNECT_BASE_MS, WS_RECONNECT_MAX_MS } from '@/constants'
import { WS_STATUS, WS_MSG, WS_CONTROL } from '@/types/ws'
import type { WsTopic, WsWireMessage, WsClientMessage, WsEventHandler, CanvasAckMsg } from '@/types/ws'
import { useStore } from '@/stores/lib'
import { authStore } from '@/stores/authStore'
import { wsConnectionStore } from '@/stores/wsConnectionStore'

// ── Types ────────────────────────────────────────────────────────────────────

type CanvasAckHandler = (ack: CanvasAckMsg) => void

type WebSocketContextValue = {
  subscribe: (topic: WsTopic, handler: WsEventHandler) => () => void
  subscribeRun: (runId: string) => () => void
  send: (message: WsClientMessage) => void
  /** Canvas mutation acks arrive on the control channel, not on a topic. */
  subscribeCanvasAck: (handler: CanvasAckHandler) => () => void
}

// ── Context ──────────────────────────────────────────────────────────────────

const WebSocketContext = createContext<WebSocketContextValue | null>(null)

// ── Provider ─────────────────────────────────────────────────────────────────

const PING_INTERVAL_MS = 30_000
/** Cap on messages buffered while the socket is down, oldest dropped first. */
const PENDING_SEND_LIMIT = 200

function WebSocketProvider({ children }: { children: ReactNode }) {
  const socketRef = useRef<WebSocket | null>(null)
  const handlersRef = useRef(new Map<WsTopic, Set<WsEventHandler>>())
  const activeRunsRef = useRef(new Set<string>())
  const reconnectAttemptRef = useRef(0)
  const reconnectTimerRef = useRef<ReturnType<typeof setTimeout> | null>(null)
  const pingTimerRef = useRef<ReturnType<typeof setInterval> | null>(null)
  // Messages written while the socket was down, replayed in order on reconnect.
  const pendingRef = useRef<string[]>([])
  const canvasAckHandlersRef = useRef(new Set<CanvasAckHandler>())

  const token = useStore(authStore.store, (s) => s.token)

  // ── Stable helper: send JSON if socket is open ──

  const sendRaw = useCallback((data: unknown) => {
    const ws = socketRef.current
    if (ws?.readyState === WebSocket.OPEN) {
      ws.send(JSON.stringify(data))
      return
    }
    // Queue rather than drop. Canvas mutations are the user's work — silently
    // losing a node created during a reconnect leaves the board and the server
    // permanently disagreeing, with nothing to retry from.
    const queue = pendingRef.current
    queue.push(JSON.stringify(data))
    if (queue.length > PENDING_SEND_LIMIT) {
      queue.splice(0, queue.length - PENDING_SEND_LIMIT)
      console.warn('[ws] outbound queue full, dropped oldest messages')
    }
  }, [])

  // ── Connection lifecycle (driven by token) ──

  useEffect(() => {
    if (!token) {
      // No token — ensure disconnected
      if (reconnectTimerRef.current !== null) {
        clearTimeout(reconnectTimerRef.current)
        reconnectTimerRef.current = null
      }
      if (pingTimerRef.current !== null) {
        clearInterval(pingTimerRef.current)
        pingTimerRef.current = null
      }
      if (socketRef.current) {
        socketRef.current.close()
        socketRef.current = null
      }
      wsConnectionStore.setState({ status: WS_STATUS.DISCONNECTED, latency: null })
      return
    }

    let cancelled = false

    const getReconnectDelay = (): number => {
      const attempt = reconnectAttemptRef.current
      const base = Math.min(WS_RECONNECT_BASE_MS * Math.pow(2, attempt), WS_RECONNECT_MAX_MS)
      const jitter = base * 0.25 * (Math.random() * 2 - 1)
      return Math.max(0, Math.round(base + jitter))
    }

    const startPingTimer = () => {
      if (pingTimerRef.current !== null) clearInterval(pingTimerRef.current)
      pingTimerRef.current = setInterval(() => {
        sendRaw({ type: WS_MSG.PING, ts: new Date().toISOString() })
      }, PING_INTERVAL_MS)
    }

    const handleControlMessage = (msg: Record<string, unknown>) => {
      if (msg.type === WS_CONTROL.PONG && typeof msg.client_ts === 'string') {
        const rtt = Date.now() - new Date(msg.client_ts).getTime()
        if (!cancelled) wsConnectionStore.setState({ latency: rtt })
      } else if (msg.type === WS_CONTROL.EVENTS_MISSED) {
        const count = typeof msg.missed_count === 'number' ? msg.missed_count : 0
        console.warn(`[ws] Missed ${count} events. Triggering data re-fetch.`)
        window.dispatchEvent(new CustomEvent('ws:events-missed', { detail: { missed_count: count } }))
      } else if (msg.type === WS_CONTROL.CANVAS_ACK && typeof msg.seq === 'number') {
        const ack: CanvasAckMsg = {
          type: WS_CONTROL.CANVAS_ACK,
          seq: msg.seq,
          element_id: typeof msg.element_id === 'string' ? msg.element_id : '',
          error: typeof msg.error === 'string' ? msg.error : null,
        }
        for (const handler of canvasAckHandlersRef.current) {
          try {
            handler(ack)
          } catch (err) {
            console.error('[ws] canvas ack handler error:', err)
          }
        }
      }
    }

    const doConnect = () => {
      if (cancelled) return

      // Tear down previous socket if any
      if (socketRef.current) {
        socketRef.current.onopen = null
        socketRef.current.onmessage = null
        socketRef.current.onclose = null
        socketRef.current.onerror = null
        socketRef.current.close()
        socketRef.current = null
      }

      wsConnectionStore.setState({ status: WS_STATUS.CONNECTING })

      const ws = new WebSocket(`${WS_URL}?token=${encodeURIComponent(token)}`)
      socketRef.current = ws

      ws.onopen = () => {
        if (cancelled) {
          ws.close()
          return
        }
        wsConnectionStore.setState({ status: WS_STATUS.CONNECTED })
        reconnectAttemptRef.current = 0

        // Re-subscribe to all topics that have active handlers
        const activeTopics: WsTopic[] = []
        for (const [topic, handlers] of handlersRef.current) {
          if (handlers.size > 0) activeTopics.push(topic)
        }
        if (activeTopics.length > 0) {
          ws.send(JSON.stringify({ type: WS_MSG.SUBSCRIBE, topics: activeTopics }))
        }

        // Re-subscribe to active runs
        for (const runId of activeRunsRef.current) {
          ws.send(JSON.stringify({ type: WS_MSG.SUBSCRIBE_RUN, run_id: runId }))
        }

        // Replay anything written while we were down, in order and after the
        // resubscribes so the server has our topics first.
        const pending = pendingRef.current
        if (pending.length > 0) {
          pendingRef.current = []
          for (const payload of pending) {
            ws.send(payload)
          }
        }

        startPingTimer()
      }

      ws.onmessage = (event: MessageEvent) => {
        if (cancelled) return
        try {
          const parsed = JSON.parse(event.data as string) as Record<string, unknown>

          // Control messages have a 'type' field
          if ('type' in parsed) {
            handleControlMessage(parsed)
            return
          }

          // Broadcast messages have a 'topic' field
          if ('topic' in parsed) {
            const msg = parsed as unknown as WsWireMessage
            const handlers = handlersRef.current.get(msg.topic)
            if (handlers) {
              for (const handler of handlers) {
                try {
                  handler(msg)
                } catch (err) {
                  console.error(`[ws] handler error (${msg.topic}/${msg.event}):`, err)
                }
              }
            }
          }
        } catch {
          // Non-JSON message — ignore
        }
      }

      ws.onclose = () => {
        if (cancelled) return
        socketRef.current = null
        wsConnectionStore.setState({ status: WS_STATUS.DISCONNECTED, latency: null })
        if (pingTimerRef.current !== null) {
          clearInterval(pingTimerRef.current)
          pingTimerRef.current = null
        }

        // Schedule reconnect with backoff
        const delay = getReconnectDelay()
        reconnectAttemptRef.current += 1
        reconnectTimerRef.current = setTimeout(() => {
          reconnectTimerRef.current = null
          doConnect()
        }, delay)
      }

      ws.onerror = () => {
        // onclose fires after onerror — reconnect handled there
      }
    }

    doConnect()

    return () => {
      cancelled = true
      if (reconnectTimerRef.current !== null) {
        clearTimeout(reconnectTimerRef.current)
        reconnectTimerRef.current = null
      }
      if (pingTimerRef.current !== null) {
        clearInterval(pingTimerRef.current)
        pingTimerRef.current = null
      }
      if (socketRef.current) {
        socketRef.current.onopen = null
        socketRef.current.onmessage = null
        socketRef.current.onclose = null
        socketRef.current.onerror = null
        socketRef.current.close()
        socketRef.current = null
      }
    }
  }, [token, sendRaw])

  // ── Public API (stable references) ──

  const subscribe = useCallback(
    (topic: WsTopic, handler: WsEventHandler): (() => void) => {
      if (!handlersRef.current.has(topic)) {
        handlersRef.current.set(topic, new Set())
      }
      const handlers = handlersRef.current.get(topic)!
      const wasEmpty = handlers.size === 0
      handlers.add(handler)

      if (wasEmpty) {
        sendRaw({ type: WS_MSG.SUBSCRIBE, topics: [topic] })
      }

      return () => {
        handlers.delete(handler)
        if (handlers.size === 0) {
          sendRaw({ type: WS_MSG.UNSUBSCRIBE, topics: [topic] })
        }
      }
    },
    [sendRaw],
  )

  const subscribeRun = useCallback(
    (runId: string): (() => void) => {
      activeRunsRef.current.add(runId)
      sendRaw({ type: WS_MSG.SUBSCRIBE_RUN, run_id: runId })

      return () => {
        activeRunsRef.current.delete(runId)
        sendRaw({ type: WS_MSG.UNSUBSCRIBE_RUN, run_id: runId })
      }
    },
    [sendRaw],
  )

  const send = useCallback(
    (message: WsClientMessage): void => {
      sendRaw(message)
    },
    [sendRaw],
  )

  const subscribeCanvasAck = useCallback((handler: CanvasAckHandler): (() => void) => {
    canvasAckHandlersRef.current.add(handler)
    return () => {
      canvasAckHandlersRef.current.delete(handler)
    }
  }, [])

  const value = useMemo(
    () => ({ subscribe, subscribeRun, send, subscribeCanvasAck }),
    [subscribe, subscribeRun, send, subscribeCanvasAck],
  )

  return <WebSocketContext.Provider value={value}>{children}</WebSocketContext.Provider>
}

export { WebSocketContext, WebSocketProvider }
