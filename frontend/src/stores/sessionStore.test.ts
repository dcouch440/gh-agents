import { sessionStore } from './sessionStore'
import { nmSize, nmGet, createNormalizedMap } from './lib'

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
      mockList.mockResolvedValue({ items: [session1, session2] })
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
      mockList.mockResolvedValue({ items: [session1] })
      await sessionStore.fetchAll()
      expect(nmSize(sessionStore.store.getState().items)).toBe(1)

      mockDelete.mockResolvedValue(undefined)
      await sessionStore.remove('s1')

      expect(nmSize(sessionStore.store.getState().items)).toBe(0)
      expect(mockDelete).toHaveBeenCalledWith('s1')
    })

    it('rolls back on api failure', async () => {
      mockList.mockResolvedValue({ items: [session1] })
      await sessionStore.fetchAll()

      mockDelete.mockRejectedValue(new Error('Delete failed'))
      await expect(sessionStore.remove('s1')).rejects.toThrow('Delete failed')

      expect(nmSize(sessionStore.store.getState().items)).toBe(1)
    })
  })

  describe('selectors', () => {
    it('selectAll returns array', async () => {
      mockList.mockResolvedValue({ items: [session1, session2] })
      await sessionStore.fetchAll()

      const all = sessionStore.selectAll(sessionStore.store.getState())
      expect(all).toHaveLength(2)
    })

    it('selectById returns undefined for missing', () => {
      const result = sessionStore.selectById('missing')(sessionStore.store.getState())
      expect(result).toBeUndefined()
    })
  })
})
