import { createStore, createNormalizedMap } from '../lib'
import type { ToolRouterState } from './types'

const store = createStore<ToolRouterState>(() => ({
  items: createNormalizedMap(),
  toolsByRouter: {},
  modesByRouter: {},
  toolsByMode: {},
  modeToRouter: {},
  loading: false,
  error: null,
}))

export { store }
