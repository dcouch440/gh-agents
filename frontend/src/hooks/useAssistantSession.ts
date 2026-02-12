import { useReducer, useEffect, useCallback, useRef } from 'react'
import { api } from '@/api'
import type { SSEEvent } from '@/api'
import { useSendSessionMessage } from './useChatMutations'
import type { ChatMessageData } from '@/components/chat'
import type { Session, ChatMessage } from '@/types'

// ---------------------------------------------------------------------------
// State
// ---------------------------------------------------------------------------

type AssistantState = {
  session: Session | null
  messages: ChatMessageData[]
  isLoading: boolean
  error: string | null
}

type AssistantAction =
  | { type: 'INIT_START' }
  | { type: 'INIT_SESSION'; session: Session; messages: ChatMessageData[] }
  | { type: 'INIT_EMPTY' }
  | { type: 'INIT_ERROR'; error: string }
  | { type: 'SESSION_CREATED'; session: Session }
  | { type: 'APPEND_USER'; message: ChatMessageData }
  | { type: 'APPEND_ASSISTANT'; message: ChatMessageData }
  | { type: 'UPDATE_LAST_ASSISTANT'; content: string }
  | { type: 'CLEAR_MESSAGES' }
  | { type: 'RESET' }

const initialState: AssistantState = {
  session: null,
  messages: [],
  isLoading: true,
  error: null,
}

const reducer = (state: AssistantState, action: AssistantAction): AssistantState => {
  switch (action.type) {
    case 'INIT_START':
      return { ...state, isLoading: true, error: null }
    case 'INIT_SESSION':
      return { session: action.session, messages: action.messages, isLoading: false, error: null }
    case 'INIT_EMPTY':
      return { session: null, messages: [], isLoading: false, error: null }
    case 'INIT_ERROR':
      return { ...state, isLoading: false, error: action.error }
    case 'SESSION_CREATED':
      return { ...state, session: action.session }
    case 'APPEND_USER':
      return { ...state, messages: [...state.messages, action.message] }
    case 'APPEND_ASSISTANT':
      return { ...state, messages: [...state.messages, action.message] }
    case 'UPDATE_LAST_ASSISTANT': {
      const msgs = [...state.messages]
      const last = msgs[msgs.length - 1]
      if (last?.role === 'assistant') {
        msgs[msgs.length - 1] = { ...last, content: action.content }
      }
      return { ...state, messages: msgs }
    }
    case 'CLEAR_MESSAGES':
      return { ...state, messages: [] }
    case 'RESET':
      return initialState
    default:
      return state
  }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

const isCancelled = (ref: React.RefObject<boolean>): boolean => ref.current

const mapHistory = (history: ChatMessage[]): ChatMessageData[] =>
  history.map((m) => ({
    id: m.id,
    role: m.role,
    content: m.content,
  }))

// ---------------------------------------------------------------------------
// Hook
// ---------------------------------------------------------------------------

type UseAssistantSessionReturn = {
  messages: ChatMessageData[]
  isLoading: boolean
  error: string | null
  streaming: boolean
  sendMessage: (content: string) => void
  clearHistory: () => void
}

const useAssistantSession = (
  workflowId: string | null,
  stepId: string,
): UseAssistantSessionReturn => {
  const [state, dispatch] = useReducer(reducer, initialState)
  const { send, abort, streaming } = useSendSessionMessage()
  const contentRef = useRef('')
  const sessionRef = useRef<Session | null>(null)

  // Keep sessionRef in sync via effect (not during render)
  useEffect(() => {
    sessionRef.current = state.session
  }, [state.session])

  // Cancellation ref — set to true on cleanup so stale async work is ignored
  const cancelledRef = useRef<boolean>(false)

  // On mount / stepId change: try to find existing session
  useEffect(() => {
    cancelledRef.current = false

    if (!workflowId) {
      dispatch({ type: 'INIT_EMPTY' })
      return
    }

    dispatch({ type: 'INIT_START' })

    const init = async () => {
      try {
        const session = await api.workflows.getStepSession(workflowId, stepId)
        if (isCancelled(cancelledRef)) return

        const history = await api.sessions.getHistory(session.id)
        if (isCancelled(cancelledRef)) return

        dispatch({ type: 'INIT_SESSION', session, messages: mapHistory(history) })
      } catch (e) {
        if (isCancelled(cancelledRef)) return
        // 404 means no session yet — that's expected
        const is404 = e instanceof Error && e.message.includes('404')
        if (is404) {
          dispatch({ type: 'INIT_EMPTY' })
        } else {
          dispatch({ type: 'INIT_ERROR', error: e instanceof Error ? e.message : 'Failed to load session' })
        }
      }
    }

    void init()
    return () => {
      cancelledRef.current = true
      abort()
      dispatch({ type: 'RESET' })
    }
  }, [workflowId, stepId, abort])

  const sendMessage = useCallback(
    (content: string) => {
      if (!workflowId) return

      const userMsg: ChatMessageData = {
        id: crypto.randomUUID(),
        role: 'user',
        content,
      }
      dispatch({ type: 'APPEND_USER', message: userMsg })

      contentRef.current = ''
      const assistantMsg: ChatMessageData = {
        id: crypto.randomUUID(),
        role: 'assistant',
        content: '',
      }
      dispatch({ type: 'APPEND_ASSISTANT', message: assistantMsg })

      const onEvent = (event: SSEEvent) => {
        if (event.event === 'token' || event.event === 'message' || event.event === 'content') {
          let text = event.data
          try {
            const parsed = JSON.parse(text) as unknown
            if (typeof parsed === 'string') {
              text = parsed
            }
          } catch {
            // raw text
          }
          contentRef.current += text
          dispatch({ type: 'UPDATE_LAST_ASSISTANT', content: contentRef.current })
        }
      }

      const doSend = async () => {
        try {
          let session = sessionRef.current
          if (!session) {
            session = await api.workflows.getOrCreateStepSession(workflowId, stepId)
            dispatch({ type: 'SESSION_CREATED', session })
          }
          await send(session.id, { message: content }, onEvent)
        } catch (e) {
          dispatch({
            type: 'UPDATE_LAST_ASSISTANT',
            content: `Error: ${e instanceof Error ? e.message : 'Failed to send message'}`,
          })
        }
      }

      void doSend()
    },
    [workflowId, stepId, send],
  )

  const clearHistory = useCallback(() => {
    if (!workflowId || !state.session) return

    const capturedSession = state.session
    dispatch({ type: 'CLEAR_MESSAGES' })

    const doClear = async () => {
      try {
        await api.workflows.clearStepMessages(workflowId, stepId)
      } catch {
        // Reload history on failure
        try {
          const history = await api.sessions.getHistory(capturedSession.id)
          dispatch({ type: 'INIT_SESSION', session: capturedSession, messages: mapHistory(history) })
        } catch {
          // best effort
        }
      }
    }

    void doClear()
  }, [workflowId, stepId, state.session])

  return {
    messages: state.messages,
    isLoading: state.isLoading,
    error: state.error,
    streaming,
    sendMessage,
    clearHistory,
  }
}

export { useAssistantSession }
export type { UseAssistantSessionReturn }
