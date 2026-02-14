import { useReducer, useEffect, useCallback, useRef } from 'react'
import { api, createSSEStream } from '@/api'
import type { SSEEvent } from '@/api'
import { API } from '@/constants'
import { useSendSessionMessage } from './useChatMutations'
import type { ChatMessageData } from '@/components/chat'
import type { Session, ChatMessage, MessageSegment } from '@/types'

// ---------------------------------------------------------------------------
// State
// ---------------------------------------------------------------------------

type PanelState = {
  content: string
  submitLabel: string
}

type AssistantState = {
  session: Session | null
  messages: ChatMessageData[]
  streamingSegments: MessageSegment[]
  isLoading: boolean
  error: string | null
  activePanel: PanelState | null
}

type AssistantAction =
  | { type: 'INIT_START' }
  | { type: 'INIT_SESSION'; session: Session; messages: ChatMessageData[] }
  | { type: 'INIT_EMPTY' }
  | { type: 'INIT_ERROR'; error: string }
  | { type: 'SESSION_CREATED'; session: Session }
  | { type: 'APPEND_USER'; message: ChatMessageData }
  | { type: 'APPEND_ASSISTANT'; message: ChatMessageData }
  | { type: 'STREAM_TOKEN'; text: string }
  | { type: 'STREAM_TOOL_START'; toolId: string; toolName: string }
  | { type: 'STREAM_TOOL_END'; toolId: string }
  | { type: 'STREAM_DOC_UPDATE'; docId: string; title: string }
  | { type: 'STREAM_PANEL_RENDER'; content: string; submitLabel: string }
  | { type: 'STREAM_FINALIZE' }
  | { type: 'STREAM_ERROR'; error: string }
  | { type: 'PANEL_DISMISS' }
  | { type: 'CLEAR_MESSAGES' }
  | { type: 'RESET' }

const initialState: AssistantState = {
  session: null,
  messages: [],
  streamingSegments: [],
  isLoading: true,
  error: null,
  activePanel: null,
}

