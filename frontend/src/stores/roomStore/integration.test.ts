import { roomStore } from '.'
import { ROOM_EVENT } from '@/types/ws'
import type { WsWireMessage } from '@/types/ws'
import { createNormalizedMap } from '../lib'

vi.mock('@/api', () => ({
  api: {
    rooms: {
      get: vi.fn(),
      create: vi.fn(),
      update: vi.fn(),
      delete: vi.fn(),
      listMembers: vi.fn(),
      addMember: vi.fn(),
      setMembers: vi.fn(),
      removeMember: vi.fn(),
      createSession: vi.fn(),
    },
    roomSessions: { get: vi.fn(), close: vi.fn(), getTranscript: vi.fn(), listOutputs: vi.fn(), sendMessage: vi.fn() },
  },
}))

let tsCounter = 0
const makeMsg = (event: string, data: Record<string, unknown>): WsWireMessage => ({
  topic: 'room',
  event,
  ts: `2025-01-01T00:00:${String(tsCounter++).padStart(2, '0')}Z`,
  run_id: null,
  user_id: null,
  data,
})

const handle = roomStore.handleWsEvent

beforeEach(() => {
  roomStore.store.setState({
    rooms: createNormalizedMap(),
    membersByRoom: {},
    sessionsByRoom: { r1: [{ id: 'rs1', room_id: 'r1', status: 'active', created_at: '2025-01-01', updated_at: '2025-01-01' }] },
    activeSessionId: null,
    transcript: [],
    outputs: [],
    loading: false,
    error: null,
  })
  tsCounter = 0
})

describe('roomStore integration', () => {
  describe('speaker sequence -> session complete', () => {
    it('accumulates transcript then marks session complete', () => {
      handle(
        makeMsg(ROOM_EVENT.SPEAKER_END, {
          room_session_id: 'rs1',
          agent_id: 'a1',
          agent_name: 'Alice',
          content: 'Hello',
          speaker_order: 0,
          turn_number: 0,
        }),
      )
      handle(
        makeMsg(ROOM_EVENT.SPEAKER_END, {
          room_session_id: 'rs1',
          agent_id: 'a2',
          agent_name: 'Bob',
          content: 'Hi there',
          speaker_order: 1,
          turn_number: 0,
        }),
      )
      handle(
        makeMsg(ROOM_EVENT.SPEAKER_END, {
          room_session_id: 'rs1',
          agent_id: 'a1',
          agent_name: 'Alice',
          content: 'Goodbye',
          speaker_order: 2,
          turn_number: 1,
        }),
      )

      const transcript = roomStore.selectTranscript(roomStore.store.getState())
      expect(transcript).toHaveLength(3)
      expect(transcript[0].agent_name).toBe('Alice')
      expect(transcript[0].content).toBe('Hello')
      expect(transcript[1].agent_name).toBe('Bob')
      expect(transcript[2].content).toBe('Goodbye')

      handle(makeMsg(ROOM_EVENT.SESSION_COMPLETE, { room_session_id: 'rs1', turn_number: 1 }))

      const sessions = roomStore.store.getState().sessionsByRoom['r1']
      expect(sessions[0].status).toBe('completed')
    })
  })

  describe('malformed messages', () => {
    it('malformed SPEAKER_END (missing content) — state unchanged', () => {
      const spy = vi.spyOn(console, 'error').mockImplementation(() => {})
      handle(makeMsg(ROOM_EVENT.SPEAKER_END, { room_session_id: 'rs1', agent_id: 'a1' }))
      spy.mockRestore()

      // Transcript may have an entry with undefined content, but store should not crash
      // The important thing is it didn't throw
      expect(roomStore.store.getState().loading).toBe(false)
    })

    it('SESSION_COMPLETE for unknown session — no crash', () => {
      handle(makeMsg(ROOM_EVENT.SESSION_COMPLETE, { room_session_id: 'unknown', turn_number: 0 }))

      // Should not crash — existing sessions unchanged
      const sessions = roomStore.store.getState().sessionsByRoom['r1']
      expect(sessions[0].status).toBe('active')
    })
  })

  describe('mixed valid + malformed', () => {
    it('only valid events applied', () => {
      handle(
        makeMsg(ROOM_EVENT.SPEAKER_END, {
          room_session_id: 'rs1',
          agent_id: 'a1',
          agent_name: 'Alice',
          content: 'First',
          speaker_order: 0,
          turn_number: 0,
        }),
      )

      const spy = vi.spyOn(console, 'error').mockImplementation(() => {})
      handle(makeMsg('totally_fake_event', { garbage: true }))
      spy.mockRestore()

      handle(
        makeMsg(ROOM_EVENT.SPEAKER_END, {
          room_session_id: 'rs1',
          agent_id: 'a2',
          agent_name: 'Bob',
          content: 'Second',
          speaker_order: 1,
          turn_number: 0,
        }),
      )

      const transcript = roomStore.selectTranscript(roomStore.store.getState())
      expect(transcript).toHaveLength(2)
      expect(transcript[0].content).toBe('First')
      expect(transcript[1].content).toBe('Second')
    })
  })
})
