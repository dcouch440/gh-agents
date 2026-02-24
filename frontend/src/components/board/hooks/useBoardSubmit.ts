import { useState, useCallback } from 'react'
import { useStore, boardStore } from '@/stores'
import type { ExcalidrawImperativeAPI } from '@excalidraw/excalidraw/types'
import type { SubmitStatus } from '@/stores/boardStore'

/**
 * Bridge between Excalidraw and boardStore.
 *
 * Holds the `ExcalidrawImperativeAPI` in local state (via the callback prop
 * pattern that Excalidraw v0.17+ requires) and exposes a `handleSubmit` that
 * reads scene elements and dispatches them through `boardStore.submitBoard`.
 *
 * `useState` rather than `useRef` for the API so the component re-renders
 * once when Excalidraw initialises — this lets the submit button's disabled
 * state react to the API becoming available.
 *
 * @param workflowId - The active workflow UUID (from route params).
 */
const useBoardSubmit = (workflowId: string) => {
  const [excalidrawApi, setExcalidrawApi] = useState<ExcalidrawImperativeAPI | null>(null)
  const isSubmitting = useStore(boardStore.store, boardStore.selectIsSubmitting)
  const error = useStore(boardStore.store, boardStore.selectError)
  const status: SubmitStatus = useStore(boardStore.store, boardStore.selectStatus)

  /**
   * Read current elements from Excalidraw and POST to the backend.
   * No-op if the API is not ready or a submit is already in flight.
   *
   * `getSceneElements()` returns Excalidraw's internal objects that may be
   * frozen, proxied, or carry non-enumerable properties. `Array.from(raw)`
   * creates a fresh standard Array via the iterator protocol. Each element
   * is shallow-spread to a plain object so `JSON.stringify` in the HTTP
   * client produces a clean JSON array for the backend.
   */
  const handleSubmit = useCallback(() => {
    if (!excalidrawApi || isSubmitting) return
    const raw = excalidrawApi.getSceneElements()
    // Excalidraw elements may be frozen/proxied — shallow-spread each to a
    // plain object so JSON.stringify in the HTTP client produces a clean array.
    // The `as Record<string, unknown>` cast is necessary because Excalidraw's
    // element types use branded/opaque fields that TypeScript sees as `any`.
    const elements = Array.isArray(raw)
      ? Array.from(raw, (el) => ({ ...(el as Record<string, unknown>) }))
      : []

    if (import.meta.env.DEV) {
      console.warn('[board] submitting', elements.length, 'elements')
    }

    void boardStore.submitBoard(workflowId, elements)
  }, [excalidrawApi, isSubmitting, workflowId])

  return { setExcalidrawApi, handleSubmit, isSubmitting, error, status } as const
}

export { useBoardSubmit }
