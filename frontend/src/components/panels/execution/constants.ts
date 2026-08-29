import type { StepExecutionStatus } from '@/stores'
import type { BadgeVariant } from '@/components/primitives'

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

export { STATUS_LABELS, STATUS_BADGE_VARIANTS }
