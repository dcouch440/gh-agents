import { nmSet, nmDelete, extractError } from '../lib'
import { api } from '@/api'
import type { Room, CreateRoomRequest, UpdateRoomRequest } from '@/types/room'
import { store } from './_store'

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
    store.setState({ rooms: prev.rooms, error: extractError('rooms', e) })
    throw e
  }
}

const loadRoom = async (id: string): Promise<void> => {
  store.setState({ loading: true, error: null })
  try {
    const [room, members] = await Promise.all([api.rooms.get(id), api.rooms.listMembers(id)])
    store.setState((s) => ({
      rooms: nmSet(s.rooms, room.id, room),
      membersByRoom: { ...s.membersByRoom, [id]: members },
      loading: false,
    }))
  } catch (e) {
    store.setState({ loading: false, error: extractError('rooms', e) })
  }
}

const upsert = (room: Room): void => {
  store.setState((s) => ({ rooms: nmSet(s.rooms, room.id, room) }))
}

const removeById = (id: string): void => {
  store.setState((s) => ({ rooms: nmDelete(s.rooms, id) }))
}

export { fetchOne, create, update, remove, loadRoom, upsert, removeById }
