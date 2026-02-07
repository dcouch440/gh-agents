// ============================================================================
// sessionStore — Hand-written store for sessions
// ============================================================================

import { createStore, createNormalizedMap, nmFromArray, nmSet, nmDelete, toArray, nmGet } from './lib'
import type { NormalizedMap } from './lib'
import { api } from '@/api'
import type { Session, CreateSessionRequest, UpdateSessionRequest } from '@/types/session'

// ── State ────────────────────────────────────────────────────────────────────

type SessionState = {
  items: NormalizedMap<Session>
  loading: boolean
  error: string | null
}

// ── Store ────────────────────────────────────────────────────────────────────

const store = createStore<SessionState>(() => ({
  items: createNormalizedMap<Session>(),
  loading: false,
  error: null,
}))

// ── Helpers ──────────────────────────────────────────────────────────────────

const extractError = (e: unknown): string =>
  e instanceof Error ? e.message : 'sessions: unknown error'

// ── Selectors ────────────────────────────────────────────────────────────────

const selectAll = (s: SessionState): Session[] => toArray(s.items)

const selectById = (id: string) => (s: SessionState): Session | undefined =>
  nmGet(s.items, id)

const selectLoading = (s: SessionState): boolean => s.loading

const selectError = (s: SessionState): string | null => s.error

// ── Async Actions ────────────────────────────────────────────────────────────

const fetchAll = async (): Promise<void> => {
  store.setState({ loading: true, error: null })
  try {
    const data = await api.sessions.list()
    store.setState({ items: nmFromArray(data.items), loading: false })
  } catch (e) {
    store.setState({ loading: false, error: extractError(e) })
  }
}

const fetchOne = async (id: string): Promise<Session> => {
  const session = await api.sessions.get(id)
  store.setState((s) => ({ items: nmSet(s.items, session.id, session) }))
  return session
}

const create = async (body: CreateSessionRequest): Promise<Session> => {
  const session = await api.sessions.create(body)
  store.setState((s) => ({ items: nmSet(s.items, session.id, session) }))
  return session
}

const update = async (id: string, body: UpdateSessionRequest): Promise<Session> => {
  const session = await api.sessions.update(id, body)
  store.setState((s) => ({ items: nmSet(s.items, session.id, session) }))
  return session
}

const remove = async (id: string): Promise<void> => {
  const prev = store.getState()
  store.setState((s) => ({ items: nmDelete(s.items, id) }))
  try {
    await api.sessions.delete(id)
  } catch (e) {
    store.setState({ items: prev.items, error: extractError(e) })
    throw e
  }
}

// ── Export ────────────────────────────────────────────────────────────────────

export const sessionStore = {
  store,
  selectAll,
  selectById,
  selectLoading,
  selectError,
  fetchAll,
  fetchOne,
  create,
  update,
  remove,
}

export type { SessionState }
