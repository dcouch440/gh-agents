import { createContext, useReducer, type ReactNode } from 'react'
import { ACTION } from '@/constants'
import type { FeedItem } from '@/types/feed'

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
  const [state] = useReducer(reducer, initialState)

  return (
    <FeedContext.Provider value={state}>
      {children}
    </FeedContext.Provider>
  )
}

export { FeedContext, FeedProvider }
export type { FeedContextValue }
