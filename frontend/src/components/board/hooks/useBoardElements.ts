import { useEffect, useState } from 'react'
import { api } from '@/api'

type BoardElementsState = {
  loading: boolean
  elements: readonly Record<string, unknown>[] | null
}

/**
 * Fetch saved Excalidraw elements for a workflow from the backend.
 *
 * Returns `loading: true` until the fetch completes, then `elements`
 * is either the saved array or `null` (no previous submit).
 * Excalidraw should only mount after loading is false, so that
 * `initialData.elements` is set on first render.
 */
const useBoardElements = (workflowId: string): BoardElementsState => {
  const [state, setState] = useState<BoardElementsState>({ loading: true, elements: null })

  useEffect(() => {
    const controller = new AbortController()
    const { signal } = controller

    api.workflows.getBoardElements(workflowId, { signal }).then(
      (resp) => {
        if (!signal.aborted) {
          setState({ loading: false, elements: resp.elements })
        }
      },
      () => {
        if (!signal.aborted) {
          setState({ loading: false, elements: null })
        }
      },
    )

    return () => { controller.abort() }
  }, [workflowId])

  return state
}

export { useBoardElements }
