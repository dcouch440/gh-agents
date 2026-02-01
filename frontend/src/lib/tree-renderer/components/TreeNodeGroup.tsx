import type { ReactNode } from 'react'
import type { TransitionState } from '../hooks/useNodeTransitions'

type TreeNodeGroupProps = {
  x: number
  y: number
  nodeId: string
  transition: TransitionState
  onClick: ((nodeId: string) => void) | undefined
  onHover: ((nodeId: string | null) => void) | undefined
  children: ReactNode
}

function TreeNodeGroup({ x, y, nodeId, transition, onClick, onHover, children }: TreeNodeGroupProps) {
  const className = transition === 'stable'
    ? 'tree-node-group'
    : `tree-node-group tree-node-group--${transition}`

  const handleClick = onClick !== undefined ? () => onClick(nodeId) : undefined
  const handleMouseEnter = onHover !== undefined ? () => onHover(nodeId) : undefined
  const handleMouseLeave = onHover !== undefined ? () => onHover(null) : undefined

  return (
    <g
      className={className}
      transform={`translate(${x}, ${y})`}
      onClick={handleClick}
      onMouseEnter={handleMouseEnter}
      onMouseLeave={handleMouseLeave}
    >
      {children}
    </g>
  )
}

export { TreeNodeGroup }
export type { TreeNodeGroupProps }
