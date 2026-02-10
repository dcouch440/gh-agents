// ============================================================================
// costStore — Singleton store for cost/spend data
// ============================================================================

import { createStore } from './lib'
import { api } from '@/api'
import type { CostResponse } from '@/types/cost'

// ── Constants ────────────────────────────────────────────────────────────────

const STALE_THRESHOLD_MS = 60_000

// ── State ────────────────────────────────────────────────────────────────────

type CostState = {
  summary: CostResponse | null
  loading: boolean
  error: string | null
  lastFetched: number | null
}

// ── Store ────────────────────────────────────────────────────────────────────

const store = createStore<CostState>(() => ({
  summary: null,
  loading: false,
  error: null,
  lastFetched: null,
}))

// ── Helpers ──────────────────────────────────────────────────────────────────

const extractError = (e: unknown): string => (e instanceof Error ? e.message : 'costs: unknown error')

// ── Selectors ────────────────────────────────────────────────────────────────

const selectSummary = (s: CostState): CostResponse | null => s.summary

const selectLoading = (s: CostState): boolean => s.loading

const selectError = (s: CostState): string | null => s.error

const selectIsStale = (s: CostState): boolean => s.lastFetched === null || Date.now() - s.lastFetched > STALE_THRESHOLD_MS

// ── Async Actions ────────────────────────────────────────────────────────────

const fetchSummary = async (): Promise<void> => {
  store.setState({ loading: true, error: null })
  try {
    const data = await api.costs.list()
    store.setState({ summary: data, loading: false, lastFetched: Date.now() })
  } catch (e) {
    store.setState({ loading: false, error: extractError(e) })
  }
}

// ── Export ────────────────────────────────────────────────────────────────────

export const costStore = {
  store,
  selectSummary,
  selectLoading,
  selectError,
  selectIsStale,
  fetchSummary,
}

export type { CostState }
