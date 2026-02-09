import type { StepExecutionStatus } from '@/stores'
import type { BadgeVariant } from '@/components/primitives'

const STATUS_COLORS: Record<StepExecutionStatus, string> = {
  idle: '#7d8590',
  pending: '#7d8590',
  running: '#3b82f6',
  success: '#2dd4bf',
  error: '#f85149',
  skipped: '#7d8590',
  paused: '#f59e0b',
}

const STATUS_LABELS: Record<StepExecutionStatus, string> = {
  idle: 'Idle',
  pending: 'Pending',
  running: 'Running',
  success: 'Completed',
  error: 'Failed',
  skipped: 'Skipped',
  paused: 'Paused',
}

const STATUS_BADGE_VARIANTS: Record<StepExecutionStatus, BadgeVariant> = {
  idle: 'neutral',
  pending: 'neutral',
  running: 'info',
  success: 'success',
  error: 'error',
  skipped: 'neutral',
  paused: 'warning',
}

export { STATUS_COLORS, STATUS_LABELS, STATUS_BADGE_VARIANTS }
