// ============================================================================
// collectionStore — Hand-written store for collections + runs
// ============================================================================

import { createStore, createNormalizedMap, nmFromArray, nmSet, nmDelete, toArray, nmGet, extractError } from './lib'
import type { NormalizedMap } from './lib'
import { api } from '@/api'
import type { Collection, CollectionRun, CreateCollectionRequest, UpdateCollectionRequest } from '@/types/collection'

// ── State ────────────────────────────────────────────────────────────────────

type CollectionState = {
  items: NormalizedMap<Collection>
  runsByCollection: Record<string, CollectionRun[]>
  loading: boolean
  error: string | null
}

// ── Store ────────────────────────────────────────────────────────────────────

const store = createStore<CollectionState>(() => ({
  items: createNormalizedMap<Collection>(),
  runsByCollection: {},
  loading: false,
  error: null,
}))

// ── Helpers ──────────────────────────────────────────────────────────────────

const EMPTY_RUNS: CollectionRun[] = []

// ── Selectors ────────────────────────────────────────────────────────────────

const selectAll = (s: CollectionState): Collection[] => toArray(s.items)

const selectById =
  (id: string) =>
  (s: CollectionState): Collection | undefined =>
    nmGet(s.items, id)

const selectRuns =
  (collectionId: string) =>
  (s: CollectionState): CollectionRun[] =>
    s.runsByCollection[collectionId] ?? EMPTY_RUNS

const selectLoading = (s: CollectionState): boolean => s.loading

const selectError = (s: CollectionState): string | null => s.error

// ── Collection CRUD ──────────────────────────────────────────────────────────

const fetchAll = async (): Promise<void> => {
  store.setState({ loading: true, error: null })
  try {
    const data = await api.collections.list()
    store.setState({ items: nmFromArray(data), loading: false })
  } catch (e) {
    store.setState({ loading: false, error: extractError('collections', e) })
  }
}

const fetchOne = async (id: string): Promise<Collection> => {
  const collection = await api.collections.get(id)
  store.setState((s) => ({ items: nmSet(s.items, collection.id, collection) }))
  return collection
}

const create = async (body: CreateCollectionRequest): Promise<Collection> => {
  const collection = await api.collections.create(body)
  store.setState((s) => ({ items: nmSet(s.items, collection.id, collection) }))
  return collection
}

const update = async (id: string, body: UpdateCollectionRequest): Promise<Collection> => {
  const collection = await api.collections.update(id, body)
  store.setState((s) => ({ items: nmSet(s.items, collection.id, collection) }))
  return collection
}

const remove = async (id: string): Promise<void> => {
  const prev = store.getState()
  store.setState((s) => ({ items: nmDelete(s.items, id) }))
  try {
    await api.collections.delete(id)
  } catch (e) {
    store.setState({ items: prev.items, error: extractError('collections', e) })
    throw e
  }
}

// ── Execution ────────────────────────────────────────────────────────────────

const execute = async (id: string): Promise<CollectionRun> => {
  const run = await api.collections.run(id)
  store.setState((s) => ({
    runsByCollection: {
      ...s.runsByCollection,
      [id]: [...(s.runsByCollection[id] ?? []), run],
    },
  }))
  return run
}

const fetchRunStatus = async (runId: string): Promise<CollectionRun> => {
  const run = await api.collections.getRunStatus(runId)
  store.setState((s) => ({
    runsByCollection: {
      ...s.runsByCollection,
      [run.collection_id]: (s.runsByCollection[run.collection_id] ?? []).map((r) => (r.id === runId ? run : r)),
    },
  }))
  return run
}

// ── Sync / Utility ───────────────────────────────────────────────────────────

const upsert = (collection: Collection): void => {
  store.setState((s) => ({ items: nmSet(s.items, collection.id, collection) }))
}

const removeById = (id: string): void => {
  store.setState((s) => ({ items: nmDelete(s.items, id) }))
}

// ── Export ────────────────────────────────────────────────────────────────────

export const collectionStore = {
  store,
  selectAll,
  selectById,
  selectRuns,
  selectLoading,
  selectError,
  fetchAll,
  fetchOne,
  create,
  update,
  remove,
  execute,
  fetchRunStatus,
  upsert,
  removeById,
}

export type { CollectionState }
