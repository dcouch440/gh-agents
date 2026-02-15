import { useRef, useCallback, useEffect } from 'react'

type DebouncedHandle<T> = {
  /** Schedule the callback after the debounce delay, resetting any pending timer. */
  call: (value: T) => void
  /** Fire the pending callback immediately. No-op if nothing is pending. */
  flush: () => void
  /** Cancel the pending callback without firing. */
  cancel: () => void
}

type Options = {
  /** When true, fires (rather than cancels) the pending callback on unmount. Default: false. */
  flushOnUnmount?: boolean
}

const useDebounceCallback = <T>(
  callback: (value: T) => void,
  delayMs: number,
  options?: Options,
): DebouncedHandle<T> => {
  const callbackRef = useRef(callback)
  const timeoutRef = useRef<ReturnType<typeof setTimeout> | null>(null)
  const lastValueRef = useRef<T | null>(null)
  const hasPendingRef = useRef(false)

  useEffect(() => {
    callbackRef.current = callback
  })

  const clearPending = useCallback(() => {
    if (timeoutRef.current !== null) {
      clearTimeout(timeoutRef.current)
      timeoutRef.current = null
    }
    hasPendingRef.current = false
  }, [])

  const flush = useCallback(() => {
    if (!hasPendingRef.current) return
    clearPending()
    callbackRef.current(lastValueRef.current as T)
  }, [clearPending])

  const cancel = useCallback(() => {
    clearPending()
    lastValueRef.current = null
  }, [clearPending])

  const call = useCallback(
    (value: T) => {
      if (timeoutRef.current !== null) {
        clearTimeout(timeoutRef.current)
      }
      lastValueRef.current = value
      hasPendingRef.current = true
      timeoutRef.current = setTimeout(() => {
        timeoutRef.current = null
        hasPendingRef.current = false
        callbackRef.current(value)
      }, delayMs)
    },
    [delayMs],
  )

  useEffect(() => {
    return () => {
      if (options?.flushOnUnmount && hasPendingRef.current) {
        callbackRef.current(lastValueRef.current as T)
      }
      if (timeoutRef.current !== null) {
        clearTimeout(timeoutRef.current)
      }
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps -- options is read via ref-like pattern at cleanup time
  }, [])

  return { call, flush, cancel }
}

export { useDebounceCallback }
export type { DebouncedHandle }
