import { useCallback, useEffect, useRef } from 'react'
import type { Node } from '@xyflow/react'
import { workflowStore } from '@/stores'
import { computeAutoLayout } from './layout'
import { isVirtualNode, setStoredPosition } from './nodeResizeStorage'
import type { StepNodeLookups } from './mappers/types'
import type { WorkflowStep, WorkflowStepEdge } from '@/types/workflow'

type UseAutoLayoutResult = {
  applyAutoLayout: () => void
}

const useAutoLayout = (
  steps: readonly WorkflowStep[],
  edges: readonly WorkflowStepEdge[],
  lookups: StepNodeLookups,
  getNodes: () => Node[],
  setNodes: (updater: (nodes: Node[]) => Node[]) => void,
  fitView: (options?: { padding?: number }) => void,
): UseAutoLayoutResult => {
  const initialLayoutApplied = useRef(false)

  const applyAutoLayout = useCallback(() => {
    const positions = computeAutoLayout(steps, edges, lookups)
    if (positions.size === 0) return

    // Apply positions to React Flow nodes
    setNodes((current) =>
      current.map((node) => {
        const pos = positions.get(node.id)
        if (!pos) return node
        return { ...node, position: { x: pos.x, y: pos.y } }
      }),
    )

    // Persist positions
    for (const [nodeId, pos] of positions) {
      const roundedPos = { x: Math.round(pos.x), y: Math.round(pos.y) }
      if (isVirtualNode(nodeId)) {
        setStoredPosition(nodeId, roundedPos)
      } else {
        void workflowStore.updateStep(nodeId, {
          position_x: roundedPos.x,
          position_y: roundedPos.y,
        })
      }
    }

    // Fit view after layout
    setTimeout(() => {
      fitView({ padding: 0.15 })
    }, 50)
  }, [steps, edges, lookups, setNodes, fitView])

  // Auto-apply layout on first load if positions look unset
  useEffect(() => {
    if (initialLayoutApplied.current) return
    if (steps.length === 0) return

    const hasPositions = steps.some((s) => s.position_x !== null && s.position_x !== 0)
    if (hasPositions) {
      initialLayoutApplied.current = true
      return
    }

    initialLayoutApplied.current = true
    // Defer to next frame so RF nodes are mounted
    requestAnimationFrame(() => {
      applyAutoLayout()
    })
  }, [steps, applyAutoLayout])

  return { applyAutoLayout }
}

export { useAutoLayout }
