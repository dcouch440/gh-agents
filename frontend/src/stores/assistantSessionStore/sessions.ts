import { api } from '@/api'
import type { ChatMessageData } from '@/components/chat'
import type { Session } from '@/types'
import { store, emptySession, updateStep } from './_store'
import { mapHistory } from './streaming'

const initStep = (stepId: string): void => {
  updateStep(stepId, { ...emptySession, isLoading: true })
}

const initEmpty = (stepId: string): void => {
  updateStep(stepId, { ...emptySession, isLoading: false })
}

const setSession = (stepId: string, session: Session, messages: ChatMessageData[]): void => {
  updateStep(stepId, {
    session,
    messages,
    streamingSegments: [],
    isLoading: false,
    error: null,
  })
}

const setSessionCreated = (stepId: string, session: Session): void => {
  updateStep(stepId, { session })
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

const loadSession = async (workflowId: string, stepId: string): Promise<void> => {
  initStep(stepId)
  try {
    const session = await api.workflows.getStepSession(workflowId, stepId)
    const history = await api.sessions.getHistory(session.id)
    setSession(stepId, session, mapHistory(history))
  } catch (e) {
    const is404 = e instanceof Error && e.message.includes('404')
    if (is404) {
      initEmpty(stepId)
    } else {
      setError(stepId, e instanceof Error ? e.message : 'Failed to load session')
    }
  }
}

const clearMessages = async (workflowId: string, stepId: string): Promise<void> => {
  const current = store.getState().byStep[stepId]
  if (!current?.session) return

  const capturedSession = current.session
  updateStep(stepId, { messages: [], streamingSegments: [] })

  try {
    await api.workflows.clearStepMessages(workflowId, stepId)
  } catch {
    try {
      const history = await api.sessions.getHistory(capturedSession.id)
      setSession(stepId, capturedSession, mapHistory(history))
    } catch {
      // best effort
    }
  }
}

export {
  initStep,
  initEmpty,
  setSession,
  setSessionCreated,
  setError,
  resetStep,
  loadSession,
  clearMessages,
}
