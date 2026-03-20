import { toArray, nmGet, memoFactory } from '../lib'
import type { Room, RoomMember, RoomSession, RoomTranscriptEntry, RoomOutput } from '@/types/room'
import type { RoomState } from './types'
import { EMPTY_MEMBERS, EMPTY_SESSIONS } from './_store'

const selectAll = (s: RoomState): Room[] => toArray(s.rooms)

const selectById = memoFactory(
  (id: string) =>
  (s: RoomState): Room | undefined =>
    nmGet(s.rooms, id),
)

const selectMembers = memoFactory(
  (roomId: string) =>
  (s: RoomState): RoomMember[] =>
    s.membersByRoom[roomId] ?? EMPTY_MEMBERS,
)

const selectSessions = memoFactory(
  (roomId: string) =>
  (s: RoomState): RoomSession[] =>
    s.sessionsByRoom[roomId] ?? EMPTY_SESSIONS,
)

const selectActiveSessionId = (s: RoomState): string | null => s.activeSessionId

const selectTranscript = (s: RoomState): RoomTranscriptEntry[] => s.transcript

const selectOutputs = (s: RoomState): RoomOutput[] => s.outputs

const selectLoading = (s: RoomState): boolean => s.loading

const selectError = (s: RoomState): string | null => s.error

export {
  selectAll,
  selectById,
  selectMembers,
  selectSessions,
  selectActiveSessionId,
  selectTranscript,
  selectOutputs,
  selectLoading,
  selectError,
}
