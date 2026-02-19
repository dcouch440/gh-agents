import { useRef, useCallback } from 'react'
import type { Node } from '@xyflow/react'
import { workflowStore } from '@/stores'
import { Collections } from '@/utils/collections'
import { Geometry } from '@/utils/geometry'
import type { Rect } from '@/utils/geometry'
import { CanvasNodeKind, HOVER_ELIGIBLE_KINDS } from './canvasKinds'
import { isVirtualNode, setStoredPosition } from './nodeResizeStorage'
import { detectOverlaps, resolveOverlaps } from './layout'
import type { LayoutNode } from './layout'
import { CANVAS } from './constants'
import { setDragging } from './useGroupHoverDelay'

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

/**
 * Build a Rect from a React Flow node using measured dimensions.
 */
const nodeToRect = (node: Node): Rect => ({
  x: node.position.x,
  y: node.position.y,
  width: node.measured?.width ?? node.width ?? 200,
  height: node.measured?.height ?? node.height ?? 100,
})

const usePackDrag = (
  getNodes: () => Node[],
  setNodes: (updater: (nodes: Node[]) => Node[]) => void,
) => {
  const packMembersRef = useRef<ReadonlySet<string>>(new Set())
  const startPositionsRef = useRef<Map<string, { x: number; y: number }>>(new Map())
  const dragStartRef = useRef<{ x: number; y: number }>({ x: 0, y: 0 })
  /** IDs of nodes pushed by collision during this drag. */
  const pushedNodesRef = useRef<Set<string>>(new Set())
  /** Original positions of ALL non-mover nodes at drag start (for reset on Alt release). */
  const allOriginalPositionsRef = useRef<Map<string, { x: number; y: number }>>(new Map())

  const onNodeDragStart = useCallback(
    (_event: React.MouseEvent, node: Node) => {
      setDragging(true)
      const allNodes = getNodes()
      const members = resolvePackMembers(node.id, Collections.mapBy(allNodes, toPackNode))
      packMembersRef.current = members
      pushedNodesRef.current = new Set()

      dragStartRef.current = { x: node.position.x, y: node.position.y }

      // Snapshot pack member start positions
      const packPositions = new Map<string, { x: number; y: number }>()
      // Snapshot ALL node positions for collision reset
      const allPositions = new Map<string, { x: number; y: number }>()

      const moverIds = new Set<string>([node.id])
      for (const id of members) moverIds.add(id)

      for (let i = 0; i < allNodes.length; i++) {
        const n = allNodes[i]!
        if (members.has(n.id)) {
          packPositions.set(n.id, { x: n.position.x, y: n.position.y })
        }
        if (!moverIds.has(n.id)) {
          allPositions.set(n.id, { x: n.position.x, y: n.position.y })
        }
      }
      startPositionsRef.current = packPositions
      allOriginalPositionsRef.current = allPositions
    },
    [getNodes],
  )

  const onNodeDrag = useCallback(
    (event: React.MouseEvent, node: Node) => {
      const members = packMembersRef.current

      // 1. Move pack members (existing logic)
      if (members.size > 0) {
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
      }

      // 2. Collision push — only when Alt is held
      if (!event.altKey) {
        // Reset any previously pushed nodes to original positions
        if (pushedNodesRef.current.size > 0) {
          const originals = allOriginalPositionsRef.current
          setNodes((current) =>
            Collections.mapBy(current, (n) => {
              if (!pushedNodesRef.current.has(n.id)) return n
              const orig = originals.get(n.id)
              if (!orig) return n
              return { ...n, position: { x: orig.x, y: orig.y } }
            }),
          )
          pushedNodesRef.current = new Set()
        }
        return
      }

      // 3. Build mover group IDs
      const moverIds = new Set<string>([node.id])
      for (const id of members) moverIds.add(id)

      // 4. Build layout nodes for non-movers (use latest positions from RF)
      // Re-read nodes after pack member update
      const currentNodes = getNodes()
      const layoutNodes: LayoutNode[] = []
      const allRects = new Map<string, Rect>()

      for (const n of currentNodes) {
        const rect = nodeToRect(n)
        allRects.set(n.id, rect)
        if (!moverIds.has(n.id)) {
          layoutNodes.push({ id: n.id, kind: CanvasNodeKind.STEP, rect })
        }
      }

      // 5. Detect overlaps for each mover (inflated by gap)
      const allOverlaps: { nodeId: string; overlapRect: Rect; pushDirection: 'left' | 'right' | 'top' | 'bottom'; pushDistance: number }[] = []
      const seenOverlapIds = new Set<string>()

      for (const moverId of moverIds) {
        const moverRect = allRects.get(moverId)
        if (!moverRect) continue
        const inflated = Geometry.expandRect(moverRect, CANVAS.COLLISION_GAP)
        const overlaps = detectOverlaps(inflated, moverId, layoutNodes)

        for (const overlap of overlaps) {
          if (seenOverlapIds.has(overlap.nodeId)) continue
          seenOverlapIds.add(overlap.nodeId)
          allOverlaps.push(overlap)
        }
      }

      if (allOverlaps.length === 0) {
        // No collisions — reset any previously pushed nodes
        if (pushedNodesRef.current.size > 0) {
          const originals = allOriginalPositionsRef.current
          setNodes((current) =>
            Collections.mapBy(current, (n) => {
              if (!pushedNodesRef.current.has(n.id)) return n
              const orig = originals.get(n.id)
              if (!orig) return n
              return { ...n, position: { x: orig.x, y: orig.y } }
            }),
          )
          pushedNodesRef.current = new Set()
        }
        return
      }

      // 6. Resolve overlaps (cascading pushes)
      const resolved = resolveOverlaps(allOverlaps, allRects, CANVAS.GRID_SIZE)

      // 7. Expand resolved positions: if a pushed node is a protocol, also move its pack members
      const packNodes = Collections.mapBy(currentNodes, toPackNode)
      const finalPositions = new Map(resolved)

      for (const [pushedId, newPos] of resolved) {
        const oldRect = allRects.get(pushedId)
        if (!oldRect) continue

        // Compute delta from original position
        const dx = newPos.x - oldRect.x
        const dy = newPos.y - oldRect.y

        // Find pack members of the pushed node
        const pushedPackMembers = resolvePackMembers(pushedId, packNodes)
        for (const memberId of pushedPackMembers) {
          if (moverIds.has(memberId)) continue // don't move our own movers
          if (finalPositions.has(memberId)) continue // already resolved
          const memberRect = allRects.get(memberId)
          if (!memberRect) continue
          finalPositions.set(memberId, { x: memberRect.x + dx, y: memberRect.y + dy })
        }
      }

      // 8. Apply all positions
      const newPushed = new Set<string>()
      setNodes((current) =>
        Collections.mapBy(current, (n) => {
          const pos = finalPositions.get(n.id)
          if (!pos) return n
          newPushed.add(n.id)
          return { ...n, position: { x: pos.x, y: pos.y } }
        }),
      )
      pushedNodesRef.current = newPushed
    },
    [getNodes, setNodes],
  )

  const onNodeDragStop = useCallback(
    (_event: React.MouseEvent, node: Node) => {
      setDragging(false)
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

      // Persist pushed node positions
      if (pushedNodesRef.current.size > 0) {
        const allNodes = getNodes()
        for (let i = 0; i < allNodes.length; i++) {
          const n = allNodes[i]!
          if (!pushedNodesRef.current.has(n.id)) continue
          const pushedPos = { x: Math.round(n.position.x), y: Math.round(n.position.y) }
          if (isVirtualNode(n.id)) {
            setStoredPosition(n.id, pushedPos)
          } else {
            void workflowStore.updateStep(n.id, {
              position_x: pushedPos.x,
              position_y: pushedPos.y,
            })
          }
        }
      }

      // Clear refs
      packMembersRef.current = new Set()
      startPositionsRef.current.clear()
      pushedNodesRef.current = new Set()
      allOriginalPositionsRef.current.clear()
    },
    [getNodes],
  )

  return { onNodeDragStart, onNodeDrag, onNodeDragStop }
}

export { resolvePackMembers, usePackDrag, toPackNode }
export type { PackNode }
