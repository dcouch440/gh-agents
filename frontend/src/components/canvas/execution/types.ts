import type { ReactNode } from 'react'

type NodeHeaderSize = 'compact' | 'standard' | 'large'

type NodeHeaderProps = {
  icon: ReactNode
  title: string
  subtitle: string | null
  accentColor: string
  size?: NodeHeaderSize
  badge?: ReactNode
  actions?: ReactNode
}

type ExecutionStatus = 'idle' | 'pending' | 'running' | 'completed' | 'failed' | 'skipped' | 'paused'

const STATUS_LABELS: Record<ExecutionStatus, string | null> = {
  idle: null,
  pending: 'Pending',
  running: 'Running',
  completed: 'Done',
  failed: 'Failed',
  skipped: 'Skipped',
  paused: 'Paused',
}

const SIZE_CONFIG = {
  compact: { iconBox: 24, iconFont: 14, titleFont: 12, gap: 1 },
  standard: { iconBox: 28, iconFont: 18, titleFont: 13, gap: 1 },
  large: { iconBox: 36, iconFont: 20, titleFont: 14, gap: 1.5 },
} as const

/**
 * Maps store-level StepExecutionStatus ('success'/'error') to
 * our component-level ExecutionStatus ('completed'/'failed').
 */
const toExecutionStatus = (status: string | undefined): ExecutionStatus => {
  if (!status || status === 'idle') return 'idle'
  if (status === 'success') return 'completed'
  if (status === 'error') return 'failed'
  return status as ExecutionStatus
}

export type { NodeHeaderSize, NodeHeaderProps, ExecutionStatus }
export { STATUS_LABELS, SIZE_CONFIG, toExecutionStatus }
