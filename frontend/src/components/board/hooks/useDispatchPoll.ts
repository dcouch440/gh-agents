import { useEffect, useRef } from 'react'
import { api } from '@/api'
import { dispatchStore } from '@/stores/dispatchStore'

const POLL_INTERVAL_MS = 2000

/**
 * Poll `GET /dispatch/:executionId/trace` while the dispatch is running.
 *
 * Hydrates dispatchStore with the REST response, which supplements or
 * replaces data received via WebSocket. Stops polling when the dispatch
 * reaches a terminal status or when executionId becomes null.
 */
const useDispatchPoll = (executionId: string | null): void => {
  const timerRef = useRef<ReturnType<typeof setInterval> | null>(null)
  const stoppedRef = useRef(false)

  useEffect(() => {
    stoppedRef.current = false

    if (timerRef.current !== null) {
      clearInterval(timerRef.current)
      timerRef.current = null
    }

    if (executionId === null) return

    const poll = async () => {
      if (stoppedRef.current) return
      try {
        const resp = await api.dispatch.trace(executionId)
        dispatchStore.hydrateFromApi(resp)

        // Stop polling on terminal status
        if (resp.status === 'completed' || resp.status === 'failed' || resp.status === 'cancelled') {
          stoppedRef.current = true
          if (timerRef.current !== null) {
            clearInterval(timerRef.current)
            timerRef.current = null
          }
        }
      } catch {
        // Silently ignore poll failures — WS is primary, REST is fallback
      }
    }

    // Initial poll immediately
    void poll()
    timerRef.current = setInterval(() => void poll(), POLL_INTERVAL_MS)

    return () => {
      stoppedRef.current = true
      if (timerRef.current !== null) {
        clearInterval(timerRef.current)
        timerRef.current = null
      }
    }
  }, [executionId])
}

export { useDispatchPoll }
