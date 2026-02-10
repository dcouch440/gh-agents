import { useRef, useCallback, useEffect } from 'react'

const useDebounceCallback = <T>(callback: (value: T) => void, delayMs: number): ((value: T) => void) => {
  const callbackRef = useRef(callback)

  useEffect(() => {
    callbackRef.current = callback
  })

  const timeoutRef = useRef<ReturnType<typeof setTimeout> | null>(null)

  useEffect(() => {
    return () => {
      if (timeoutRef.current !== null) {
        clearTimeout(timeoutRef.current)
      }
    }
  }, [])

  return useCallback(
    (value: T) => {
      if (timeoutRef.current !== null) {
        clearTimeout(timeoutRef.current)
      }
      timeoutRef.current = setTimeout(() => {
        callbackRef.current(value)
        timeoutRef.current = null
      }, delayMs)
    },
    [delayMs],
  )
}

export { useDebounceCallback }
