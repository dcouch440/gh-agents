import { createStore, logger } from '../lib'
import type { BoardState } from './types'

const INITIAL_STATE: BoardState = {
  status: 'idle',
  error: null,
  lastResponse: null,
  isFirstSubmit: false,
  elementStepMap: {},
  elementEdgeMap: {},
}

const store = logger(
  'boardStore',
  createStore<BoardState>(() => INITIAL_STATE),
)

export { store, INITIAL_STATE }
