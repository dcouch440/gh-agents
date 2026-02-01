import type { ReactNode } from 'react'

type TreeNodeLabelProps = {
  x?: number
  y: number
  variant?: 'primary' | 'secondary' | 'warning'
  children: ReactNode
}

function TreeNodeLabel({ x = 8, y, variant = 'primary', children }: TreeNodeLabelProps) {
  const className = variant === 'primary'
    ? 'tree-node-label'
    : `tree-node-label tree-node-label--${variant}`

  return (
    <text className={className} x={x} y={y}>
      {children}
    </text>
  )
}

export { TreeNodeLabel }
export type { TreeNodeLabelProps }
