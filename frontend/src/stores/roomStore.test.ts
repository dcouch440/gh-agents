import { roomStore } from './roomStore'
import { nmSize, nmGet, createNormalizedMap } from './lib'
import type { Room, RoomMember, RoomSession, RoomTranscriptEntry, RoomOutput } from '@/types/room'

const {
  mockRoomGet,
  mockRoomCreate,
  mockRoomUpdate,
  mockRoomDelete,
  mockListMembers,
  mockAddMember,
  mockSetMembers,
  mockRemoveMember,
  mockCreateSession,
  mockSessionGet,
  mockSendMessage,
  mockGetTranscript,
  mockCloseSession,
  mockListOutputs,
} = vi.hoisted(() => ({
  mockRoomGet: vi.fn(),
  mockRoomCreate: vi.fn(),
  mockRoomUpdate: vi.fn(),
  mockRoomDelete: vi.fn(),
  mockListMembers: vi.fn(),
  mockAddMember: vi.fn(),
  mockSetMembers: vi.fn(),
  mockRemoveMember: vi.fn(),
  mockCreateSession: vi.fn(),
  mockSessionGet: vi.fn(),
  mockSendMessage: vi.fn(),
  mockGetTranscript: vi.fn(),
  mockCloseSession: vi.fn(),
  mockListOutputs: vi.fn(),
}))

vi.mock('@/api', () => ({
  api: {
    rooms: {
      get: mockRoomGet,
      create: mockRoomCreate,
      update: mockRoomUpdate,
      delete: mockRoomDelete,
      listMembers: mockListMembers,
      addMember: mockAddMember,
      setMembers: mockSetMembers,
      removeMember: mockRemoveMember,
      createSession: mockCreateSession,
    },
    roomSessions: {
      get: mockSessionGet,
      sendMessage: mockSendMessage,
      getTranscript: mockGetTranscript,
      close: mockCloseSession,
      listOutputs: mockListOutputs,
    },
  },
}))

const room1: Room = {
  id: 'r1',
  user_id: 'u1',
  collection_id: null,
  name: 'Room 1',
  gatekeeper_enabled: false,
  gatekeeper_model_id: 'claude-haiku-4-20250414',
  max_speakers_per_turn: 4,
  max_turns: 20,
  tools_enabled: false,
  created_at: '2024-01-01T00:00:00Z',
  updated_at: '2024-01-01T00:00:00Z',
}

const member1: RoomMember = {
  room_id: 'r1',
  agent_id: 'a1',
  display_name: 'Agent One',
  role_description: 'Lead reviewer',
  display_order: 0,
}

const session1: RoomSession = {
  id: 's1',
  room_id: 'r1',
  run_id: null,
  status: 'active',
  current_turn: 1,
  transcript_summary: null,
  started_at: '2024-01-01T00:00:00Z',
  completed_at: null,
}

const transcript1: RoomTranscriptEntry = {
  agent_name: 'Agent One',
  role_description: 'Lead reviewer',
  content: 'Hello, world',
  speaker_order: 0,
  created_at: '2024-01-01T00:00:00Z',
}

const output1: RoomOutput = {
  id: 'o1',
  agent_id: 'a1',
  speaker_order: 0,
  turn_number: 1,
  output_name: 'analysis',
  structured_output: { result: 'pass' },
  raw_output: 'pass',
}

beforeEach(() => {
  vi.clearAllMocks()
  roomStore.store.setState({
    rooms: createNormalizedMap(),
    membersByRoom: {},
    sessionsByRoom: {},
    activeSessionId: null,
    transcript: [],
    outputs: [],
    loading: false,
    error: null,
  })
})

