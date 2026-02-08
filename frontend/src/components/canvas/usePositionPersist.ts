import { useRef, useCallback } from 'react'
import type { Node } from '@xyflow/react'
import { workflowStore } from '@/stores'

const usePositionPersist = () => {
  const pendingRef = useRef<Map<string, { position_x: number; position_y: number }>>(new Map())

  const flush = useCallback(() => {
    const pending = pendingRef.current
    if (pending.size === 0) return
    const entries = [...pending.entries()]
    pending.clear()
    for (const [stepId, pos] of entries) {
      void workflowStore.updateStep(stepId, pos)
    }
  }, [])

  const onNodeDragStop = useCallback(
    (_event: React.MouseEvent, node: Node) => {
      pendingRef.current.set(node.id, {
        position_x: Math.round(node.position.x),
        position_y: Math.round(node.position.y),
      })
      flush()
    },
    [flush],
  )

  return { onNodeDragStop }
}

export { usePositionPersist }
