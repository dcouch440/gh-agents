import { api } from '@/api'
import type { RoomTranscriptEntry, RoomOutput, RoomMessageRequest } from '@/types/room'
import { store } from './_store'

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

const sendMessage = async (sessionId: string, body: RoomMessageRequest): Promise<void> => {
  await api.roomSessions.sendMessage(sessionId, body)
}

const appendTranscriptEntry = (entry: RoomTranscriptEntry): void => {
  store.setState((s) => ({ transcript: [...s.transcript, entry] }))
}

export { fetchTranscript, fetchOutputs, sendMessage, appendTranscriptEntry }
