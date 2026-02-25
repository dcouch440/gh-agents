import { useEffect, useState } from 'react'
import { api } from '@/api'
import type { BoardElements } from '../elements'
import { deserializeFromExcalidraw } from '../elements'

type SetElements = (fn: (s: BoardElements) => BoardElements) => void

/**
 * Fetch saved board elements from the backend and deserialize into BoardElements.
 *
 * Returns `loading: true` until the fetch completes. On success, calls
 * `setElements` with the deserialized board state. The backend returns
 * the same Excalidraw JSON array that was last POSTed, so
 * `deserializeFromExcalidraw` reconstructs the internal representation.
 */
const useBoardElements = (workflowId: string, setElements: SetElements): { loading: boolean } => {
  const [loading, setLoading] = useState(true)

  useEffect(() => {
    const controller = new AbortController()
    const { signal } = controller

    api.workflows.getBoardElements(workflowId, { signal }).then(
      (resp) => {
        if (!signal.aborted) {
          if (resp.elements !== null) {
            const board = deserializeFromExcalidraw(resp.elements as Record<string, unknown>[])
            setElements(() => board)
          }
          setLoading(false)
        }
      },
      () => {
        if (!signal.aborted) {
          setLoading(false)
        }
      },
    )

    return () => { controller.abort() }
  }, [workflowId, setElements])

  return { loading }
}

export { useBoardElements }
