import type { NodeStatus, TreeTheme } from '../types'
import { getStatusColor } from '../theme'

type StatusIndicatorProps = {
  status: NodeStatus
  x: number
  y: number
  size?: number
  theme: TreeTheme
}

function StatusIndicator({ status, x, y, size = 10, theme }: StatusIndicatorProps) {
  const color = getStatusColor(theme, status)
  const cx = x + size / 2
  const cy = y + size / 2
  const r = size / 2 - 1

  const className =
    status === 'running' ? 'tree-status-bubble tree-status-bubble--running' :
    status === 'waiting' ? 'tree-status-bubble tree-status-bubble--waiting' :
    'tree-status-bubble'

  return (
    <circle
      className={className}
      cx={cx}
      cy={cy}
      r={r}
      fill={color}
    />
  )
}

export { StatusIndicator }
export type { StatusIndicatorProps }
