import type { CSSProperties } from 'react'
import { useEffect } from 'react'
import type { Node, Edge, ReactFlowInstance } from '@xyflow/react'
import { Collections } from '@/utils/collections'
import { nodeDataEqual } from './mappers'

const stylesEqual = (a: CSSProperties | undefined, b: CSSProperties | undefined): boolean => {
  if (a === b) return true
  if (a === undefined || b === undefined) return false
  return a.width === b.width && a.height === b.height
}

/**
 * Pushes store-derived node/edge data into ReactFlow state.
 *
 * Only touches data, type, and style — never clobbers selection or position
 * (RF owns those during drag). Structural changes (add/remove) trigger a full
 * replacement that preserves selection & positions for surviving nodes.
 */
const useCanvasSync = (
  rfNodes: Node[],
  rfEdges: Edge[],
  setNodes: ReactFlowInstance['setNodes'],
  setEdges: ReactFlowInstance['setEdges'],
): void => {
  useEffect(() => {
    setNodes((current) => {
      const currentIds = Collections.toSetBy(current, (n) => n.id)
      const newIds = Collections.toSetBy(rfNodes, (n) => n.id)

      const hasStructuralChange = rfNodes.some((n) => !currentIds.has(n.id)) || current.some((n) => !newIds.has(n.id))

      if (hasStructuralChange) {
        // Nodes added/removed — full replacement, preserve selection + positions.
        // RF owns position state (drag), so keep existing positions for nodes that
        // were already on the canvas. Only truly new nodes get computed defaults.
        const selMap = Collections.toLookupMap(
          current,
          (n) => n.id,
          (n) => n.selected ?? false,
        )
        const posMap = Collections.toLookupMap(
          current,
          (n) => n.id,
          (n) => n.position,
        )
        return Collections.mapBy(rfNodes, (n) => ({
          ...n,
          selected: selMap.get(n.id) ?? false,
          position: posMap.get(n.id) ?? n.position,
        }))
      }

      // Data-only change — value-compare, return current when nothing changed.
      // NEVER overwrite position here: RF owns position state (updated via drag),
      // and the store catches up via onNodeDragStop. Overwriting mid-drag causes
      // nodes to snap back to stale store positions.
      const newDataMap = Collections.keyBy(rfNodes, (n) => n.id)
      let anyChanged = false
      const result: typeof current = []
      for (let i = 0; i < current.length; i++) {
        const n = current[i]!
        const updated = newDataMap.get(n.id)
        if (!updated) {
          result.push(n)
          continue
        }

        const dEq = nodeDataEqual(n.data, updated.data)
        const tEq = n.type === updated.type
        const sEq = stylesEqual(n.style, updated.style)

        if (dEq && tEq && sEq) {
          result.push(n)
          continue
        }

        anyChanged = true
        result.push({
          ...n,
          data: dEq ? n.data : updated.data,
          type: updated.type,
          style: sEq ? n.style : updated.style,
        })
      }

      return anyChanged ? result : current
    })
  }, [rfNodes, setNodes])

  useEffect(() => {
    setEdges((current) => {
      const currentIds = Collections.toSetBy(current, (e) => e.id)
      const newIds = Collections.toSetBy(rfEdges, (e) => e.id)
      const hasStructuralChange = rfEdges.some((e) => !currentIds.has(e.id)) || current.some((e) => !newIds.has(e.id))

      if (hasStructuralChange) {
        const selMap = Collections.toLookupMap(
          current,
          (e) => e.id,
          (e) => e.selected ?? false,
        )
        return Collections.mapBy(rfEdges, (e) => ({
          ...e,
          selected: selMap.get(e.id) ?? false,
        }))
      }

      const newEdgeMap = Collections.keyBy(rfEdges, (e) => e.id)
      let anyChanged = false
      const result: typeof current = []
      for (let i = 0; i < current.length; i++) {
        const e = current[i]!
        const updated = newEdgeMap.get(e.id)
        if (!updated) {
          result.push(e)
          continue
        }
        if (e.source === updated.source && e.target === updated.target && e.type === updated.type) {
          result.push(e)
          continue
        }
        anyChanged = true
        result.push({ ...e, source: updated.source, target: updated.target, type: updated.type })
      }

      return anyChanged ? result : current
    })
  }, [rfEdges, setEdges])
}

export { useCanvasSync, stylesEqual }
