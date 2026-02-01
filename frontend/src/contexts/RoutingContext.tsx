import { createContext, useReducer, useEffect, type ReactNode } from 'react'
import { useWebSocket } from '@/hooks/useWebSocket'
import { ACTION, WS_CHANNEL } from '@/constants'
import type { RoutingEvent } from '@/types/routing'

// ── Constants ────────────────────────────────────────────────────────────────

const ROUTING_MAX_EVENTS = 200

// ── State ────────────────────────────────────────────────────────────────────

type RoutingState = {
  events: RoutingEvent[]
}

const initialState: RoutingState = { events: [] }

// ── Actions ──────────────────────────────────────────────────────────────────

type RoutingAction =
  | { type: typeof ACTION.APPEND; event: RoutingEvent }
  | { type: typeof ACTION.UPDATE; event: RoutingEvent }
  | { type: typeof ACTION.CLEAR }

const reducer = (state: RoutingState, action: RoutingAction): RoutingState => {
  switch (action.type) {
    case ACTION.APPEND: {
      const next = [action.event, ...state.events]
      return { events: next.length > ROUTING_MAX_EVENTS ? next.slice(0, ROUTING_MAX_EVENTS) : next }
    }
    case ACTION.UPDATE:
      return {
        events: state.events.map((e) => (e.id === action.event.id ? action.event : e)),
      }
    case ACTION.CLEAR:
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
          dispatch({ type: ACTION.UPDATE, event: msg.event })
        } else {
          dispatch({ type: ACTION.APPEND, event: msg.event })
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
