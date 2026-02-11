import type { NormalizedMap } from '../lib'
import type { Room, RoomMember, RoomSession, RoomTranscriptEntry, RoomOutput } from '@/types/room'

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

export type { RoomState }
