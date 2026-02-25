import { useCallback } from 'react'
import { useStore, boardStore } from '@/stores'
import type { SubmitStatus } from '@/stores/boardStore'
import type { BoardElements } from '../elements'
import { serializeToExcalidraw } from '../elements'

/**
 * Bridge between internal BoardElements and boardStore.
 *
 * Serializes internal elements to Excalidraw JSON format on submit,
 * then delegates to `boardStore.submitBoard` which POSTs to the backend.
 * The boardStore and backend remain completely unchanged.
 *
 * @param workflowId - The active workflow UUID (from route params).
 * @param elements - Current board elements state.
 */
const useBoardSubmit = (workflowId: string, elements: BoardElements) => {
  const isSubmitting = useStore(boardStore.store, boardStore.selectIsSubmitting)
  const error = useStore(boardStore.store, boardStore.selectError)
  const status: SubmitStatus = useStore(boardStore.store, boardStore.selectStatus)

  const handleSubmit = useCallback(() => {
    if (isSubmitting) return

    const excalidrawElements = serializeToExcalidraw(elements)

    if (import.meta.env.DEV) {
      console.warn('[board] submitting', excalidrawElements.length, 'elements')
    }

    void boardStore.submitBoard(workflowId, excalidrawElements)
  }, [elements, isSubmitting, workflowId])

  return { handleSubmit, isSubmitting, error, status } as const
}

export { useBoardSubmit }
