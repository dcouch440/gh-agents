import type { NodeStatus } from '../types'

type TreeNodeBoxProps = {
  width: number
  height: number
  status: NodeStatus
}

function TreeNodeBox({ width, height, status }: TreeNodeBoxProps) {
  return (
    <rect
      className={`tree-node-box tree-node-box--${status}`}
      x={0}
      y={0}
      width={width}
      height={height}
      rx="var(--tree-node-radius, 2)"
      ry="var(--tree-node-radius, 2)"
      strokeWidth={1.5}
      filter={status === 'running' ? 'url(#tree-glow)' : undefined}
    />
  )
}

export { TreeNodeBox }
export type { TreeNodeBoxProps }
