import { store } from './_store'
import { selectAll, selectById, selectRouterTools, selectModes, selectModeTools, selectLoading, selectError } from './selectors'
import { fetchAll, fetchOne, create, update, remove, fetchRouterTools, setRouterTools } from './routers'
import { fetchModes, createMode, updateMode, deleteMode, fetchModeTools, setModeTools } from './modes'

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

export type { ToolRouterState } from './types'
