import { useState, useCallback } from 'react'
import { workflowStore } from '@/stores'
import type { CreateDocumentDefRequest } from '@/types/workflow'

type DocumentActions = {
  adding: boolean
  onAdd: () => void
  onSubmitNew: (body: CreateDocumentDefRequest) => void
  onCancelAdd: () => void
  onRemove: (defId: string) => void
}

const useDocumentActions = (stepId: string): DocumentActions => {
  const [adding, setAdding] = useState(false)

  const onAdd = useCallback(() => {
    setAdding(true)
  }, [])

  const onSubmitNew = useCallback(
    (body: CreateDocumentDefRequest) => {
      void workflowStore.createDocumentDef(stepId, body)
      setAdding(false)
    },
    [stepId],
  )

  const onCancelAdd = useCallback(() => {
    setAdding(false)
  }, [])

  const onRemove = useCallback(
    (defId: string) => {
      void workflowStore.deleteDocumentDef(stepId, defId)
    },
    [stepId],
  )

  return { adding, onAdd, onSubmitNew, onCancelAdd, onRemove }
}

export { useDocumentActions }
export type { DocumentActions }
