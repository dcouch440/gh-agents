import { useEffect, useState } from 'react'
import { api } from '@/api'
import { boardStore } from '@/stores'
import { boardElementStore } from '@/stores/boardElementStore'
import { undoStore } from '@/stores/undoStore'
import { mergeElementStepMap, mergeElementEdgeMap } from '@/stores/boardStore/submit'
import { deserializeFromExcalidraw, emptyBoard } from '../elements'

/**
 * Fetch saved board elements from the backend and deserialize into boardElementStore.
 *
 * Returns `loading: true` until the fetch completes. On success, writes the
 * deserialized board state to `boardElementStore`. The backend returns
 * the same Excalidraw JSON array that was last POSTed, so
 * `deserializeFromExcalidraw` reconstructs the internal representation.
 *
 * Also hydrates boardStore.lastResponse from the persisted last submit
 * response, so the debug panel shows Phase 0 results on page refresh.
 *
 * Resets boardElementStore and undoStore on workflow change so stale state
 * from a previous workflow doesn't bleed through.
 *
 * Uses a boolean `cancelled` flag instead of AbortController to avoid a race
 * condition with the API client's request deduplication under React StrictMode.
 * StrictMode double-mounts: if mount 1 aborts its signal, the cached promise
 * rejects, and mount 2 (reusing the cache) gets the rejected promise too.
 * A boolean flag lets mount 1's request complete normally so mount 2's dedup
 * hit returns the successful result.
 */
const useBoardElements = (workflowId: string): { loading: boolean } => {
  const [loading, setLoading] = useState(true)

  useEffect(() => {
    let cancelled = false

    // Reset to empty on workflow change
    boardElementStore.replaceElements(emptyBoard())
    undoStore.clear()

    api.workflows.getBoardElements(workflowId).then(
      (resp) => {
        if (!cancelled) {
          if (resp.elements !== null) {
            const board = deserializeFromExcalidraw(resp.elements as Record<string, unknown>[])
            boardElementStore.replaceElements(board)
          }

          // Hydrate boardStore from persisted last submit response
          if (resp.last_submit !== null) {
            boardStore.store.setState({
              status: 'success',
              lastResponse: resp.last_submit,
              isFirstSubmit: resp.last_submit.is_first_submit,
              elementStepMap: mergeElementStepMap({}, resp.last_submit.phase_zero),
              elementEdgeMap: mergeElementEdgeMap({}, resp.last_submit.phase_zero),
            })
          }

          setLoading(false)
        }
      },
      () => {
        if (!cancelled) {
          setLoading(false)
        }
      },
    )

    return () => { cancelled = true }
  }, [workflowId])

  return { loading }
}

export { useBoardElements }
