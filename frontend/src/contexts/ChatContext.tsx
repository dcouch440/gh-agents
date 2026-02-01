import { createContext, useReducer, useEffect, useCallback, useRef, type ReactNode } from 'react'
import { useWebSocket } from '@/hooks/useWebSocket'
import { ACTION, WS_CHANNEL, USE_MOCK_DATA, API } from '@/constants'
import { api } from '@/api'
import { mock } from '@/mock'
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
  const { subscribe } = useWebSocket()
  const mountedRef = useRef(true)

  const load = useCallback(async () => {
    dispatch({ type: ACTION.SET_LOADING, loading: true })
    try {
      const messages = USE_MOCK_DATA
        ? await mock.getChatHistory(sessionId)
        : await api.get<ChatMessage[]>(API.SESSION_HISTORY(sessionId))
      if (mountedRef.current) dispatch({ type: ACTION.SET_ALL, messages })
    } catch (e) {
      if (mountedRef.current) dispatch({ type: ACTION.SET_ERROR, error: e instanceof Error ? e.message : 'Failed to load chat history' })
    }
  }, [sessionId])

  // Reset and reload when sessionId changes
  useEffect(() => {
    mountedRef.current = true
    dispatch({ type: ACTION.CLEAR })
    load()
    return () => { mountedRef.current = false }
  }, [load])

  // WS subscription — filter messages for this session
  useEffect(() => {
    const unsub = subscribe(WS_CHANNEL.SESSIONS, (data) => {
      const msg = data as { session_id?: string; message?: ChatMessage }
      if (msg.session_id === sessionId && msg.message) {
        dispatch({ type: ACTION.APPEND, message: msg.message })
      }
    })
    return unsub
  }, [subscribe, sessionId])

  return (
    <ChatContext.Provider value={{ ...state, reload: load }}>
      {children}
    </ChatContext.Provider>
  )
}

export { ChatContext, ChatProvider }
export type { ChatContextValue }
