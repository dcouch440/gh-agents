import type { NodeStatus, TreeTheme } from '../types'
import { getStatusColor } from '../theme'

type StatusIndicatorProps = {
  status: NodeStatus
  x: number
  y: number
  size?: number
  theme: TreeTheme
}

function StatusIndicator({ status, x, y, size = 12, theme }: StatusIndicatorProps) {
  const color = getStatusColor(theme, status)
  const cx = x + size / 2
  const cy = y + size / 2
  const r = size / 2 - 1

  if (status === 'pending') {
    return (
      <circle
        className="tree-status-indicator"
        cx={cx}
        cy={cy}
        r={r}
        fill="none"
        stroke={color}
        strokeWidth={1.5}
      />
    )
  }

  if (status === 'running') {
    const circumference = 2 * Math.PI * r
    return (
      <circle
        className="tree-status-running"
        cx={cx}
        cy={cy}
        r={r}
        fill="none"
        stroke={color}
        strokeWidth={1.5}
        strokeDasharray={`${circumference * 0.3} ${circumference * 0.7}`}
        strokeLinecap="round"
      />
    )
  }

  if (status === 'completed') {
    const s = size * 0.3
    return (
      <path
        className="tree-status-indicator"
        d={`M ${cx - s} ${cy} L ${cx - s * 0.3} ${cy + s * 0.7} L ${cx + s} ${cy - s * 0.5}`}
        fill="none"
        stroke={color}
        strokeWidth={2}
        strokeLinecap="round"
        strokeLinejoin="round"
      />
    )
  }

  if (status === 'failed') {
    const s = size * 0.25
    return (
      <g className="tree-status-indicator">
        <line x1={cx - s} y1={cy - s} x2={cx + s} y2={cy + s} stroke={color} strokeWidth={2} strokeLinecap="round" />
        <line x1={cx + s} y1={cy - s} x2={cx - s} y2={cy + s} stroke={color} strokeWidth={2} strokeLinecap="round" />
      </g>
    )
  }

  if (status === 'waiting') {
    const s = size * 0.35
    return (
      <polygon
        className="tree-status-indicator"
        points={`${cx} ${cy - s}, ${cx + s} ${cy}, ${cx} ${cy + s}, ${cx - s} ${cy}`}
        fill={color}
      />
    )
  }

  // skipped: dash
  return (
    <line
      className="tree-status-indicator"
      x1={cx - size * 0.25}
      y1={cy}
      x2={cx + size * 0.25}
      y2={cy}
      stroke={color}
      strokeWidth={2}
      strokeLinecap="round"
    />
  )
}

export { StatusIndicator }
export type { StatusIndicatorProps }