describe('roomStore', () => {
  describe('Room CRUD', () => {
    it('fetchOne upserts room', async () => {
      mockRoomGet.mockResolvedValue(room1)

      const result = await roomStore.fetchOne('r1')

      expect(result).toEqual(room1)
      expect(nmGet(roomStore.store.getState().rooms, 'r1')).toEqual(room1)
    })

    it('create adds room to store', async () => {
      mockRoomCreate.mockResolvedValue(room1)

      const result = await roomStore.create({ name: 'Room 1' })

      expect(result).toEqual(room1)
      expect(nmGet(roomStore.store.getState().rooms, 'r1')).toEqual(room1)
    })

    it('update replaces room in store', async () => {
      mockRoomGet.mockResolvedValue(room1)
      await roomStore.fetchOne('r1')

      const updated = { ...room1, name: 'Updated Room' }
      mockRoomUpdate.mockResolvedValue(updated)

      const result = await roomStore.update('r1', { name: 'Updated Room' })

      expect(result.name).toBe('Updated Room')
      expect(nmGet(roomStore.store.getState().rooms, 'r1')?.name).toBe('Updated Room')
    })

    it('remove optimistically deletes then calls API', async () => {
      mockRoomGet.mockResolvedValue(room1)
      mockRoomDelete.mockResolvedValue(undefined)
      await roomStore.fetchOne('r1')

      await roomStore.remove('r1')

      expect(nmSize(roomStore.store.getState().rooms)).toBe(0)
    })

    it('remove rolls back on API failure', async () => {
      mockRoomGet.mockResolvedValue(room1)
      await roomStore.fetchOne('r1')

      mockRoomDelete.mockRejectedValue(new Error('Server error'))

      await expect(roomStore.remove('r1')).rejects.toThrow('Server error')
      expect(nmSize(roomStore.store.getState().rooms)).toBe(1)
    })
  })

  describe('members', () => {
    it('fetchMembers stores members by room', async () => {
      mockListMembers.mockResolvedValue([member1])

      const result = await roomStore.fetchMembers('r1')

      expect(result).toEqual([member1])
      expect(roomStore.store.getState().membersByRoom['r1']).toEqual([member1])
    })

    it('addMember calls API then re-fetches', async () => {
      mockAddMember.mockResolvedValue(undefined)
      mockListMembers.mockResolvedValue([member1])

      await roomStore.addMember('r1', { agent_id: 'a1', role_description: 'Lead reviewer' })

      expect(mockAddMember).toHaveBeenCalledWith('r1', { agent_id: 'a1', role_description: 'Lead reviewer' })
      expect(mockListMembers).toHaveBeenCalledWith('r1')
    })

    it('setMembers calls API then re-fetches', async () => {
      mockSetMembers.mockResolvedValue(undefined)
      mockListMembers.mockResolvedValue([member1])

      await roomStore.setMembers('r1', { members: [{ agent_id: 'a1', role_description: 'Lead reviewer' }] })

      expect(mockSetMembers).toHaveBeenCalled()
      expect(mockListMembers).toHaveBeenCalledWith('r1')
    })

    it('removeMember calls API then re-fetches', async () => {
      mockRemoveMember.mockResolvedValue(undefined)
      mockListMembers.mockResolvedValue([])

      await roomStore.removeMember('r1', 'a1')

      expect(mockRemoveMember).toHaveBeenCalledWith('r1', 'a1')
      expect(mockListMembers).toHaveBeenCalledWith('r1')
    })
  })

  describe('sessions', () => {
    it('createSession appends session and sets activeSessionId', async () => {
      mockCreateSession.mockResolvedValue(session1)

      const result = await roomStore.createSession('r1')

      expect(result).toEqual(session1)
      expect(roomStore.store.getState().sessionsByRoom['r1']).toEqual([session1])
      expect(roomStore.store.getState().activeSessionId).toBe('s1')
    })

    it('fetchSession updates existing session in sessionsByRoom', async () => {
      mockCreateSession.mockResolvedValue(session1)
      await roomStore.createSession('r1')

      const updatedSession = { ...session1, current_turn: 3 }
      mockSessionGet.mockResolvedValue(updatedSession)

      const result = await roomStore.fetchSession('s1')

      expect(result.current_turn).toBe(3)
      expect(roomStore.store.getState().sessionsByRoom['r1']?.[0]?.current_turn).toBe(3)
    })

    it('setActiveSession updates activeSessionId', () => {
      roomStore.setActiveSession('s2')
      expect(roomStore.store.getState().activeSessionId).toBe('s2')

      roomStore.setActiveSession(null)
      expect(roomStore.store.getState().activeSessionId).toBeNull()
    })

    it('closeSession updates session status', async () => {
      mockCreateSession.mockResolvedValue(session1)
      await roomStore.createSession('r1')

      const closed = { ...session1, status: 'completed', completed_at: '2024-01-01T01:00:00Z' }
      mockCloseSession.mockResolvedValue(closed)

      const result = await roomStore.closeSession('s1')

      expect(result.status).toBe('completed')
      expect(roomStore.store.getState().sessionsByRoom['r1']?.[0]?.status).toBe('completed')
    })
  })

  describe('transcript + outputs', () => {
    it('fetchTranscript stores transcript entries', async () => {
      mockGetTranscript.mockResolvedValue([transcript1])

      const result = await roomStore.fetchTranscript('s1')

      expect(result).toEqual([transcript1])
      expect(roomStore.store.getState().transcript).toEqual([transcript1])
    })

    it('fetchOutputs stores outputs', async () => {
      mockListOutputs.mockResolvedValue([output1])

      const result = await roomStore.fetchOutputs('s1')

      expect(result).toEqual([output1])
      expect(roomStore.store.getState().outputs).toEqual([output1])
    })

    it('sendMessage calls API', async () => {
      mockSendMessage.mockResolvedValue(undefined)

      await roomStore.sendMessage('s1', { content: 'Hello' })

      expect(mockSendMessage).toHaveBeenCalledWith('s1', { content: 'Hello' })
    })
  })

  describe('loadRoom', () => {
    it('loads room and members in parallel', async () => {
      mockRoomGet.mockResolvedValue(room1)
      mockListMembers.mockResolvedValue([member1])

      await roomStore.loadRoom('r1')

      const state = roomStore.store.getState()
      expect(nmGet(state.rooms, 'r1')).toEqual(room1)
      expect(state.membersByRoom['r1']).toEqual([member1])
      expect(state.loading).toBe(false)
    })

    it('sets error on failure', async () => {
      mockRoomGet.mockRejectedValue(new Error('Not found'))

      await roomStore.loadRoom('r1')

      const state = roomStore.store.getState()
      expect(state.error).toBe('Not found')
      expect(state.loading).toBe(false)
    })
  })

  describe('sync utilities', () => {
    it('upsert adds room without API call', () => {
      roomStore.upsert(room1)
      expect(nmGet(roomStore.store.getState().rooms, 'r1')).toEqual(room1)
    })

    it('removeById removes room without API call', () => {
      roomStore.upsert(room1)
      roomStore.removeById('r1')
      expect(nmGet(roomStore.store.getState().rooms, 'r1')).toBeUndefined()
    })

    it('appendTranscriptEntry appends to transcript', () => {
      roomStore.appendTranscriptEntry(transcript1)
      expect(roomStore.store.getState().transcript).toEqual([transcript1])

      const entry2 = { ...transcript1, content: 'Second message' }
      roomStore.appendTranscriptEntry(entry2)
      expect(roomStore.store.getState().transcript).toHaveLength(2)
    })
  })

  describe('selectors', () => {
    it('selectById returns undefined for missing room', () => {
      const result = roomStore.selectById('missing')(roomStore.store.getState())
      expect(result).toBeUndefined()
    })

    it('selectMembers returns empty array for unknown room', () => {
      const result = roomStore.selectMembers('unknown')(roomStore.store.getState())
      expect(result).toEqual([])
    })

    it('selectSessions returns empty array for unknown room', () => {
      const result = roomStore.selectSessions('unknown')(roomStore.store.getState())
      expect(result).toEqual([])
    })
  })

  describe('handleWsEvent', () => {
    it('ROOM_EVENT.SPEAKER_END appends transcript entry', () => {
      roomStore.handleWsEvent({
        topic: 'room',
        event: 'speaker_end',
        ts: '2025-06-01T00:00:00Z',
        run_id: null,
        user_id: null,
        data: {
          room_session_id: 's1',
          agent_id: 'a1',
          agent_name: 'Agent One',
          content: 'Hello from WS',
          speaker_order: 0,
          turn_number: 1,
        },
      })

      const transcript = roomStore.store.getState().transcript
      expect(transcript).toHaveLength(1)
      expect(transcript[0].agent_name).toBe('Agent One')
      expect(transcript[0].content).toBe('Hello from WS')
      expect(transcript[0].speaker_order).toBe(0)
      expect(transcript[0].created_at).toBe('2025-06-01T00:00:00Z')
    })

    it('ROOM_EVENT.SESSION_COMPLETE marks session as completed', () => {
      roomStore.store.setState({
        sessionsByRoom: { r1: [session1] },
      })

      roomStore.handleWsEvent({
        topic: 'room',
        event: 'session_complete',
        ts: '2025-06-01T00:00:00Z',
        run_id: null,
        user_id: null,
        data: { room_session_id: 's1', turn_number: 5 },
      })

      const sessions = roomStore.store.getState().sessionsByRoom['r1']
      expect(sessions?.[0]?.status).toBe('completed')
    })

    it('ROOM_EVENT.SESSION_COMPLETE does not affect unrelated sessions', () => {
      roomStore.store.setState({
        sessionsByRoom: { r1: [session1] },
      })

      roomStore.handleWsEvent({
        topic: 'room',
        event: 'session_complete',
        ts: '2025-06-01T00:00:00Z',
        run_id: null,
        user_id: null,
        data: { room_session_id: 'unknown-session', turn_number: 5 },
      })

      const sessions = roomStore.store.getState().sessionsByRoom['r1']
      expect(sessions?.[0]?.status).toBe('active')
    })
  })
})
