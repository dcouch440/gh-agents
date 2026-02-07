// ============================================================================
// createResourceStore — CRUD Factory
// ============================================================================

import { createStore } from './createStore'
import { createNormalizedMap, nmFromArray, nmSet, nmDelete, toArray, nmGet } from './NormalizedMap'
import type { StoreApi } from './types'
import type { NormalizedMap } from './NormalizedMap'

// ── Types ────────────────────────────────────────────────────────────────────

type Identifiable = { id: string }

type ResourceStoreConfig<T extends Identifiable, TCreate, TUpdate> = {
  name: string
  api: {
    list: (...args: unknown[]) => Promise<unknown>
    get: (id: string) => Promise<T>
    create: (body: TCreate) => Promise<T>
    update: (id: string, body: TUpdate) => Promise<T>
    delete: (id: string) => Promise<void>
  }
  unwrapList: (response: unknown) => T[]
  staleThresholdMs?: number
}

type ResourceState<T extends Identifiable> = {
  items: NormalizedMap<T>
  loading: boolean
  error: string | null
  lastFetched: number | null
}

type ResourceStore<T extends Identifiable, TCreate, TUpdate> = {
  store: StoreApi<ResourceState<T>>

  // Selectors
  selectAll: (state: ResourceState<T>) => T[]
  selectById: (id: string) => (state: ResourceState<T>) => T | undefined
  selectLoading: (state: ResourceState<T>) => boolean
  selectError: (state: ResourceState<T>) => string | null
  selectIsStale: (state: ResourceState<T>) => boolean

  // Async actions
  fetchAll: () => Promise<void>
  fetchIfStale: () => Promise<void>
  fetchOne: (id: string) => Promise<T>
  create: (body: TCreate) => Promise<T>
  update: (id: string, body: TUpdate) => Promise<T>
  remove: (id: string) => Promise<void>

  // Sync mutations
  upsert: (item: T) => void
  removeById: (id: string) => void
  setAll: (items: T[]) => void
}

// ── Factory ──────────────────────────────────────────────────────────────────

const createResourceStore = <T extends Identifiable, TCreate = Partial<T>, TUpdate = Partial<T>>(
  config: ResourceStoreConfig<T, TCreate, TUpdate>,
): ResourceStore<T, TCreate, TUpdate> => {
  const { api, unwrapList, name } = config
  const staleThresholdMs = config.staleThresholdMs ?? 60_000

  const store = createStore<ResourceState<T>>(() => ({
    items: createNormalizedMap<T>(),
    loading: false,
    error: null,
    lastFetched: null,
  }))

  // ── Selectors ────────────────────────────────────────────────────────────

  const selectAll = (state: ResourceState<T>): T[] => toArray(state.items)

  const selectById = (id: string) => (state: ResourceState<T>): T | undefined =>
    nmGet(state.items, id)

  const selectLoading = (state: ResourceState<T>): boolean => state.loading

  const selectError = (state: ResourceState<T>): string | null => state.error

  const selectIsStale = (state: ResourceState<T>): boolean =>
    state.lastFetched === null || Date.now() - state.lastFetched > staleThresholdMs

  // ── Helpers ──────────────────────────────────────────────────────────────

  const extractError = (e: unknown): string => {
    if (e instanceof Error) return e.message
    return `${name}: unknown error`
  }

  // ── Async Actions ────────────────────────────────────────────────────────

  const fetchAll = async (): Promise<void> => {
    store.setState({ loading: true, error: null })
    try {
      const response = await api.list()
      const items = unwrapList(response)
      store.setState({ items: nmFromArray(items), loading: false, lastFetched: Date.now() })
    } catch (e) {
      store.setState({ loading: false, error: extractError(e) })
    }
  }

  const fetchIfStale = async (): Promise<void> => {
    if (selectIsStale(store.getState())) {
      await fetchAll()
    }
  }

  const fetchOne = async (id: string): Promise<T> => {
    const item = await api.get(id)
    store.setState((s) => ({ items: nmSet(s.items, item.id, item) }))
    return item
  }

  const create = async (body: TCreate): Promise<T> => {
    const item = await api.create(body)
    store.setState((s) => ({ items: nmSet(s.items, item.id, item) }))
    return item
  }

  const update = async (id: string, body: TUpdate): Promise<T> => {
    const item = await api.update(id, body)
    store.setState((s) => ({ items: nmSet(s.items, item.id, item) }))
    return item
  }

  const remove = async (id: string): Promise<void> => {
    const prev = store.getState()
    store.setState((s) => ({ items: nmDelete(s.items, id) }))
    try {
      await api.delete(id)
    } catch (e) {
      // Rollback
      store.setState({ items: prev.items, error: extractError(e) })
      throw e
    }
  }

  // ── Sync Mutations ───────────────────────────────────────────────────────

  const upsert = (item: T): void => {
    store.setState((s) => ({ items: nmSet(s.items, item.id, item) }))
  }

  const removeById = (id: string): void => {
    store.setState((s) => ({ items: nmDelete(s.items, id) }))
  }

  const setAll = (items: T[]): void => {
    store.setState({ items: nmFromArray(items) })
  }

  return {
    store,
    selectAll, selectById, selectLoading, selectError, selectIsStale,
    fetchAll, fetchIfStale, fetchOne, create, update, remove,
    upsert, removeById, setAll,
  }
}

export { createResourceStore }
export type { Identifiable, ResourceStoreConfig, ResourceState, ResourceStore }
