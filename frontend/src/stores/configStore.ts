// ============================================================================
// configStore — Singleton store for system config + stats
// ============================================================================

import { createStore } from './lib'
import { api } from '@/api'
import type { Config, UpdateConfigRequest } from '@/types/config'
import type { UsageSummary } from '@/types/stats'

// ── State ────────────────────────────────────────────────────────────────────

type ConfigState = {
  config: Config | null
  stats: UsageSummary[] | null
  loading: boolean
  error: string | null
}

// ── Store ────────────────────────────────────────────────────────────────────

const store = createStore<ConfigState>(() => ({
  config: null,
  stats: null,
  loading: false,
  error: null,
}))

// ── Helpers ──────────────────────────────────────────────────────────────────

const extractError = (e: unknown): string =>
  e instanceof Error ? e.message : 'config: unknown error'

// ── Selectors ────────────────────────────────────────────────────────────────

const selectConfig = (s: ConfigState): Config | null => s.config

const selectStats = (s: ConfigState): UsageSummary[] | null => s.stats

const selectLoading = (s: ConfigState): boolean => s.loading

const selectError = (s: ConfigState): string | null => s.error

// ── Async Actions ────────────────────────────────────────────────────────────

const fetchConfig = async (): Promise<void> => {
  store.setState({ loading: true, error: null })
  try {
    const config = await api.config.get()
    store.setState({ config, loading: false })
  } catch (e) {
    store.setState({ loading: false, error: extractError(e) })
  }
}

const updateConfig = async (body: UpdateConfigRequest): Promise<void> => {
  store.setState({ loading: true, error: null })
  try {
    const config = await api.config.update(body)
    store.setState({ config, loading: false })
  } catch (e) {
    store.setState({ loading: false, error: extractError(e) })
  }
}

const fetchStats = async (): Promise<void> => {
  try {
    const data = await api.stats.get()
    store.setState({ stats: Array.isArray(data) ? data : [data] })
  } catch (e) {
    store.setState({ error: extractError(e) })
  }
}

// ── Export ────────────────────────────────────────────────────────────────────

export const configStore = {
  store,
  selectConfig,
  selectStats,
  selectLoading,
  selectError,
  fetchConfig,
  updateConfig,
  fetchStats,
}

export type { ConfigState }
