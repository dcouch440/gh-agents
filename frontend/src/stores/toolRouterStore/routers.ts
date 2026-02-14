import { nmFromArray, nmSet, nmDelete, extractError } from '../lib'
import { api } from '@/api'
import type { CreateToolRouterRequest, UpdateToolRouterRequest, SetRouterToolsRequest, ToolRouter } from '@/types/toolRouter'
import type { Tool } from '@/types/tool'
import { store } from './_store'

// ── Router CRUD ─────────────────────────────────────────────────────────────

const fetchAll = async (): Promise<void> => {
  store.setState({ loading: true, error: null })
  try {
    const data = await api.toolRouters.list()
    store.setState({ items: nmFromArray(data), loading: false })
  } catch (e) {
    store.setState({ loading: false, error: extractError('toolRouters', e) })
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
    store.setState({ items: prev.items, error: extractError('toolRouters', e) })
    throw e
  }
}

// ── Router Tools ────────────────────────────────────────────────────────────

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

export { fetchAll, fetchOne, create, update, remove, fetchRouterTools, setRouterTools }
