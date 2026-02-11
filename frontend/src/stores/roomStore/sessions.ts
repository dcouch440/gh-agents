import { api } from '@/api'
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
    const idx = sessions.findIndex((rs) => rs.id === sessionId)
    if (idx === -1) {
      return { sessionsByRoom: { ...s.sessionsByRoom, [session.room_id]: [...sessions, session] } }
    }
    const updated = sessions.slice()
    updated[idx] = session
    return { sessionsByRoom: { ...s.sessionsByRoom, [session.room_id]: updated } }
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
    const idx = sessions.findIndex((rs) => rs.id === sessionId)
    if (idx === -1) return s
    const updated = sessions.slice()
    updated[idx] = session
    return { sessionsByRoom: { ...s.sessionsByRoom, [session.room_id]: updated } }
  })
  return session
}

export { createSession, fetchSession, setActiveSession, closeSession }
