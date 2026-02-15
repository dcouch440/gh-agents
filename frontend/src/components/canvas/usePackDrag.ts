import { useRef, useCallback } from 'react'
import type { Node } from '@xyflow/react'
import { workflowStore } from '@/stores'
import { Collections } from '@/utils/collections'
import { CanvasNodeKind, HOVER_ELIGIBLE_KINDS } from './canvasKinds'
import { isVirtualNode, setStoredPosition } from './nodeResizeStorage'

type PackNode = {
  id: string
  kind: CanvasNodeKind
  protocolStepId: string | null
}

const resolvePackMembers = (
  draggedNodeId: string,
  nodes: ReadonlyArray<PackNode>,
): ReadonlySet<string> => {
  let draggedKind: CanvasNodeKind | null = null
  const candidates = new Set<string>()

  for (let i = 0; i < nodes.length; i++) {
    const node = nodes[i]!
    if (node.id === draggedNodeId) {
      draggedKind = node.kind
      continue
    }
    if (node.protocolStepId === draggedNodeId && HOVER_ELIGIBLE_KINDS.has(node.kind)) {
      candidates.add(node.id)
    }
  }

  if (draggedKind === CanvasNodeKind.PROTOCOL) return candidates
  return new Set()
}

const toPackNode = (node: Node): PackNode => ({
  id: node.id,
  kind: (node.data.kind as CanvasNodeKind | undefined) ?? CanvasNodeKind.STEP,
  protocolStepId: (node.data.protocolStepId as string | undefined) ?? null,
})

const usePackDrag = (
  getNodes: () => Node[],
  setNodes: (updater: (nodes: Node[]) => Node[]) => void,
) => {
  const packMembersRef = useRef<ReadonlySet<string>>(new Set())
  const startPositionsRef = useRef<Map<string, { x: number; y: number }>>(new Map())
  const dragStartRef = useRef<{ x: number; y: number }>({ x: 0, y: 0 })

  const onNodeDragStart = useCallback(
    (_event: React.MouseEvent, node: Node) => {
      const allNodes = getNodes()
      const members = resolvePackMembers(node.id, Collections.mapBy(allNodes, toPackNode))
      packMembersRef.current = members

      dragStartRef.current = { x: node.position.x, y: node.position.y }

      const positions = new Map<string, { x: number; y: number }>()
      for (let i = 0; i < allNodes.length; i++) {
        const n = allNodes[i]!
        if (members.has(n.id)) {
          positions.set(n.id, { x: n.position.x, y: n.position.y })
        }
      }
      startPositionsRef.current = positions
    },
    [getNodes],
  )

  const onNodeDrag = useCallback(
    (_event: React.MouseEvent, node: Node) => {
      const members = packMembersRef.current
      if (members.size === 0) return

      const dx = node.position.x - dragStartRef.current.x
      const dy = node.position.y - dragStartRef.current.y

      setNodes((current) =>
        Collections.mapBy(current, (n) => {
          if (!members.has(n.id)) return n
          const start = startPositionsRef.current.get(n.id)
          if (!start) return n
          return { ...n, position: { x: start.x + dx, y: start.y + dy } }
        }),
      )
    },
    [setNodes],
  )

  const onNodeDragStop = useCallback(
    (_event: React.MouseEvent, node: Node) => {
      const roundedPos = { x: Math.round(node.position.x), y: Math.round(node.position.y) }
      if (isVirtualNode(node.id)) {
        setStoredPosition(node.id, roundedPos)
      } else {
        void workflowStore.updateStep(node.id, {
          position_x: roundedPos.x,
          position_y: roundedPos.y,
        })
      }

      // Persist pack member positions
      const members = packMembersRef.current
      if (members.size > 0) {
        const allNodes = getNodes()
        for (let i = 0; i < allNodes.length; i++) {
          const n = allNodes[i]!
          if (!members.has(n.id)) continue
          const memberPos = { x: Math.round(n.position.x), y: Math.round(n.position.y) }
          if (isVirtualNode(n.id)) {
            setStoredPosition(n.id, memberPos)
          } else {
            void workflowStore.updateStep(n.id, {
              position_x: memberPos.x,
              position_y: memberPos.y,
            })
          }
        }
      }

      // Clear refs
      packMembersRef.current = new Set()
      startPositionsRef.current.clear()
    },
    [getNodes],
  )

  return { onNodeDragStart, onNodeDrag, onNodeDragStop }
}

export { resolvePackMembers, usePackDrag, toPackNode }
export type { PackNode }
