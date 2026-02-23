import { store } from './_store'
import { selectMessages, selectLoading, selectError } from './selectors'
import {
  initStep,
  initEmpty,
  setError,
  resetStep,
  loadSession,
  appendDispatchResult,
} from './sessions'

export const dispatchSessionStore = {
  store,
  selectMessages,
  selectLoading,
  selectError,
  initStep,
  initEmpty,
  setError,
  resetStep,
  loadSession,
  appendDispatchResult,
}

export type { DispatchStepSession, DispatchSessionState } from './types'
