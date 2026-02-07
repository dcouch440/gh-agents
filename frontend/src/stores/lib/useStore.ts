// ============================================================================
// useStore — React Hook with Selector-Based Subscriptions
// ============================================================================

import { useSyncExternalStore, useCallback, useRef, useEffect } from 'react'
import type { StoreApi } from './types'

const useStore = <T, S>(
  store: StoreApi<T>,
  selector: (state: T) => S,
  equalityFn: (a: S, b: S) => boolean = Object.is,
): S => {
  const selectorRef = useRef(selector)
  const equalityRef = useRef(equalityFn)
  const selectedRef = useRef<S>(selector(store.getState()))

  useEffect(() => {
    selectorRef.current = selector
    equalityRef.current = equalityFn
  })

  const getSnapshot = useCallback(() => {
    const next = selectorRef.current(store.getState())
    if (equalityRef.current(selectedRef.current, next)) {
      return selectedRef.current
    }
    selectedRef.current = next
    return next
  }, [store])

  return useSyncExternalStore(store.subscribe, getSnapshot)
}

export { useStore }
