// ============================================================================
// toolRouterStore — Hand-written store for tool routers + modes + tools
// ============================================================================

import { createStore, createNormalizedMap, nmFromArray, nmSet, nmDelete, toArray, nmGet } from './lib'
import type { NormalizedMap } from './lib'
import { api } from '@/api'
import type { ToolRouter, CreateToolRouterRequest, UpdateToolRouterRequest, SetRouterToolsRequest } from '@/types/toolRouter'
import type { RouterMode, CreateRouterModeRequest, UpdateRouterModeRequest, SetModeToolsRequest } from '@/types/router'
import type { Tool } from '@/types/tool'

// ── State ────────────────────────────────────────────────────────────────────

type ToolRouterState = {
  items: NormalizedMap<ToolRouter>
  toolsByRouter: Record<string, Tool[]>
  modesByRouter: Record<string, RouterMode[]>
  toolsByMode: Record<string, Tool[]>
  loading: boolean
  error: string | null
}

// ── Store ────────────────────────────────────────────────────────────────────

const store = createStore<ToolRouterState>(() => ({
  items: createNormalizedMap<ToolRouter>(),
  toolsByRouter: {},
  modesByRouter: {},
  toolsByMode: {},
  loading: false,
  error: null,
}))

// ── Helpers ──────────────────────────────────────────────────────────────────

const extractError = (e: unknown): string =>
  e instanceof Error ? e.message : 'toolRouters: unknown error'

// ── Selectors ────────────────────────────────────────────────────────────────

const selectAll = (s: ToolRouterState): ToolRouter[] => toArray(s.items)

const selectById = (id: string) => (s: ToolRouterState): ToolRouter | undefined =>
  nmGet(s.items, id)

const selectRouterTools = (routerId: string) => (s: ToolRouterState): Tool[] =>
  s.toolsByRouter[routerId] ?? []

const selectModes = (routerId: string) => (s: ToolRouterState): RouterMode[] =>
  s.modesByRouter[routerId] ?? []

const selectModeTools = (modeId: string) => (s: ToolRouterState): Tool[] =>
  s.toolsByMode[modeId] ?? []

const selectLoading = (s: ToolRouterState): boolean => s.loading

const selectError = (s: ToolRouterState): string | null => s.error

// ── Async Actions: Router CRUD ───────────────────────────────────────────────

const fetchAll = async (): Promise<void> => {
  store.setState({ loading: true, error: null })
  try {
    const data = await api.toolRouters.list()
    store.setState({ items: nmFromArray(data), loading: false })
  } catch (e) {
    store.setState({ loading: false, error: extractError(e) })
  }
}

const fetchOne = async (id: string): Promise<ToolRouter> => {
  const router = await api.toolRouters.get(id)
  store.setState((s) => ({ items: nmSet(s.items, router.id, router) }))
  return router
}

const create = async (body: CreateToolRouterRequest): Promise<ToolRouter> => {
  const router = await api.toolRouters.create(body)
  store.setState((s) => ({ items: nmSet(s.items, router.id, router) }))
  return router
}

const update = async (id: string, body: UpdateToolRouterRequest): Promise<ToolRouter> => {
  const router = await api.toolRouters.update(id, body)
  store.setState((s) => ({ items: nmSet(s.items, router.id, router) }))
  return router
}

const remove = async (id: string): Promise<void> => {
  const prev = store.getState()
  store.setState((s) => ({ items: nmDelete(s.items, id) }))
  try {
    await api.toolRouters.delete(id)
  } catch (e) {
    store.setState({ items: prev.items, error: extractError(e) })
    throw e
  }
}

// ── Sub-resource: Router Tools ───────────────────────────────────────────────

const fetchRouterTools = async (routerId: string): Promise<Tool[]> => {
  const tools = await api.toolRouters.getTools(routerId)
  store.setState((s) => ({
    toolsByRouter: { ...s.toolsByRouter, [routerId]: tools },
  }))
  return tools
}

const setRouterTools = async (routerId: string, body: SetRouterToolsRequest): Promise<void> => {
  await api.toolRouters.setTools(routerId, body)
  await fetchRouterTools(routerId)
}

// ── Sub-resource: Modes ──────────────────────────────────────────────────────

const fetchModes = async (routerId: string): Promise<RouterMode[]> => {
  const modes = await api.routerModes.listByRouter(routerId)
  store.setState((s) => ({
    modesByRouter: { ...s.modesByRouter, [routerId]: modes },
  }))
  return modes
}

const createMode = async (routerId: string, body: CreateRouterModeRequest): Promise<RouterMode> => {
  const mode = await api.routerModes.createForRouter(routerId, body)
  store.setState((s) => ({
    modesByRouter: {
      ...s.modesByRouter,
      [routerId]: [...(s.modesByRouter[routerId] ?? []), mode],
    },
  }))
  return mode
}

const updateMode = async (modeId: string, body: UpdateRouterModeRequest): Promise<RouterMode> => {
  const mode = await api.routerModes.update(modeId, body)
  store.setState((s) => {
    const routerId = mode.router_id
    const current = s.modesByRouter[routerId] ?? []
    return {
      modesByRouter: {
        ...s.modesByRouter,
        [routerId]: current.map((m) => (m.id === modeId ? mode : m)),
      },
    }
  })
  return mode
}

const deleteMode = async (modeId: string): Promise<void> => {
  // Find which router owns this mode before deleting
  const state = store.getState()
  let ownerRouterId: string | null = null
  for (const [routerId, modes] of Object.entries(state.modesByRouter)) {
    if (modes.some((m) => m.id === modeId)) {
      ownerRouterId = routerId
      break
    }
  }

  await api.routerModes.delete(modeId)

  if (ownerRouterId) {
    store.setState((s) => ({
      modesByRouter: {
        ...s.modesByRouter,
        [ownerRouterId]: (s.modesByRouter[ownerRouterId] ?? []).filter((m) => m.id !== modeId),
      },
    }))
  }
}

// ── Sub-resource: Mode Tools ─────────────────────────────────────────────────

const fetchModeTools = async (modeId: string): Promise<Tool[]> => {
  const tools = await api.routerModes.getTools(modeId)
  store.setState((s) => ({
    toolsByMode: { ...s.toolsByMode, [modeId]: tools },
  }))
  return tools
}

const setModeTools = async (modeId: string, body: SetModeToolsRequest): Promise<void> => {
  await api.routerModes.setTools(modeId, body)
  await fetchModeTools(modeId)
}

// ── Export ────────────────────────────────────────────────────────────────────

export const toolRouterStore = {
  store,
  selectAll,
  selectById,
  selectRouterTools,
  selectModes,
  selectModeTools,
  selectLoading,
  selectError,
  fetchAll,
  fetchOne,
  create,
  update,
  remove,
  fetchRouterTools,
  setRouterTools,
  fetchModes,
  createMode,
  updateMode,
  deleteMode,
  fetchModeTools,
  setModeTools,
}

export type { ToolRouterState }
