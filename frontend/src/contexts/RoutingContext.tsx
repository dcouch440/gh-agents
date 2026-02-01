import { createContext, useReducer, useEffect, type ReactNode } from 'react'
import { useWebSocket } from '../hooks/useWebSocket'
import { WS_CHANNEL } from '../constants'
import type { RoutingEvent } from '../types/routing'

// ── Constants ────────────────────────────────────────────────────────────────

const ROUTING_MAX_EVENTS = 200

// ── State ────────────────────────────────────────────────────────────────────

type RoutingState = {
  events: RoutingEvent[]
}

const initialState: RoutingState = { events: [] }

// ── Actions ──────────────────────────────────────────────────────────────────

type RoutingAction =
  | { type: 'APPEND'; event: RoutingEvent }
  | { type: 'UPDATE'; event: RoutingEvent }
  | { type: 'CLEAR' }

const reducer = (state: RoutingState, action: RoutingAction): RoutingState => {
  switch (action.type) {
    case 'APPEND': {
      const next = [action.event, ...state.events]
      return { events: next.length > ROUTING_MAX_EVENTS ? next.slice(0, ROUTING_MAX_EVENTS) : next }
    }
    case 'UPDATE':
      return {
        events: state.events.map((e) => (e.id === action.event.id ? action.event : e)),
      }
    case 'CLEAR':
      return initialState
  }
}

// ── Context ──────────────────────────────────────────────────────────────────

type RoutingContextValue = RoutingState

const RoutingContext = createContext<RoutingContextValue | null>(null)

// ── Provider ─────────────────────────────────────────────────────────────────

function RoutingProvider({ children }: { children: ReactNode }) {
  const [state, dispatch] = useReducer(reducer, initialState)
  const { subscribe } = useWebSocket()

  useEffect(() => {
    const unsub = subscribe(WS_CHANNEL.ROUTING, (data) => {
      const msg = data as { event?: RoutingEvent; completed?: boolean }
      if (msg.event) {
        if (msg.completed) {
          dispatch({ type: 'UPDATE', event: msg.event })
        } else {
          dispatch({ type: 'APPEND', event: msg.event })
        }
      }
    })
    return unsub
  }, [subscribe])

  return (
    <RoutingContext.Provider value={state}>
      {children}
    </RoutingContext.Provider>
  )
}

export { RoutingContext, RoutingProvider }
export type { RoutingContextValue }
