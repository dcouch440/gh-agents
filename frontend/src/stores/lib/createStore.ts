// ============================================================================
// Store Factory
// ============================================================================

import { scheduleBatchNotify } from './batch'
import type { Listener, SetState, StoreApi, StateCreator } from './types'

const createStore = <T>(creator: StateCreator<T>): StoreApi<T> => {
  let state: T
  const listeners = new Set<Listener>()

  const getState = (): T => state

  const setState: SetState<T> = (partial: Partial<T> | ((s: T) => Partial<T>)) => {
    const nextPartial = typeof partial === 'function' ? (partial as (s: T) => Partial<T>)(state) : partial

    state = Object.assign({}, state, nextPartial)

    const notify = () => {
      for (const listener of listeners) {
        listener()
      }
    }
    scheduleBatchNotify(notify)
  }

  const subscribe = (listener: Listener): (() => void) => {
    listeners.add(listener)
    return () => {
      listeners.delete(listener)
    }
  }

  const destroy = (): void => {
    listeners.clear()
  }

  state = creator(setState, getState)

  return { getState, setState, subscribe, destroy }
}

export { createStore }
