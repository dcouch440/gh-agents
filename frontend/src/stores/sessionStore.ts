// ============================================================================
// sessionStore — Hand-written store for sessions
// ============================================================================

import { createStore, createNormalizedMap, nmFromArray, nmSet, nmDelete, toArray, nmGet, extractError } from './lib'
import type { NormalizedMap } from './lib'
import { api } from '@/api'
import type { Session, CreateSessionRequest, UpdateSessionRequest } from '@/types/session'
import { SESSION_EVENT } from '@/types/ws'
import type { WsWireMessage } from '@/types/ws'

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

// ── Selectors ────────────────────────────────────────────────────────────────

const selectAll = (s: SessionState): Session[] => toArray(s.items)

const selectById =
  (id: string) =>
  (s: SessionState): Session | undefined =>
    nmGet(s.items, id)

const selectLoading = (s: SessionState): boolean => s.loading

const selectError = (s: SessionState): string | null => s.error

// ── Async Actions ────────────────────────────────────────────────────────────

const fetchAll = async (): Promise<void> => {
  store.setState({ loading: true, error: null })
  try {
    const data = await api.sessions.list()
    store.setState({ items: nmFromArray(data as Session[]), loading: false })
  } catch (e) {
    store.setState({ loading: false, error: extractError('sessions', e) })
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
    store.setState({ items: prev.items, error: extractError('sessions', e) })
    throw e
  }
}

// ── WebSocket Handler ────────────────────────────────────────────────────────

const handleWsEvent = (msg: WsWireMessage): void => {
  try {
    const data = msg.data

    switch (msg.event) {
      case SESSION_EVENT.CREATED: {
        const session: Session = {
          id: data.session_id as string,
          mode_id: data.mode_id as string,
          agent_id: null,
          draft_config: null,
          title: data.title as string,
          created_at: msg.ts,
          updated_at: msg.ts,
        }
        store.setState((s) => ({ items: nmSet(s.items, session.id, session) }))
        break
      }
      case SESSION_EVENT.UPDATED: {
        const sessionId = data.session_id as string
        store.setState((s) => {
          const existing = nmGet(s.items, sessionId)
          if (!existing) return s
          const patched = { ...existing, updated_at: msg.ts }
          if (typeof data.title === 'string') patched.title = data.title
          if (typeof data.mode_id === 'string') patched.mode_id = data.mode_id
          return { items: nmSet(s.items, sessionId, patched) }
        })
        break
      }
      case SESSION_EVENT.DELETED: {
        const sessionId = data.session_id as string
        store.setState((s) => ({ items: nmDelete(s.items, sessionId) }))
        break
      }
    }
  } catch (err) {
    console.error(`[sessionStore] WS handler error on "${msg.event}":`, err)
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
  handleWsEvent,
}

export type { SessionState }
