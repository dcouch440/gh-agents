import { createStore } from '../lib'
import type { AgentTraceState } from './types'

const initialState: AgentTraceState = {
  traces: {},
  order: [],
  hydratedRunId: null,
}

const store = createStore<AgentTraceState>(() => ({ ...initialState }))

const reset = (): void => {
  store.setState({ ...initialState })
}

export { store, initialState, reset }
