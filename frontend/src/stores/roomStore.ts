// ============================================================================
// roomStore — Hand-written store for rooms + members + sessions + transcript
// ============================================================================

import { createStore, createNormalizedMap, nmSet, nmDelete, toArray, nmGet } from './lib'
import type { NormalizedMap } from './lib'
import { api } from '@/api'
import type {
  Room,
  RoomMember,
  RoomSession,
  RoomTranscriptEntry,
  RoomOutput,
  CreateRoomRequest,
  UpdateRoomRequest,
  AddRoomMemberRequest,
  SetRoomMembersRequest,
  RoomMessageRequest,
} from '@/types/room'
import { ROOM_EVENT } from '@/types/ws'
import type { WsWireMessage } from '@/types/ws'

// ── State ────────────────────────────────────────────────────────────────────

type RoomState = {
  rooms: NormalizedMap<Room>
  membersByRoom: Record<string, RoomMember[]>
  sessionsByRoom: Record<string, RoomSession[]>
  activeSessionId: string | null
  transcript: RoomTranscriptEntry[]
  outputs: RoomOutput[]
  loading: boolean
  error: string | null
}

// ── Store ────────────────────────────────────────────────────────────────────

const store = createStore<RoomState>(() => ({
  rooms: createNormalizedMap<Room>(),
  membersByRoom: {},
  sessionsByRoom: {},
  activeSessionId: null,
  transcript: [],
  outputs: [],
  loading: false,
  error: null,
}))

// ── Helpers ──────────────────────────────────────────────────────────────────

const extractError = (e: unknown): string =>
  e instanceof Error ? e.message : 'rooms: unknown error'

const EMPTY_MEMBERS: RoomMember[] = []
const EMPTY_SESSIONS: RoomSession[] = []

// ── Selectors ────────────────────────────────────────────────────────────────

const selectAll = (s: RoomState): Room[] => toArray(s.rooms)

const selectById = (id: string) => (s: RoomState): Room | undefined =>
  nmGet(s.rooms, id)

const selectMembers = (roomId: string) => (s: RoomState): RoomMember[] =>
  s.membersByRoom[roomId] ?? EMPTY_MEMBERS

const selectSessions = (roomId: string) => (s: RoomState): RoomSession[] =>
  s.sessionsByRoom[roomId] ?? EMPTY_SESSIONS

const selectActiveSessionId = (s: RoomState): string | null => s.activeSessionId

const selectTranscript = (s: RoomState): RoomTranscriptEntry[] => s.transcript

const selectOutputs = (s: RoomState): RoomOutput[] => s.outputs

const selectLoading = (s: RoomState): boolean => s.loading

const selectError = (s: RoomState): string | null => s.error

// ── Room CRUD ────────────────────────────────────────────────────────────────

const fetchOne = async (id: string): Promise<Room> => {
  const room = await api.rooms.get(id)
  store.setState((s) => ({ rooms: nmSet(s.rooms, room.id, room) }))
  return room
}

const create = async (body: CreateRoomRequest): Promise<Room> => {
  const room = await api.rooms.create(body)
  store.setState((s) => ({ rooms: nmSet(s.rooms, room.id, room) }))
  return room
}

const update = async (id: string, body: UpdateRoomRequest): Promise<Room> => {
  const room = await api.rooms.update(id, body)
  store.setState((s) => ({ rooms: nmSet(s.rooms, room.id, room) }))
  return room
}

const remove = async (id: string): Promise<void> => {
  const prev = store.getState()
  store.setState((s) => ({ rooms: nmDelete(s.rooms, id) }))
  try {
    await api.rooms.delete(id)
  } catch (e) {
    store.setState({ rooms: prev.rooms, error: extractError(e) })
    throw e
  }
}

// ── Members ──────────────────────────────────────────────────────────────────

const fetchMembers = async (roomId: string): Promise<RoomMember[]> => {
  const members = await api.rooms.listMembers(roomId)
  store.setState((s) => ({
    membersByRoom: { ...s.membersByRoom, [roomId]: members },
  }))
  return members
}

const addMember = async (roomId: string, body: AddRoomMemberRequest): Promise<void> => {
  await api.rooms.addMember(roomId, body)
  await fetchMembers(roomId)
}

const setMembers = async (roomId: string, body: SetRoomMembersRequest): Promise<void> => {
  await api.rooms.setMembers(roomId, body)
  await fetchMembers(roomId)
}

