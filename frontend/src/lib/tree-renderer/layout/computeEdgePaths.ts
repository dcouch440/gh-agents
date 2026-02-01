import type { EdgeStyle, PositionedNode, PositionedEdge, TreeEdgeData, Orientation } from '../types'

/**
 * Generates SVG path `d` strings for edges between positioned nodes.
 */
const computeEdgePaths = (
  edges: TreeEdgeData[],
  nodeMap: Map<string, PositionedNode>,
  edgeStyle: EdgeStyle,
  orientation: Orientation,
): PositionedEdge[] => {
  const result: PositionedEdge[] = []

  for (const edge of edges) {
    const source = nodeMap.get(edge.sourceId)
    const target = nodeMap.get(edge.targetId)
    if (source === undefined || target === undefined) continue

    const path = buildPath(source, target, edgeStyle, orientation)

    result.push({
      sourceId: edge.sourceId,
      targetId: edge.targetId,
      path,
      variant: edge.variant,
      label: edge.label,
    })
  }

  return result
}

const buildPath = (
  source: PositionedNode,
  target: PositionedNode,
  style: EdgeStyle,
  orientation: Orientation,
): string => {
  // Source exit point: center-bottom (vertical) or center-right (horizontal)
  // Target entry point: center-top (vertical) or center-left (horizontal)
  let sx: number, sy: number, tx: number, ty: number

  if (orientation === 'vertical') {
    sx = source.x + source.width / 2
    sy = source.y + source.height
    tx = target.x + target.width / 2
    ty = target.y
  } else {
    sx = source.x + source.width
    sy = source.y + source.height / 2
    tx = target.x
    ty = target.y + target.height / 2
  }

  if (style === 'straight') {
    return `M ${sx} ${sy} L ${tx} ${ty}`
  }

  if (style === 'curve') {
    if (orientation === 'vertical') {
      const midY = (sy + ty) / 2
      return `M ${sx} ${sy} C ${sx} ${midY}, ${tx} ${midY}, ${tx} ${ty}`
    }
    const midX = (sx + tx) / 2
    return `M ${sx} ${sy} C ${midX} ${sy}, ${midX} ${ty}, ${tx} ${ty}`
  }

  // step (default): right-angle connectors
  if (orientation === 'vertical') {
    const midY = (sy + ty) / 2
    return `M ${sx} ${sy} L ${sx} ${midY} L ${tx} ${midY} L ${tx} ${ty}`
  }
  const midX = (sx + tx) / 2
  return `M ${sx} ${sy} L ${midX} ${sy} L ${midX} ${ty} L ${tx} ${ty}`
}

export { computeEdgePaths }