const reducer = (state: AssistantState, action: AssistantAction): AssistantState => {
  switch (action.type) {
    case 'INIT_START':
      return { ...state, isLoading: true, error: null }
    case 'INIT_SESSION':
      return { session: action.session, messages: action.messages, streamingSegments: [], isLoading: false, error: null, activePanel: null }
    case 'INIT_EMPTY':
      return { session: null, messages: [], streamingSegments: [], isLoading: false, error: null, activePanel: null }
    case 'INIT_ERROR':
      return { ...state, isLoading: false, error: action.error }
    case 'SESSION_CREATED':
      return { ...state, session: action.session }
    case 'APPEND_USER':
      return { ...state, messages: [...state.messages, action.message] }
    case 'APPEND_ASSISTANT':
      return { ...state, messages: [...state.messages, action.message] }

    case 'STREAM_TOKEN': {
      // Update segments: append to last text segment or create new one
      const segments = [...state.streamingSegments]
      const lastSeg = segments[segments.length - 1]
      if (lastSeg?.type === 'text') {
        segments[segments.length - 1] = { ...lastSeg, content: lastSeg.content + action.text }
      } else {
        segments.push({ type: 'text', content: action.text })
      }

      // Update last assistant message content (for scroll tracking + finalization)
      const msgs = [...state.messages]
      const last = msgs[msgs.length - 1]
      if (last?.role === 'assistant') {
        msgs[msgs.length - 1] = { ...last, content: last.content + action.text }
      }

      return { ...state, messages: msgs, streamingSegments: segments }
    }

    case 'STREAM_TOOL_START': {
      const segments: MessageSegment[] = [
        ...state.streamingSegments,
        { type: 'tool', toolId: action.toolId, toolName: action.toolName, status: 'running' },
      ]
      return { ...state, streamingSegments: segments }
    }

    case 'STREAM_TOOL_END': {
      const segments = state.streamingSegments.map((s) =>
        s.type === 'tool' && s.toolId === action.toolId ? { ...s, status: 'complete' as const } : s,
      )
      return { ...state, streamingSegments: segments }
    }

    case 'STREAM_DOC_UPDATE': {
      const segments: MessageSegment[] = [
        ...state.streamingSegments,
        { type: 'doc_update', docId: action.docId, title: action.title },
      ]
      return { ...state, streamingSegments: segments }
    }

    case 'STREAM_PANEL_RENDER':
      return { ...state, activePanel: { content: action.content, submitLabel: action.submitLabel } }

    case 'STREAM_FINALIZE':
      return { ...state, streamingSegments: [] }

    case 'PANEL_DISMISS':
      return { ...state, activePanel: null }

    case 'STREAM_ERROR': {
      const msgs = [...state.messages]
      const last = msgs[msgs.length - 1]
      if (last?.role === 'assistant') {
        msgs[msgs.length - 1] = { ...last, content: last.content || `Error: ${action.error}` }
      }
      return { ...state, messages: msgs, streamingSegments: [], error: action.error }
    }

    case 'CLEAR_MESSAGES':
      return { ...state, messages: [], streamingSegments: [], activePanel: null }
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

const parseTokenText = (data: string): string => {
  try {
    const parsed = JSON.parse(data) as unknown
    if (typeof parsed === 'string') return parsed
  } catch {
    // raw text
  }
  return data
}

// ---------------------------------------------------------------------------
// Hook
// ---------------------------------------------------------------------------

type UseAssistantSessionReturn = {
  messages: ChatMessageData[]
  streamingSegments: MessageSegment[]
  isLoading: boolean
  error: string | null
  streaming: boolean
  activePanel: PanelState | null
  sendMessage: (content: string) => void
  clearHistory: () => void
  dismissPanel: () => void
  submitPanelSelections: (selections: string) => void
}

const useAssistantSession = (
  workflowId: string | null,
  stepId: string,
): UseAssistantSessionReturn => {
  const [state, dispatch] = useReducer(reducer, initialState)
  const { send, abort, streaming } = useSendSessionMessage()
  const sessionRef = useRef<Session | null>(null)
  const receivedLengthRef = useRef(0)
  const retriedRef = useRef(false)
  const retryAbortRef = useRef<(() => void) | null>(null)

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
      retryAbortRef.current?.()
      retryAbortRef.current = null
      dispatch({ type: 'RESET' })
    }
  }, [workflowId, stepId, abort])

  const sendMessage = useCallback(
    (content: string) => {
      if (!workflowId) return

      // Reset retry tracking for new message
      receivedLengthRef.current = 0
      retriedRef.current = false
      retryAbortRef.current?.()
      retryAbortRef.current = null

      const userMsg: ChatMessageData = {
        id: crypto.randomUUID(),
        role: 'user',
        content,
      }
      dispatch({ type: 'APPEND_USER', message: userMsg })

      const assistantMsg: ChatMessageData = {
        id: crypto.randomUUID(),
        role: 'assistant',
        content: '',
      }
      dispatch({ type: 'APPEND_ASSISTANT', message: assistantMsg })

      const onEvent = (event: SSEEvent) => {
        switch (event.event) {
          case 'token':
          case 'message':
          case 'content': {
            const text = parseTokenText(event.data)
            receivedLengthRef.current += text.length
            dispatch({ type: 'STREAM_TOKEN', text })
            break
          }
          case 'tool_start': {
            const data = JSON.parse(event.data) as { name: string; id: string }
            dispatch({ type: 'STREAM_TOOL_START', toolId: data.id, toolName: data.name })
            break
          }
          case 'tool_end': {
            const data = JSON.parse(event.data) as { name: string; id: string }
            dispatch({ type: 'STREAM_TOOL_END', toolId: data.id })
            break
          }
          case 'doc_update': {
            const data = JSON.parse(event.data) as { doc_id: string; title: string }
            dispatch({ type: 'STREAM_DOC_UPDATE', docId: data.doc_id, title: data.title })
            break
          }
          case 'panel_render': {
            const data = JSON.parse(event.data) as { content: string; submit_label: string }
            dispatch({ type: 'STREAM_PANEL_RENDER', content: data.content, submitLabel: data.submit_label })
            break
          }
          case 'error': {
            dispatch({ type: 'STREAM_ERROR', error: event.data })
            break
          }
        }
      }

      const onDone = () => {
        dispatch({ type: 'STREAM_FINALIZE' })
      }

      const doSend = async () => {
        try {
          let session = sessionRef.current
          if (!session) {
            session = await api.workflows.getOrCreateStepSession(workflowId, stepId)
            dispatch({ type: 'SESSION_CREATED', session })
          }

          const messageId = await send(
            session.id,
            { message: content },
            onEvent,
            onDone,
            (error: Error) => {
              // SSE connection error — attempt one retry
              if (!retriedRef.current) {
                retriedRef.current = true
                const dedupeAfter = receivedLengthRef.current

                let replayedLength = 0
                const deduplicatingHandler = (evt: SSEEvent) => {
                  if (evt.event === 'token' || evt.event === 'message' || evt.event === 'content') {
                    const text = parseTokenText(evt.data)
                    replayedLength += text.length
                    if (replayedLength <= dedupeAfter) return
                    const overlap = dedupeAfter - (replayedLength - text.length)
                    const newText = overlap > 0 ? text.slice(overlap) : text
                    if (newText) {
                      receivedLengthRef.current += newText.length
                      dispatch({ type: 'STREAM_TOKEN', text: newText })
                    }
                  } else {
                    onEvent(evt)
                  }
                }

                retryAbortRef.current = createSSEStream(
                  API.SESSION_CHAT_STREAM(session.id, messageId),
                  {
                    onEvent: dedupeAfter > 0 ? deduplicatingHandler : onEvent,
                    onDone,
                    onError: () => {
                      dispatch({ type: 'STREAM_ERROR', error: 'Stream connection lost' })
                    },
                  },
                )
              } else {
                dispatch({ type: 'STREAM_ERROR', error: error.message })
              }
            },
          )
          // messageId is used in the onError closure above
          void messageId
        } catch (e) {
          dispatch({
            type: 'STREAM_ERROR',
            error: e instanceof Error ? e.message : 'Failed to send message',
          })
        }
      }

      void doSend()
    },
    [workflowId, stepId, send],
  )

  const dismissPanel = useCallback(() => {
    dispatch({ type: 'PANEL_DISMISS' })
  }, [])

  const submitPanelSelections = useCallback(
    (selections: string) => {
      dispatch({ type: 'PANEL_DISMISS' })
      sendMessage(selections)
    },
    [sendMessage],
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
    streamingSegments: state.streamingSegments,
    isLoading: state.isLoading,
    error: state.error,
    streaming,
    activePanel: state.activePanel,
    sendMessage,
    clearHistory,
    dismissPanel,
    submitPanelSelections,
  }
}

export { useAssistantSession, reducer, initialState }
export type { UseAssistantSessionReturn, AssistantState, AssistantAction, PanelState }
