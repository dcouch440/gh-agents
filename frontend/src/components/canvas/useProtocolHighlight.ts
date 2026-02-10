import { useStore, canvasStore } from '@/stores'
import type { CanvasState } from '@/stores'
import type { CanvasNodeKind } from './canvasKinds'
import { HighlightMode, HOVER_ELIGIBLE_KINDS } from './canvasKinds'

const hoverStateManager = (nodeKind: CanvasNodeKind, nodeId: string, protocolStepId: string | null) =>
  (s: CanvasState): HighlightMode => {
    switch (true) {
      case protocolStepId === null:
        return HighlightMode.NONE
      case s.selectedStepIds.has(protocolStepId!) || s.highlightedProtocolStepIds.has(protocolStepId!):
        return HighlightMode.SELECT
      case s.hoveredProtocolId === protocolStepId:
        if (HOVER_ELIGIBLE_KINDS.has(nodeKind)) return HighlightMode.HOVER
        return HighlightMode.NONE
      case s.hoveredStepId === nodeId:
        if (HOVER_ELIGIBLE_KINDS.has(nodeKind)) return HighlightMode.HOVER
        return HighlightMode.NONE
      default:
        return HighlightMode.NONE
    }
  }

const useProtocolHighlight = (nodeKind: CanvasNodeKind, nodeId: string, protocolStepId: string | null): HighlightMode => {
  const mode = useStore(canvasStore.store, hoverStateManager(nodeKind, nodeId, protocolStepId))
  return mode
}

export { useProtocolHighlight, hoverStateManager }
