import { useCallback } from 'react'
import { useStore } from '@/stores/lib'
import { contextPickerStore } from '@/stores/contextPickerStore'
import type { PickableEntity } from '@/stores/contextPickerStore'

type UsePickableReturn = {
  isPickingActive: boolean
  onPick: () => void
}

const usePickable = (entity: PickableEntity | null): UsePickableReturn => {
  const isPickingActive = useStore(contextPickerStore.store, contextPickerStore.selectActive)

  const onPick = useCallback(() => {
    if (!isPickingActive || !entity) return
    contextPickerStore.pick(entity)
  }, [isPickingActive, entity])

  return { isPickingActive, onPick }
}

export { usePickable }
export type { UsePickableReturn }
