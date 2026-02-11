import { createStore, createNormalizedMap } from '../lib'
import type { Room, RoomMember, RoomSession } from '@/types/room'
import type { RoomState } from './types'

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

const EMPTY_MEMBERS: RoomMember[] = []
const EMPTY_SESSIONS: RoomSession[] = []

export { store, EMPTY_MEMBERS, EMPTY_SESSIONS }
