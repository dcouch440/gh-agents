import { createResourceStore } from './createResourceStore'
import { toArray, nmGet, nmSize } from './NormalizedMap'

type Widget = { id: string; label: string }
type CreateWidget = { label: string }
type UpdateWidget = { label?: string }

const widget1: Widget = { id: 'w1', label: 'Alpha' }
const widget2: Widget = { id: 'w2', label: 'Beta' }
const widget3: Widget = { id: 'w3', label: 'Gamma' }

const makeMockApi = () => ({
  list: vi.fn<() => Promise<{ items: Widget[] }>>().mockResolvedValue({ items: [widget1, widget2] }),
  get: vi.fn<(id: string) => Promise<Widget>>().mockResolvedValue(widget1),
  create: vi.fn<(body: CreateWidget) => Promise<Widget>>().mockResolvedValue(widget3),
  update: vi.fn<(id: string, body: UpdateWidget) => Promise<Widget>>().mockResolvedValue({ ...widget1, label: 'Updated' }),
  delete: vi.fn<(id: string) => Promise<void>>().mockResolvedValue(undefined),
})

const makeStore = (apiOverrides?: Partial<ReturnType<typeof makeMockApi>>) => {
  const api = { ...makeMockApi(), ...apiOverrides }
  return {
    ...createResourceStore<Widget, CreateWidget, UpdateWidget>({
      name: 'widgets',
      api,
      unwrapList: (res) => (res as { items: Widget[] }).items,
    }),
    api,
  }
}

