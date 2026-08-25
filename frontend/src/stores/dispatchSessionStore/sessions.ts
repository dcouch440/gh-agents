import { api } from '@/api'
import type { ChatMessageData } from '@/components/chat'
import { Collections } from '@/utils/collections'
import type { ChatMessage } from '@/types'
import { store, emptySession, updateStep } from './_store'

const mapHistory = (history: readonly ChatMessage[]): ChatMessageData[] =>
  Collections.mapBy(history, (m) => ({
    id: m.id,
    role: m.role,
    content: m.content,
    source_type: m.source_type,
    error: m.error,
  }))

const initStep = (stepId: string): void => {
  updateStep(stepId, { ...emptySession, isLoading: true })
}

const initEmpty = (stepId: string): void => {
  updateStep(stepId, { ...emptySession, isLoading: false })
}

const setError = (stepId: string, error: string): void => {
  updateStep(stepId, { isLoading: false, error })
}

const resetStep = (stepId: string): void => {
  store.setState((s) => {
    const next = { ...s.byStep }
    delete next[stepId]
    return { byStep: next }
  })
}

const loadSession = async (stepId: string): Promise<void> => {
  initStep(stepId)
  try {
    const { session_id } = await api.dispatch.session(stepId)
    const history = await api.sessions.getHistory(session_id)
    updateStep(stepId, {
      sessionId: session_id,
      messages: mapHistory(history),
      isLoading: false,
      error: null,
    })
  } catch (e) {
    const is404 = e instanceof Error && e.message.includes('404')
    if (is404) {
      initEmpty(stepId)
    } else {
      setError(stepId, e instanceof Error ? e.message : 'Failed to load session')
    }
  }
}

const appendDispatchResult = (stepId: string, instruction: string, summary: string): void => {
  const current = store.getState().byStep[stepId]
  if (!current) return

  const now = Date.now().toString()
  const newMessages: ChatMessageData[] = [
    ...current.messages,
    { id: `dispatch-user-${now}`, role: 'user', content: instruction },
    { id: `dispatch-assistant-${now}`, role: 'assistant', content: summary },
  ]

  updateStep(stepId, { messages: newMessages })
}

export {
  initStep,
  initEmpty,
  setError,
  resetStep,
  loadSession,
  appendDispatchResult,
  mapHistory,
}
