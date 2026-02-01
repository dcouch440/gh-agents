import { createContext, useReducer, useEffect, type ReactNode } from 'react'
import { useWebSocket } from '../hooks/useWebSocket'
import { ACTION, WS_CHANNEL } from '../constants'
import type { FeedItem } from '../types/feed'

// ── Constants ────────────────────────────────────────────────────────────────

const FEED_MAX_ITEMS = 200

// ── State ────────────────────────────────────────────────────────────────────

type FeedState = {
  items: FeedItem[]
}

const initialState: FeedState = { items: [] }

// ── Actions ──────────────────────────────────────────────────────────────────

type FeedAction =
  | { type: typeof ACTION.APPEND; item: FeedItem }
  | { type: typeof ACTION.CLEAR }

const reducer = (state: FeedState, action: FeedAction): FeedState => {
  switch (action.type) {
    case ACTION.APPEND: {
      const next = [action.item, ...state.items]
      return { items: next.length > FEED_MAX_ITEMS ? next.slice(0, FEED_MAX_ITEMS) : next }
    }
    case ACTION.CLEAR:
      return initialState
  }
}

// ── Context ──────────────────────────────────────────────────────────────────

type FeedContextValue = FeedState

const FeedContext = createContext<FeedContextValue | null>(null)

// ── Provider ─────────────────────────────────────────────────────────────────

function FeedProvider({ children }: { children: ReactNode }) {
  const [state, dispatch] = useReducer(reducer, initialState)
  const { subscribe } = useWebSocket()

  useEffect(() => {
    const unsub = subscribe(WS_CHANNEL.FEED, (data) => {
      const msg = data as { item?: FeedItem }
      if (msg.item) dispatch({ type: ACTION.APPEND, item: msg.item })
    })
    return unsub
  }, [subscribe])

  return (
    <FeedContext.Provider value={state}>
      {children}
    </FeedContext.Provider>
  )
}

export { FeedContext, FeedProvider }
export type { FeedContextValue }
