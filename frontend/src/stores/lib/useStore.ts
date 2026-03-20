// ============================================================================
// useStore — React Hook with Selector-Based Subscriptions
// ============================================================================

import { useSyncExternalStore, useCallback, useRef } from 'react'
import type { StoreApi } from './types'

const useStore = <T, S>(store: StoreApi<T>, selector: (state: T) => S, equalityFn: (a: S, b: S) => boolean = Object.is): S => {
  const cacheRef = useRef({ state: store.getState(), selected: selector(store.getState()) })

  // getSnapshot must return a cached result when the store state hasn't changed.
  // React 19 calls getSnapshot multiple times during render to verify stability;
  // selectors that return new references (e.g. .filter(), ?? []) would otherwise
  // produce different objects on each call, triggering an infinite loop.
  const getSnapshot = useCallback(() => {
    const state = store.getState()

    // Store state unchanged — return exact same reference (React verification safe)
    if (state === cacheRef.current.state) {
      return cacheRef.current.selected
    }

    // Store state changed — re-run selector
    const next = selector(state)
    if (equalityFn(cacheRef.current.selected, next)) {
      cacheRef.current.state = state
      return cacheRef.current.selected
    }
    cacheRef.current = { state, selected: next }
    return next
  }, [store, selector, equalityFn])

  return useSyncExternalStore(store.subscribe, getSnapshot)
}

export { useStore }
