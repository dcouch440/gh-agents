import { store } from './_store'
import {
  selectAll,
  selectById,
  selectMembers,
  selectSessions,
  selectActiveSessionId,
  selectTranscript,
  selectOutputs,
  selectLoading,
  selectError,
} from './selectors'
import { fetchOne, create, update, remove, loadRoom, upsert, removeById } from './rooms'
import { fetchMembers, addMember, setMembers, removeMember } from './members'
import { createSession, fetchSession, setActiveSession, closeSession } from './sessions'
import { fetchTranscript, fetchOutputs, sendMessage, appendTranscriptEntry } from './transcript'
import { handleWsEvent } from './wsHandler'

export const roomStore = {
  store,
  selectAll,
  selectById,
  selectMembers,
  selectSessions,
  selectActiveSessionId,
  selectTranscript,
  selectOutputs,
  selectLoading,
  selectError,
  fetchOne,
  create,
  update,
  remove,
  fetchMembers,
  addMember,
  setMembers,
  removeMember,
  createSession,
  fetchSession,
  setActiveSession,
  closeSession,
  fetchTranscript,
  fetchOutputs,
  sendMessage,
  loadRoom,
  upsert,
  removeById,
  appendTranscriptEntry,
  handleWsEvent,
}

export type { RoomState } from './types'
