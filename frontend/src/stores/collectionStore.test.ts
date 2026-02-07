import { collectionStore } from './collectionStore'
import { nmSize, nmGet, createNormalizedMap } from './lib'
import type { Collection, CollectionRun } from '@/types/collection'

const {
  mockList,
  mockGet,
  mockCreate,
  mockUpdate,
  mockDelete,
  mockRun,
  mockGetRunStatus,
} = vi.hoisted(() => ({
  mockList: vi.fn(),
  mockGet: vi.fn(),
  mockCreate: vi.fn(),
  mockUpdate: vi.fn(),
  mockDelete: vi.fn(),
  mockRun: vi.fn(),
  mockGetRunStatus: vi.fn(),
}))

vi.mock('@/api', () => ({
  api: {
    collections: {
      list: mockList,
      get: mockGet,
      create: mockCreate,
      update: mockUpdate,
      delete: mockDelete,
      run: mockRun,
      getRunStatus: mockGetRunStatus,
    },
  },
}))

const collection1: Collection = {
  id: 'c1',
  user_id: 'u1',
  name: 'Collection 1',
  description: 'A test collection',
  execution_mode: 'sequential',
  created_at: '2024-01-01T00:00:00Z',
  updated_at: '2024-01-01T00:00:00Z',
}

const collection2: Collection = {
  id: 'c2',
  user_id: 'u1',
  name: 'Collection 2',
  description: null,
  execution_mode: 'parallel',
  created_at: '2024-01-01T00:00:00Z',
  updated_at: '2024-01-01T00:00:00Z',
}

const run1: CollectionRun = {
  id: 'run1',
  collection_id: 'c1',
  user_id: 'u1',
  status: 'running',
  started_at: '2024-01-01T00:00:00Z',
  completed_at: null,
  error: null,
}

beforeEach(() => {
  vi.clearAllMocks()
  collectionStore.store.setState({
    items: createNormalizedMap(),
    runsByCollection: {},
    loading: false,
    error: null,
  })
})

describe('collectionStore', () => {
  describe('CRUD', () => {
    it('fetchAll populates collections', async () => {
      mockList.mockResolvedValue({ items: [collection1, collection2] })

      await collectionStore.fetchAll()

      const state = collectionStore.store.getState()
      expect(nmSize(state.items)).toBe(2)
      expect(nmGet(state.items, 'c1')?.name).toBe('Collection 1')
      expect(state.loading).toBe(false)
    })

    it('fetchAll sets error on failure', async () => {
      mockList.mockRejectedValue(new Error('Network error'))

      await collectionStore.fetchAll()

      const state = collectionStore.store.getState()
      expect(state.error).toBe('Network error')
      expect(state.loading).toBe(false)
    })

    it('fetchOne upserts collection', async () => {
      mockGet.mockResolvedValue(collection1)

      const result = await collectionStore.fetchOne('c1')

      expect(result).toEqual(collection1)
      expect(nmGet(collectionStore.store.getState().items, 'c1')).toEqual(collection1)
    })

    it('create adds collection to store', async () => {
      mockCreate.mockResolvedValue(collection1)

      const result = await collectionStore.create({ name: 'Collection 1' })

      expect(result).toEqual(collection1)
      expect(nmGet(collectionStore.store.getState().items, 'c1')).toEqual(collection1)
    })

    it('update replaces collection in store', async () => {
      mockList.mockResolvedValue({ items: [collection1] })
      await collectionStore.fetchAll()

      const updated = { ...collection1, name: 'Updated' }
      mockUpdate.mockResolvedValue(updated)

      const result = await collectionStore.update('c1', { name: 'Updated' })

      expect(result.name).toBe('Updated')
      expect(nmGet(collectionStore.store.getState().items, 'c1')?.name).toBe('Updated')
    })

    it('remove optimistically deletes then calls API', async () => {
      mockList.mockResolvedValue({ items: [collection1, collection2] })
      mockDelete.mockResolvedValue(undefined)
      await collectionStore.fetchAll()

      await collectionStore.remove('c1')

      expect(nmSize(collectionStore.store.getState().items)).toBe(1)
      expect(nmGet(collectionStore.store.getState().items, 'c1')).toBeUndefined()
    })

    it('remove rolls back on API failure', async () => {
      mockList.mockResolvedValue({ items: [collection1] })
      await collectionStore.fetchAll()

      mockDelete.mockRejectedValue(new Error('Server error'))

      await expect(collectionStore.remove('c1')).rejects.toThrow('Server error')
      expect(nmSize(collectionStore.store.getState().items)).toBe(1)
    })
  })

  describe('execution', () => {
    it('execute starts a run and stores it', async () => {
      mockRun.mockResolvedValue(run1)

      const result = await collectionStore.execute('c1')

      expect(result).toEqual(run1)
      expect(collectionStore.store.getState().runsByCollection['c1']).toEqual([run1])
    })

    it('execute appends to existing runs', async () => {
      mockRun.mockResolvedValue(run1)
      await collectionStore.execute('c1')

      const run2 = { ...run1, id: 'run2' }
      mockRun.mockResolvedValue(run2)
      await collectionStore.execute('c1')

      expect(collectionStore.store.getState().runsByCollection['c1']).toHaveLength(2)
    })

    it('fetchRunStatus updates existing run', async () => {
      mockRun.mockResolvedValue(run1)
      await collectionStore.execute('c1')

      const completed = { ...run1, status: 'completed', completed_at: '2024-01-01T01:00:00Z' }
      mockGetRunStatus.mockResolvedValue(completed)

      const result = await collectionStore.fetchRunStatus('run1')

      expect(result.status).toBe('completed')
      expect(collectionStore.store.getState().runsByCollection['c1']?.[0]?.status).toBe('completed')
    })
  })

  describe('sync utilities', () => {
    it('upsert adds collection without API call', () => {
      collectionStore.upsert(collection1)
      expect(nmGet(collectionStore.store.getState().items, 'c1')).toEqual(collection1)
    })

    it('removeById removes collection without API call', () => {
      collectionStore.upsert(collection1)
      collectionStore.removeById('c1')
      expect(nmGet(collectionStore.store.getState().items, 'c1')).toBeUndefined()
    })
  })

  describe('selectors', () => {
    it('selectById returns undefined for missing collection', () => {
      const result = collectionStore.selectById('missing')(collectionStore.store.getState())
      expect(result).toBeUndefined()
    })

    it('selectRuns returns empty array for unknown collection', () => {
      const result = collectionStore.selectRuns('unknown')(collectionStore.store.getState())
      expect(result).toEqual([])
    })
  })
})
