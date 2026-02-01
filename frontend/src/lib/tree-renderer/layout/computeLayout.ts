import type { TreeData, LayoutOptions, LayoutResult } from '../types'
import { assignLayers } from './assignLayers'
import { orderWithinLayers } from './orderWithinLayers'
import { assignPositions } from './assignPositions'
import { computeEdgePaths } from './computeEdgePaths'

const DEFAULT_OPTIONS: LayoutOptions = {
  orientation: 'vertical',
  nodeWidth: 160,
  nodeHeight: 48,
  horizontalGap: 32,
  verticalGap: 48,
  edgeStyle: 'step',
}

const computeLayout = <M>(
  data: TreeData<M>,
  options?: Partial<LayoutOptions>,
): LayoutResult => {
  const opts: LayoutOptions = { ...DEFAULT_OPTIONS, ...options }

  // Empty tree
  if (data.rootIds.length === 0) {
    return { nodes: [], edges: [], width: 0, height: 0 }
  }

  // 1. Assign layers (depth)
  const layerMap = assignLayers(data)

  // 2. Order within layers (minimize crossings)
  const layers = orderWithinLayers(data, layerMap)

  // 3. Build children lookup
  const childrenMap = new Map<string, string[]>()
  for (const [id, node] of Object.entries(data.nodes)) {
    childrenMap.set(id, node.children)
  }

  // 4. Assign x/y positions
  const nodes = assignPositions(layers, childrenMap, opts)

  // 5. Compute edge paths (initial, will recompute after normalization)
  const nodeMap = new Map(nodes.map((n) => [n.id, n]))
  computeEdgePaths(data.edges, nodeMap, opts.edgeStyle, opts.orientation)

  // 6. Compute total bounds
  let minX = Infinity, minY = Infinity, maxX = -Infinity, maxY = -Infinity
  for (const node of nodes) {
    minX = Math.min(minX, node.x)
    minY = Math.min(minY, node.y)
    maxX = Math.max(maxX, node.x + node.width)
    maxY = Math.max(maxY, node.y + node.height)
  }

  // Normalize positions so min is at 0,0
  for (const node of nodes) {
    node.x -= minX
    node.y -= minY
  }

  // Recompute edges after normalization
  const normalizedNodeMap = new Map(nodes.map((n) => [n.id, n]))
  const normalizedEdges = computeEdgePaths(data.edges, normalizedNodeMap, opts.edgeStyle, opts.orientation)

  return {
    nodes,
    edges: normalizedEdges,
    width: maxX - minX,
    height: maxY - minY,
  }
}

export { computeLayout, DEFAULT_OPTIONS }
