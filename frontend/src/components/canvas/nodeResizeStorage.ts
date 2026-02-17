/**
 * Lightweight localStorage persistence for virtual node layout (agent, document, and notes nodes).
 * These nodes have no backing workflow_step row, so we use localStorage instead.
 */

const LS_DIMS_KEY = 'nexor_node_dimensions'
const LS_POS_KEY = 'nexor_node_positions'

type NodeDimensions = { width: number; height: number }
type NodePosition = { x: number; y: number }

/** Returns true for nodes that have no backing workflow_step and use localStorage. */
const isVirtualNode = (nodeId: string): boolean =>
  nodeId.startsWith('doc-artifact-') || nodeId.startsWith('notes-') || nodeId.startsWith('agent-artifact-')

// ── Dimensions ──────────────────────────────────────────────────────

const getStoredDimensions = (nodeId: string): NodeDimensions | null => {
  try {
    const raw = localStorage.getItem(LS_DIMS_KEY)
    if (!raw) return null
    const map = JSON.parse(raw) as Record<string, NodeDimensions>
    return map[nodeId] ?? null
  } catch {
    return null
  }
}

const setStoredDimensions = (nodeId: string, dims: NodeDimensions): void => {
  try {
    const raw = localStorage.getItem(LS_DIMS_KEY)
    const map: Record<string, NodeDimensions> = raw ? (JSON.parse(raw) as Record<string, NodeDimensions>) : {}
    map[nodeId] = dims
    localStorage.setItem(LS_DIMS_KEY, JSON.stringify(map))
  } catch {
    // Silently ignore storage errors
  }
}

// ── Positions ───────────────────────────────────────────────────────

const getStoredPosition = (nodeId: string): NodePosition | null => {
  try {
    const raw = localStorage.getItem(LS_POS_KEY)
    if (!raw) return null
    const map = JSON.parse(raw) as Record<string, NodePosition>
    return map[nodeId] ?? null
  } catch {
    return null
  }
}

const setStoredPosition = (nodeId: string, pos: NodePosition): void => {
  try {
    const raw = localStorage.getItem(LS_POS_KEY)
    const map: Record<string, NodePosition> = raw ? (JSON.parse(raw) as Record<string, NodePosition>) : {}
    map[nodeId] = pos
    localStorage.setItem(LS_POS_KEY, JSON.stringify(map))
  } catch {
    // Silently ignore storage errors
  }
}

export { isVirtualNode, getStoredDimensions, setStoredDimensions, getStoredPosition, setStoredPosition }
export type { NodeDimensions, NodePosition }
