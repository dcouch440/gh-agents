// ============================================================================
// resultStore — Read-only store for execution results
// ============================================================================

import { createStore, createNormalizedMap, nmFromArray, nmSet, toArray, nmGet } from './lib'
import type { NormalizedMap } from './lib'
import { api } from '@/api'
import type { Result } from '@/types/result'

// ── State ────────────────────────────────────────────────────────────────────

type ResultState = {
  items: NormalizedMap<Result>
  loading: boolean
  error: string | null
}

// ── Store ────────────────────────────────────────────────────────────────────

const store = createStore<ResultState>(() => ({
  items: createNormalizedMap<Result>(),
  loading: false,
  error: null,
}))

// ── Helpers ──────────────────────────────────────────────────────────────────

const extractError = (e: unknown): string =>
  e instanceof Error ? e.message : 'results: unknown error'

// ── Selectors ────────────────────────────────────────────────────────────────

const selectAll = (s: ResultState): Result[] => toArray(s.items)

const selectById = (id: string) => (s: ResultState): Result | undefined =>
  nmGet(s.items, id)

const selectLoading = (s: ResultState): boolean => s.loading

const selectError = (s: ResultState): string | null => s.error

// ── Async Actions ────────────────────────────────────────────────────────────

const fetchAll = async (): Promise<void> => {
  store.setState({ loading: true, error: null })
  try {
    const data = await api.results.list()
    store.setState({ items: nmFromArray((data as { items: Result[] }).items), loading: false })
  } catch (e) {
    store.setState({ loading: false, error: extractError(e) })
  }
}

const fetchOne = async (id: string): Promise<Result> => {
  const result = await api.results.get(id)
  store.setState((s) => ({ items: nmSet(s.items, result.id, result) }))
  return result
}

// ── Export ────────────────────────────────────────────────────────────────────

export const resultStore = {
  store,
  selectAll,
  selectById,
  selectLoading,
  selectError,
  fetchAll,
  fetchOne,
}

export type { ResultState }
