// ============================================================================
// bridge/useFlowSync — React Flow events → store actions
// ============================================================================

import { useCallback } from 'react'
import type { OnNodeDrag, OnConnect, OnSelectionChangeFunc } from '@xyflow/react'
import { workflowStore } from '@/stores/workflowStore'
import { canvasStore } from '@/stores/canvasStore'
import { nodeToPositionUpdate } from './transforms'
import type { StepNode, StepEdge } from './types'

type FlowSyncCallbacks = {
  onNodeDragStop: OnNodeDrag<StepNode>
  onConnect: OnConnect
  onSelectionChange: OnSelectionChangeFunc<StepNode, StepEdge>
  onNodeMouseEnter: (_event: React.MouseEvent, node: StepNode) => void
  onNodeMouseLeave: () => void
  onEdgeMouseEnter: (_event: React.MouseEvent, edge: StepEdge) => void
  onEdgeMouseLeave: () => void
}

const useFlowSync = (): FlowSyncCallbacks => {
  const onNodeDragStop: OnNodeDrag<StepNode> = useCallback((_event, node) => {
    const update = nodeToPositionUpdate(node)
    workflowStore.updateStep(node.id, update).catch(() => {
      // Position update failed — node will snap back on next store read
    })
  }, [])

  const onConnect: OnConnect = useCallback((connection) => {
    if (!connection.source || !connection.target) return
    workflowStore.addEdge({
      from_step_id: connection.source,
      to_step_id: connection.target,
    }).catch(() => {
      // Edge creation failed silently — UI will not show edge
    })
  }, [])

  const onSelectionChange: OnSelectionChangeFunc<StepNode, StepEdge> = useCallback((params) => {
    canvasStore.selectSteps(params.nodes.map((n) => n.id))
    canvasStore.selectEdges(params.edges.map((e) => e.id))
  }, [])

  const onNodeMouseEnter = useCallback((_event: React.MouseEvent, node: StepNode) => {
    canvasStore.setHoveredStep(node.id)
  }, [])

  const onNodeMouseLeave = useCallback(() => {
    canvasStore.setHoveredStep(null)
  }, [])

  const onEdgeMouseEnter = useCallback((_event: React.MouseEvent, edge: StepEdge) => {
    canvasStore.setHoveredEdge(edge.id)
  }, [])

  const onEdgeMouseLeave = useCallback(() => {
    canvasStore.setHoveredEdge(null)
  }, [])

  return {
    onNodeDragStop,
    onConnect,
    onSelectionChange,
    onNodeMouseEnter,
    onNodeMouseLeave,
    onEdgeMouseEnter,
    onEdgeMouseLeave,
  }
}

export { useFlowSync }
export type { FlowSyncCallbacks }
