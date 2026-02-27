import { useCallback } from 'react'
import { useStore, boardStore } from '@/stores'
import { boardElementStore } from '@/stores/boardElementStore'
import { undoStore } from '@/stores/undoStore'
import type { SubmitStatus } from '@/stores/boardStore'
import { serializeToExcalidraw } from '../elements'

/**
 * Bridge between boardElementStore and boardStore.
 *
 * Reads current elements from boardElementStore, serializes to Excalidraw
 * JSON format on submit, then delegates to `boardStore.submitBoard` which
 * POSTs to the backend. Clears the undo stack after successful submit.
 *
 * @param workflowId - The active workflow UUID (from route params).
 */
const useBoardSubmit = (workflowId: string) => {
  const isSubmitting = useStore(boardStore.store, boardStore.selectIsSubmitting)
  const error = useStore(boardStore.store, boardStore.selectError)
  const status: SubmitStatus = useStore(boardStore.store, boardStore.selectStatus)

  const handleSubmit = useCallback(() => {
    if (isSubmitting) return

    const elements = boardElementStore.getElements()
    const excalidrawElements = serializeToExcalidraw(elements)

    if (import.meta.env.DEV) {
      console.warn('[board] submitting', excalidrawElements.length, 'elements')
    }

    void boardStore.submitBoard(workflowId, excalidrawElements).then(() => {
      undoStore.clear()
    })
  }, [isSubmitting, workflowId])

  return { handleSubmit, isSubmitting, error, status } as const
}

export { useBoardSubmit }
