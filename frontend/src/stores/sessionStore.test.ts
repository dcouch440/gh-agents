import { sessionStore } from './sessionStore'
import { nmSize, nmGet, nmSet, createNormalizedMap } from './lib'

const {
  mockList,
  mockGet,
  mockCreate,
  mockUpdate,
  mockDelete,
} = vi.hoisted(() => ({
  mockList: vi.fn(),
  mockGet: vi.fn(),
  mockCreate: vi.fn(),
  mockUpdate: vi.fn(),
  mockDelete: vi.fn(),
}))

vi.mock('@/api', () => ({
  api: {
    sessions: {
      list: mockList,
      get: mockGet,
      create: mockCreate,
      update: mockUpdate,
      delete: mockDelete,
    },
  },
}))

const session1 = { id: 's1', mode_id: 'home', agent_id: null, draft_config: null, title: 'Session 1', created_at: '2025-01-01T00:00:00Z', updated_at: '2025-01-01T00:00:00Z' }
const session2 = { id: 's2', mode_id: 'workshop', agent_id: 'a1', draft_config: null, title: 'Session 2', created_at: '2025-01-02T00:00:00Z', updated_at: '2025-01-02T00:00:00Z' }

beforeEach(() => {
  vi.clearAllMocks()
  sessionStore.store.setState({
    items: createNormalizedMap(),
    loading: false,
    error: null,
  })
})

describe('sessionStore', () => {
  describe('fetchAll', () => {
    it('populates items from api.sessions.list()', async () => {
      mockList.mockResolvedValue([session1, session2])
      await sessionStore.fetchAll()

      const s = sessionStore.store.getState()
      expect(nmSize(s.items)).toBe(2)
      expect(nmGet(s.items, 's1')).toEqual(session1)
      expect(s.loading).toBe(false)
      expect(s.error).toBeNull()
    })

    it('sets error on failure', async () => {
      mockList.mockRejectedValue(new Error('Network error'))
      await sessionStore.fetchAll()

      const s = sessionStore.store.getState()
      expect(s.error).toBe('Network error')
      expect(s.loading).toBe(false)
    })
  })

  describe('fetchOne', () => {
    it('upserts single session', async () => {
      mockGet.mockResolvedValue(session1)
      const result = await sessionStore.fetchOne('s1')

      expect(result).toEqual(session1)
      expect(nmGet(sessionStore.store.getState().items, 's1')).toEqual(session1)
    })
  })

  describe('create', () => {
    it('creates and upserts session', async () => {
      mockCreate.mockResolvedValue(session1)
      const result = await sessionStore.create({ mode_id: 'home' })

      expect(result).toEqual(session1)
      expect(nmGet(sessionStore.store.getState().items, 's1')).toEqual(session1)
    })
  })

  describe('update', () => {
    it('updates and upserts session', async () => {
      const updated = { ...session1, title: 'Updated' }
      mockUpdate.mockResolvedValue(updated)
      const result = await sessionStore.update('s1', { title: 'Updated' })

      expect(result).toEqual(updated)
      expect(nmGet(sessionStore.store.getState().items, 's1')?.title).toBe('Updated')
    })
  })

  describe('remove', () => {
    it('optimistically deletes and calls api', async () => {
      mockList.mockResolvedValue([session1])
      await sessionStore.fetchAll()
      expect(nmSize(sessionStore.store.getState().items)).toBe(1)

      mockDelete.mockResolvedValue(undefined)
      await sessionStore.remove('s1')

      expect(nmSize(sessionStore.store.getState().items)).toBe(0)
      expect(mockDelete).toHaveBeenCalledWith('s1')
    })

    it('rolls back on api failure', async () => {
      mockList.mockResolvedValue([session1])
      await sessionStore.fetchAll()

      mockDelete.mockRejectedValue(new Error('Delete failed'))
      await expect(sessionStore.remove('s1')).rejects.toThrow('Delete failed')

      expect(nmSize(sessionStore.store.getState().items)).toBe(1)
    })
  })

  describe('selectors', () => {
    it('selectAll returns array', async () => {
      mockList.mockResolvedValue([session1, session2])
      await sessionStore.fetchAll()

      const all = sessionStore.selectAll(sessionStore.store.getState())
      expect(all).toHaveLength(2)
    })

    it('selectById returns undefined for missing', () => {
      const result = sessionStore.selectById('missing')(sessionStore.store.getState())
      expect(result).toBeUndefined()
    })
  })

  describe('handleWsEvent', () => {
    it('SESSION_EVENT.CREATED upserts a new session', () => {
      sessionStore.handleWsEvent({
        topic: 'session',
        event: 'created',
        ts: '2025-06-01T00:00:00Z',
        run_id: null,
        user_id: null,
        data: { session_id: 's-new', title: 'New Session', mode_id: 'home' },
      })

      const session = nmGet(sessionStore.store.getState().items, 's-new')
      expect(session).toBeDefined()
      expect(session!.id).toBe('s-new')
      expect(session!.title).toBe('New Session')
      expect(session!.mode_id).toBe('home')
      expect(session!.agent_id).toBeNull()
      expect(session!.draft_config).toBeNull()
      expect(session!.created_at).toBe('2025-06-01T00:00:00Z')
      expect(session!.updated_at).toBe('2025-06-01T00:00:00Z')
    })

    it('SESSION_EVENT.UPDATED patches existing session fields', () => {
      sessionStore.store.setState((s) => ({
        items: nmSet(s.items, 's1', session1),
      }))

      sessionStore.handleWsEvent({
        topic: 'session',
        event: 'updated',
        ts: '2025-06-01T12:00:00Z',
        run_id: null,
        user_id: null,
        data: { session_id: 's1', title: 'Updated Title' },
      })

      const session = nmGet(sessionStore.store.getState().items, 's1')
      expect(session!.title).toBe('Updated Title')
      expect(session!.mode_id).toBe('home')
      expect(session!.updated_at).toBe('2025-06-01T12:00:00Z')
    })

    it('SESSION_EVENT.UPDATED ignores unknown session', () => {
      sessionStore.handleWsEvent({
        topic: 'session',
        event: 'updated',
        ts: '2025-06-01T12:00:00Z',
        run_id: null,
        user_id: null,
        data: { session_id: 'unknown', title: 'Nope' },
      })

      expect(nmSize(sessionStore.store.getState().items)).toBe(0)
    })

    it('SESSION_EVENT.DELETED removes session from store', () => {
      sessionStore.store.setState((s) => ({
        items: nmSet(s.items, 's1', session1),
      }))

      sessionStore.handleWsEvent({
        topic: 'session',
        event: 'deleted',
        ts: '2025-06-01T12:00:00Z',
        run_id: null,
        user_id: null,
        data: { session_id: 's1' },
      })

      expect(nmGet(sessionStore.store.getState().items, 's1')).toBeUndefined()
    })
  })
})
