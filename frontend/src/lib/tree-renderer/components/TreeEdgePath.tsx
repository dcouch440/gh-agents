import type { PositionedEdge, NodeStatus } from '../types'

type TreeEdgePathProps = {
  edge: PositionedEdge
  sourceStatus: NodeStatus
  targetStatus: NodeStatus
}

function TreeEdgePath({ edge, sourceStatus, targetStatus }: TreeEdgePathProps) {
  const isActive = sourceStatus === 'running' || targetStatus === 'running'
  const variantClass = edge.variant !== 'normal' ? ` tree-edge-path--${edge.variant}` : ''
  const activeClass = isActive ? ' tree-edge-path--active' : ''

  return (
    <path
      className={`tree-edge-path${variantClass}${activeClass}`}
      d={edge.path}
      markerEnd={isActive ? 'url(#tree-arrow-active)' : 'url(#tree-arrow)'}
    />
  )
}

export { TreeEdgePath }
export type { TreeEdgePathProps }
