import { api } from '@/api'
import type { RouterMode, CreateRouterModeRequest, UpdateRouterModeRequest, SetModeToolsRequest } from '@/types/router'
import type { Tool } from '@/types/tool'
import { store } from './_store'

// ── Mode CRUD ───────────────────────────────────────────────────────────────

const fetchModes = async (routerId: string): Promise<RouterMode[]> => {
  const modes = await api.routerModes.listByRouter(routerId)

  // Build reverse lookup entries for O(1) owner resolution
  const lookup: Record<string, string> = {}
  for (const mode of modes) lookup[mode.id] = routerId

  store.setState((s) => ({
    modesByRouter: { ...s.modesByRouter, [routerId]: modes },
    modeToRouter: { ...s.modeToRouter, ...lookup },
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
    modeToRouter: { ...s.modeToRouter, [mode.id]: routerId },
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
  // O(1) owner resolution via reverse lookup
  const ownerRouterId = store.getState().modeToRouter[modeId] ?? null

  await api.routerModes.delete(modeId)

  if (ownerRouterId) {
    store.setState((s) => {
      const nextModeToRouter = Object.fromEntries(
        Object.entries(s.modeToRouter).filter(([k]) => k !== modeId),
      )
      return {
        modesByRouter: {
          ...s.modesByRouter,
          [ownerRouterId]: (s.modesByRouter[ownerRouterId] ?? []).filter((m) => m.id !== modeId),
        },
        modeToRouter: nextModeToRouter,
      }
    })
  }
}

// ── Mode Tools ──────────────────────────────────────────────────────────────

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

export { fetchModes, createMode, updateMode, deleteMode, fetchModeTools, setModeTools }
