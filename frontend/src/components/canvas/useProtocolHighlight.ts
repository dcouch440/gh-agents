import { useStore, canvasStore } from '@/stores'

type HighlightMode = 'none' | 'hover' | 'select'

const useProtocolHighlight = (protocolStepId: string | null): HighlightMode => {
  const mode = useStore(canvasStore.store, (s): HighlightMode => {
    if (protocolStepId === null) return 'none'
    if (s.selectedStepIds.has(protocolStepId)) return 'select'
    if (s.hoveredStepId === protocolStepId) return 'hover'
    return 'none'
  })
  return mode
}

export { useProtocolHighlight }
export type { HighlightMode }
