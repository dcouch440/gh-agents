import { useEffect, useRef } from 'react'
import { api } from '@/api'
import { dispatchStore } from '@/stores/dispatchStore'
import type { BoardDispatchInfo } from '@/types/board'

const POLL_INTERVAL_MS = 2000

const TERMINAL_STATUSES = new Set(['completed', 'failed', 'cancelled'])

/**
 * Poll `GET /dispatch/:executionId/trace` for every active dispatch.
 *
 * Hydrates dispatchStore with each REST response. Stops polling individual
 * dispatches when they reach a terminal status. Cleans up all timers on
 * unmount or when the dispatches array changes (new submit).
 */
const useDispatchPollAll = (dispatches: readonly BoardDispatchInfo[]): void => {
  const timersRef = useRef(new Map<string, ReturnType<typeof setInterval>>())
  const stoppedRef = useRef(new Set<string>())

  useEffect(() => {
    const timers = timersRef.current
    const stopped = stoppedRef.current

    // Clear previous timers
    for (const timer of timers.values()) {
      clearInterval(timer)
    }
    timers.clear()
    stopped.clear()

    if (dispatches.length === 0) return

    for (const dispatch of dispatches) {
      const { execution_id } = dispatch

      const poll = async () => {
        if (stopped.has(execution_id)) return
        try {
          const resp = await api.dispatch.trace(execution_id)
          dispatchStore.hydrateFromApi(resp)

          if (TERMINAL_STATUSES.has(resp.status)) {
            stopped.add(execution_id)
            const timer = timers.get(execution_id)
            if (timer !== undefined) {
              clearInterval(timer)
              timers.delete(execution_id)
            }
          }
        } catch {
          // Silently ignore poll failures — WS is primary, REST is fallback
        }
      }

      // Initial poll immediately
      void poll()
      timers.set(execution_id, setInterval(() => void poll(), POLL_INTERVAL_MS))
    }

    return () => {
      for (const timer of timers.values()) {
        clearInterval(timer)
      }
      timers.clear()
      stopped.clear()
    }
  }, [dispatches])
}

export { useDispatchPollAll }