const removeMember = async (roomId: string, agentId: string): Promise<void> => {
  await api.rooms.removeMember(roomId, agentId)
  await fetchMembers(roomId)
}

// ── Sessions ─────────────────────────────────────────────────────────────────

const createSession = async (roomId: string): Promise<RoomSession> => {
  const session = await api.rooms.createSession(roomId)
  store.setState((s) => ({
    sessionsByRoom: {
      ...s.sessionsByRoom,
      [roomId]: [...(s.sessionsByRoom[roomId] ?? []), session],
    },
    activeSessionId: session.id,
  }))
  return session
}

const fetchSession = async (sessionId: string): Promise<RoomSession> => {
  const session = await api.roomSessions.get(sessionId)
  store.setState((s) => ({
    sessionsByRoom: {
      ...s.sessionsByRoom,
      [session.room_id]: (s.sessionsByRoom[session.room_id] ?? []).map((rs) =>
        rs.id === sessionId ? session : rs,
      ),
    },
  }))
  return session
}

const setActiveSession = (sessionId: string | null): void => {
  store.setState({ activeSessionId: sessionId })
}

const closeSession = async (sessionId: string): Promise<RoomSession> => {
  const session = await api.roomSessions.close(sessionId)
  store.setState((s) => ({
    sessionsByRoom: {
      ...s.sessionsByRoom,
      [session.room_id]: (s.sessionsByRoom[session.room_id] ?? []).map((rs) =>
        rs.id === sessionId ? session : rs,
      ),
    },
  }))
  return session
}

// ── Transcript + Outputs ─────────────────────────────────────────────────────

const fetchTranscript = async (sessionId: string): Promise<RoomTranscriptEntry[]> => {
  const transcript = await api.roomSessions.getTranscript(sessionId)
  store.setState({ transcript })
  return transcript
}

const fetchOutputs = async (sessionId: string): Promise<RoomOutput[]> => {
  const outputs = await api.roomSessions.listOutputs(sessionId)
  store.setState({ outputs })
  return outputs
}

// ── Messaging ────────────────────────────────────────────────────────────────

const sendMessage = async (sessionId: string, body: RoomMessageRequest): Promise<void> => {
  await api.roomSessions.sendMessage(sessionId, body)
}

// ── Load Room (full context) ─────────────────────────────────────────────────

const loadRoom = async (id: string): Promise<void> => {
  store.setState({ loading: true, error: null })
  try {
    const [room, members] = await Promise.all([
      api.rooms.get(id),
      api.rooms.listMembers(id),
    ])
    store.setState((s) => ({
      rooms: nmSet(s.rooms, room.id, room),
      membersByRoom: { ...s.membersByRoom, [id]: members },
      loading: false,
    }))
  } catch (e) {
    store.setState({ loading: false, error: extractError(e) })
  }
}

// ── Sync / Utility ───────────────────────────────────────────────────────────

const upsert = (room: Room): void => {
  store.setState((s) => ({ rooms: nmSet(s.rooms, room.id, room) }))
}

const removeById = (id: string): void => {
  store.setState((s) => ({ rooms: nmDelete(s.rooms, id) }))
}

const appendTranscriptEntry = (entry: RoomTranscriptEntry): void => {
  store.setState((s) => ({ transcript: [...s.transcript, entry] }))
}

// ── WebSocket Handler ────────────────────────────────────────────────────────

const handleWsEvent = (msg: WsWireMessage): void => {
  try {
    const data = msg.data

    switch (msg.event) {
      case ROOM_EVENT.SPEAKER_END: {
        appendTranscriptEntry({
          agent_name: data.agent_name as string,
          role_description: '',
          content: data.content as string,
          speaker_order: data.speaker_order as number,
          created_at: msg.ts,
        })
        break
      }
      case ROOM_EVENT.SESSION_COMPLETE: {
        const sessionId = data.room_session_id as string
        store.setState((s) => {
          const updated: Record<string, RoomSession[]> = {}
          for (const [roomId, sessions] of Object.entries(s.sessionsByRoom)) {
            updated[roomId] = sessions.map((rs) =>
              rs.id === sessionId ? { ...rs, status: 'completed' } : rs,
            )
          }
          return { sessionsByRoom: updated }
        })
        break
      }
    }
  } catch (err) {
    console.error(`[roomStore] WS handler error on "${msg.event}":`, err)
  }
}

// ── Export ────────────────────────────────────────────────────────────────────

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

export type { RoomState }
