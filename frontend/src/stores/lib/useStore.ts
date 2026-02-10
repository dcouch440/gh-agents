// ============================================================================
// useStore — React Hook with Selector-Based Subscriptions
// ============================================================================

import { useSyncExternalStore, useCallback, useRef } from 'react'
import type { StoreApi } from './types'

const useStore = <T, S>(store: StoreApi<T>, selector: (state: T) => S, equalityFn: (a: S, b: S) => boolean = Object.is): S => {
  const selectedRef = useRef<S>(selector(store.getState()))

  // getSnapshot captures selector directly in its closure, so it always uses
  // the latest selector — fixing stale-selector bugs with dynamic selectors
  // like selectStepById(id). For static selectors the deps are stable and
  // getSnapshot identity doesn't change.
  const getSnapshot = useCallback(() => {
    const next = selector(store.getState())
    if (equalityFn(selectedRef.current, next)) {
      return selectedRef.current
    }
    selectedRef.current = next
    return next
  }, [store, selector, equalityFn])

  return useSyncExternalStore(store.subscribe, getSnapshot)
}

export { useStore }
