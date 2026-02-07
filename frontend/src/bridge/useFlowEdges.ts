// ============================================================================
// bridge/useFlowEdges — Store → React Flow edges (memoized)
// ============================================================================

import { useMemo } from 'react'
import { useStore } from '@/stores/lib'
import { workflowStore } from '@/stores/workflowStore'
import { canvasStore } from '@/stores/canvasStore'
import { edgeToFlowEdge } from './transforms'
import type { StepEdge } from './types'

const useFlowEdges = (): StepEdge[] => {
  const edges = useStore(workflowStore.store, workflowStore.selectEdges)
  const selectedIds = useStore(canvasStore.store, canvasStore.selectSelectedEdgeIds)
  const hoveredId = useStore(canvasStore.store, canvasStore.selectHoveredEdgeId)

  return useMemo(
    () =>
      edges.map((edge) =>
        edgeToFlowEdge(
          edge,
          selectedIds.has(edge.id),
          hoveredId === edge.id,
        ),
      ),
    [edges, selectedIds, hoveredId],
  )
}

export { useFlowEdges }
