import { ROOM_EVENT } from '@/types/ws'
import type { WsWireMessage } from '@/types/ws'
import { store } from './_store'
import { appendTranscriptEntry } from './transcript'

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
          for (const [roomId, sessions] of Object.entries(s.sessionsByRoom)) {
            const idx = sessions.findIndex((rs) => rs.id === sessionId)
            if (idx !== -1) {
              const updated = sessions.slice()
              updated[idx] = { ...sessions[idx], status: 'completed' }
              return { sessionsByRoom: { ...s.sessionsByRoom, [roomId]: updated } }
            }
          }
          return s
        })
        break
      }
    }
  } catch (err) {
    console.error(`[roomStore] WS handler error on "${msg.event}":`, err)
  }
}

export { handleWsEvent }
