import { api } from '@/api'
import type { RoomMember, AddRoomMemberRequest, SetRoomMembersRequest } from '@/types/room'
import { store } from './_store'

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

export { fetchMembers, addMember, setMembers, removeMember }
