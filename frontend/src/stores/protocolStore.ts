// ============================================================================
// protocolStore — Hand-written store for protocols + sub-resources
// ============================================================================

import { createStore, createNormalizedMap, nmFromArray, nmSet, nmDelete, toArray, nmGet, logger } from './lib'
import type { NormalizedMap } from './lib'
import { api } from '@/api'
import type { Protocol, ProtocolTypeInfo, CreateProtocolRequest, UpdateProtocolRequest, CreatePortRequest } from '@/types/protocol'

// ── State ────────────────────────────────────────────────────────────────────

type ProtocolState = {
  items: NormalizedMap<Protocol>
  types: ProtocolTypeInfo[]
  loading: boolean
  error: string | null
}

// ── Store ────────────────────────────────────────────────────────────────────

const store = logger(
  'protocolStore',
  createStore<ProtocolState>(() => ({
    items: createNormalizedMap<Protocol>(),
    types: [],
    loading: false,
    error: null,
  })),
)

// ── Helpers ──────────────────────────────────────────────────────────────────

const extractError = (e: unknown): string => (e instanceof Error ? e.message : 'protocols: unknown error')

// ── Selectors ────────────────────────────────────────────────────────────────

const selectAll = (s: ProtocolState): Protocol[] => toArray(s.items)

const selectById =
  (id: string) =>
  (s: ProtocolState): Protocol | undefined =>
    nmGet(s.items, id)

const selectTypes = (s: ProtocolState): ProtocolTypeInfo[] => s.types

const selectByType =
  (protocolType: string) =>
  (s: ProtocolState): Protocol[] =>
    toArray(s.items).filter((p) => p.protocol_type === protocolType)

const selectLoading = (s: ProtocolState): boolean => s.loading

const selectError = (s: ProtocolState): string | null => s.error

// ── Async Actions ────────────────────────────────────────────────────────────

const fetchAll = async (): Promise<void> => {
  store.setState({ loading: true, error: null })
  try {
    const data = await api.protocols.list()
    store.setState({ items: nmFromArray(data), loading: false })
  } catch (e) {
    store.setState({ loading: false, error: extractError(e) })
  }
}

const fetchOne = async (id: string): Promise<Protocol> => {
  const protocol = await api.protocols.get(id)
  store.setState((s) => ({ items: nmSet(s.items, protocol.id, protocol) }))
  return protocol
}

const fetchTypes = async (): Promise<void> => {
  try {
    const data = await api.protocols.listTypes()
    store.setState({ types: data.types })
  } catch (e) {
    store.setState({ error: extractError(e) })
  }
}

const create = async (body: CreateProtocolRequest): Promise<Protocol> => {
  const protocol = await api.protocols.create(body)
  store.setState((s) => ({ items: nmSet(s.items, protocol.id, protocol) }))
  return protocol
}

const update = async (id: string, body: UpdateProtocolRequest): Promise<Protocol> => {
  const protocol = await api.protocols.update(id, body)
  store.setState((s) => ({ items: nmSet(s.items, protocol.id, protocol) }))
  return protocol
}

const remove = async (id: string): Promise<void> => {
  const prev = store.getState()
  store.setState((s) => ({ items: nmDelete(s.items, id) }))
  try {
    await api.protocols.delete(id)
  } catch (e) {
    store.setState({ items: prev.items, error: extractError(e) })
    throw e
  }
}

// ── Sub-resource: Ports ─────────────────────────────────────────────────────

const createPort = async (protocolId: string, body: CreatePortRequest): Promise<void> => {
  await api.protocols.createPort(protocolId, body)
  await fetchOne(protocolId)
}

const deletePort = async (protocolId: string, portId: string): Promise<void> => {
  await api.protocols.deletePort(protocolId, portId)
  await fetchOne(protocolId)
}

// ── Export ────────────────────────────────────────────────────────────────────

export const protocolStore = {
  store,
  selectAll,
  selectById,
  selectTypes,
  selectByType,
  selectLoading,
  selectError,
  fetchAll,
  fetchOne,
  fetchTypes,
  create,
  update,
  remove,
  createPort,
  deletePort,
}

export type { ProtocolState }