describe('createResourceStore', () => {
  describe('fetchAll', () => {
    it('populates store from API response', async () => {
      const { store, fetchAll } = makeStore()
      await fetchAll()

      const state = store.getState()
      expect(nmSize(state.items)).toBe(2)
      expect(nmGet(state.items, 'w1')).toEqual(widget1)
      expect(nmGet(state.items, 'w2')).toEqual(widget2)
    })

    it('sets loading true then false', async () => {
      const { store, fetchAll } = makeStore()

      const loadingStates: boolean[] = []
      store.subscribe(() => {
        loadingStates.push(store.getState().loading)
      })

      await fetchAll()

      expect(loadingStates[0]).toBe(true)
      expect(loadingStates[loadingStates.length - 1]).toBe(false)
    })

    it('sets error on API failure', async () => {
      const { store, fetchAll } = makeStore({
        list: vi.fn().mockRejectedValue(new Error('Network error')),
      })

      await fetchAll()

      const state = store.getState()
      expect(state.error).toBe('Network error')
      expect(state.loading).toBe(false)
    })
  })

  describe('fetchOne', () => {
    it('adds item to store', async () => {
      const { store, fetchOne } = makeStore({
        get: vi.fn().mockResolvedValue(widget3),
      })

      const result = await fetchOne('w3')

      expect(result).toEqual(widget3)
      expect(nmGet(store.getState().items, 'w3')).toEqual(widget3)
    })
  })

  describe('create', () => {
    it('adds new item to store', async () => {
      const { store, create, api } = makeStore()

      const result = await create({ label: 'Gamma' })

      expect(api.create).toHaveBeenCalledWith({ label: 'Gamma' })
      expect(result).toEqual(widget3)
      expect(nmGet(store.getState().items, 'w3')).toEqual(widget3)
    })
  })

  describe('update', () => {
    it('replaces item in store', async () => {
      const updated = { ...widget1, label: 'Updated' }
      const { store, fetchAll, update, api } = makeStore({
        update: vi.fn().mockResolvedValue(updated),
      })

      await fetchAll()
      const result = await update('w1', { label: 'Updated' })

      expect(api.update).toHaveBeenCalledWith('w1', { label: 'Updated' })
      expect(result).toEqual(updated)
      expect(nmGet(store.getState().items, 'w1')?.label).toBe('Updated')
    })
  })

  describe('remove', () => {
    it('deletes item from store', async () => {
      const { store, fetchAll, remove, api } = makeStore()

      await fetchAll()
      expect(nmSize(store.getState().items)).toBe(2)

      await remove('w1')

      expect(api.delete).toHaveBeenCalledWith('w1')
      expect(nmSize(store.getState().items)).toBe(1)
      expect(nmGet(store.getState().items, 'w1')).toBeUndefined()
    })

    it('rolls back on API failure', async () => {
      const { store, fetchAll, remove } = makeStore({
        delete: vi.fn().mockRejectedValue(new Error('Delete failed')),
      })

      await fetchAll()
      expect(nmSize(store.getState().items)).toBe(2)

      await expect(remove('w1')).rejects.toThrow('Delete failed')

      // Rolled back
      expect(nmSize(store.getState().items)).toBe(2)
      expect(nmGet(store.getState().items, 'w1')).toEqual(widget1)
      expect(store.getState().error).toBe('Delete failed')
    })
  })

  describe('sync mutations', () => {
    it('upsert adds item synchronously', () => {
      const { store, upsert } = makeStore()
      upsert(widget3)
      expect(nmGet(store.getState().items, 'w3')).toEqual(widget3)
    })

    it('removeById removes item synchronously', () => {
      const { store, upsert, removeById } = makeStore()
      upsert(widget1)
      upsert(widget2)
      removeById('w1')
      expect(nmGet(store.getState().items, 'w1')).toBeUndefined()
      expect(nmGet(store.getState().items, 'w2')).toEqual(widget2)
    })

    it('setAll replaces all items', () => {
      const { store, upsert, setAll } = makeStore()
      upsert(widget1)
      setAll([widget2, widget3])

      const items = toArray(store.getState().items)
      expect(items).toHaveLength(2)
      expect(nmGet(store.getState().items, 'w1')).toBeUndefined()
      expect(nmGet(store.getState().items, 'w2')).toEqual(widget2)
    })
  })

  describe('stale data', () => {
    it('starts as stale (lastFetched is null)', () => {
      const { store, selectIsStale } = makeStore()
      expect(selectIsStale(store.getState())).toBe(true)
      expect(store.getState().lastFetched).toBeNull()
    })

    it('is not stale after fetchAll', async () => {
      const { store, selectIsStale, fetchAll } = makeStore()
      await fetchAll()
      expect(selectIsStale(store.getState())).toBe(false)
      expect(store.getState().lastFetched).toBeTypeOf('number')
    })

    it('does not set lastFetched on fetchAll failure', async () => {
      const { store, selectIsStale, fetchAll } = makeStore({
        list: vi.fn().mockRejectedValue(new Error('fail')),
      })
      await fetchAll()
      expect(store.getState().lastFetched).toBeNull()
      expect(selectIsStale(store.getState())).toBe(true)
    })

    it('fetchIfStale calls API when stale', async () => {
      const { store, fetchIfStale, api } = makeStore()
      expect(store.getState().lastFetched).toBeNull()
      await fetchIfStale()
      expect(api.list).toHaveBeenCalledTimes(1)
      expect(store.getState().lastFetched).toBeTypeOf('number')
    })

    it('fetchIfStale skips API when fresh', async () => {
      const { fetchAll, fetchIfStale, api } = makeStore()
      await fetchAll()
      expect(api.list).toHaveBeenCalledTimes(1)
      await fetchIfStale()
      expect(api.list).toHaveBeenCalledTimes(1)
    })

    it('respects custom staleThresholdMs', async () => {
      const now = vi.spyOn(Date, 'now')
      now.mockReturnValue(1000)

      const api = makeMockApi()
      const s = createResourceStore<Widget, CreateWidget, UpdateWidget>({
        name: 'widgets',
        api,
        unwrapList: (res) => (res as { items: Widget[] }).items,
        staleThresholdMs: 500,
      })
      await s.fetchAll()
      // 200ms later — still fresh
      now.mockReturnValue(1200)
      expect(s.selectIsStale(s.store.getState())).toBe(false)
      // 600ms later — stale
      now.mockReturnValue(1600)
      expect(s.selectIsStale(s.store.getState())).toBe(true)

      now.mockRestore()
    })
  })

  describe('selectors', () => {
    it('selectAll returns memoized array', async () => {
      const { store, selectAll, fetchAll } = makeStore()
      await fetchAll()

      const state = store.getState()
      const arr1 = selectAll(state)
      const arr2 = selectAll(state)
      expect(arr1).toBe(arr2)
      expect(arr1).toHaveLength(2)
    })

    it('selectById returns correct item', async () => {
      const { store, selectById, fetchAll } = makeStore()
      await fetchAll()

      const state = store.getState()
      expect(selectById('w1')(state)).toEqual(widget1)
      expect(selectById('nonexistent')(state)).toBeUndefined()
    })
  })
})
