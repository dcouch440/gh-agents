import { store } from './_store'
import {
  selectSession,
  selectMessages,
  selectSegments,
  selectPanel,
  selectLoading,
  selectError,
} from './selectors'
import {
  initStep,
  initEmpty,
  setSession,
  setSessionCreated,
  setError,
  resetStep,
  loadSession,
  clearMessages,
} from './sessions'
import {
  appendMessage,
  streamToken,
  addTool,
  completeTool,
  addDoc,
  setPanel,
  dismissPanel,
  finalizeStream,
  handleStreamError,
  parseTokenText,
} from './streaming'

export const assistantSessionStore = {
  store,
  selectSession,
  selectMessages,
  selectSegments,
  selectPanel,
  selectLoading,
  selectError,
  initStep,
  initEmpty,
  setSession,
  setSessionCreated,
  setError,
  resetStep,
  loadSession,
  clearMessages,
  appendMessage,
  streamToken,
  addTool,
  completeTool,
  addDoc,
  setPanel,
  dismissPanel,
  finalizeStream,
  handleStreamError,
  parseTokenText,
}

export type { PanelState, StepSession, AssistantSessionState } from './types'
