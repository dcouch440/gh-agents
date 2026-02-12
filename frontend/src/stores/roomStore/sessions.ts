import { api } from '@/api'
import { Collections } from '@/utils/collections'
import type { RoomSession } from '@/types/room'
import { store } from './_store'

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
  store.setState((s) => {
    const sessions = s.sessionsByRoom[session.room_id] ?? []
    const exists = sessions.some((rs) => rs.id === sessionId)
    if (!exists) {
      return { sessionsByRoom: { ...s.sessionsByRoom, [session.room_id]: [...sessions, session] } }
    }
    return {
      sessionsByRoom: {
        ...s.sessionsByRoom,
        [session.room_id]: Collections.mapBy(sessions, (rs) => (rs.id === sessionId ? session : rs)),
      },
    }
  })
  return session
}

const setActiveSession = (sessionId: string | null): void => {
  store.setState({ activeSessionId: sessionId })
}

const closeSession = async (sessionId: string): Promise<RoomSession> => {
  const session = await api.roomSessions.close(sessionId)
  store.setState((s) => {
    const sessions = s.sessionsByRoom[session.room_id] ?? []
    if (!sessions.some((rs) => rs.id === sessionId)) return s
    return {
      sessionsByRoom: {
        ...s.sessionsByRoom,
        [session.room_id]: Collections.mapBy(sessions, (rs) => (rs.id === sessionId ? session : rs)),
      },
    }
  })
  return session
}

export { createSession, fetchSession, setActiveSession, closeSession }
