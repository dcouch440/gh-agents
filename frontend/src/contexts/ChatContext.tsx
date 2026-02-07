import { createContext, useReducer, useEffect, useCallback, useRef, type ReactNode } from 'react'
import { ACTION } from '@/constants'
import { api } from '@/api'
import type { ChatMessage } from '@/types/session'

// ── State ────────────────────────────────────────────────────────────────────

type ChatState = {
  messages: ChatMessage[]
  loading: boolean
  error: string | null
}

const initialState: ChatState = { messages: [], loading: true, error: null }

// ── Actions ──────────────────────────────────────────────────────────────────

type ChatAction =
  | { type: typeof ACTION.SET_ALL; messages: ChatMessage[] }
  | { type: typeof ACTION.APPEND; message: ChatMessage }
  | { type: typeof ACTION.SET_LOADING; loading: boolean }
  | { type: typeof ACTION.SET_ERROR; error: string }
  | { type: typeof ACTION.CLEAR }

const reducer = (state: ChatState, action: ChatAction): ChatState => {
  switch (action.type) {
    case ACTION.SET_ALL:
      return { messages: action.messages, loading: false, error: null }
    case ACTION.APPEND:
      return { ...state, messages: [...state.messages, action.message] }
    case ACTION.SET_LOADING:
      return { ...state, loading: action.loading }
    case ACTION.SET_ERROR:
      return { ...state, loading: false, error: action.error }
    case ACTION.CLEAR:
      return initialState
  }
}

// ── Context ──────────────────────────────────────────────────────────────────

type ChatContextValue = ChatState & { reload: () => void }

const ChatContext = createContext<ChatContextValue | null>(null)

// ── Provider ─────────────────────────────────────────────────────────────────

type ChatProviderProps = {
  sessionId: string
  children: ReactNode
}

function ChatProvider({ sessionId, children }: ChatProviderProps) {
  const [state, dispatch] = useReducer(reducer, initialState)
  const mountedRef = useRef(true)

  const load = useCallback(async () => {
    dispatch({ type: ACTION.SET_LOADING, loading: true })
    try {
      const messages = await api.sessions.getHistory(sessionId)
      if (mountedRef.current) dispatch({ type: ACTION.SET_ALL, messages })
    } catch (e) {
      if (mountedRef.current) dispatch({ type: ACTION.SET_ERROR, error: e instanceof Error ? e.message : 'Failed to load chat history' })
    }
  }, [sessionId])

  // Reset and reload when sessionId changes
  useEffect(() => {
    mountedRef.current = true
    dispatch({ type: ACTION.CLEAR })
    void load()
    return () => { mountedRef.current = false }
  }, [load])

  return (
    <ChatContext.Provider value={{ ...state, reload: () => { void load() } }}>
      {children}
    </ChatContext.Provider>
  )
}

export { ChatContext, ChatProvider }
export type { ChatContextValue }
