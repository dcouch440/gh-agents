import { StatusBadge } from '@/components/primitives/StatusBadge'
import type { BadgeVariant } from '@/components/primitives/StatusBadge'
import type { AgentExecutionStatus } from '@/types/execution'

type ExecutionStatusBadgeProps = {
  status: AgentExecutionStatus
}

const STATUS_MAP: Record<AgentExecutionStatus, { label: string; variant: BadgeVariant }> = {
  awaiting_user: { label: 'Awaiting Review', variant: 'warning' },
  running: { label: 'Running', variant: 'info' },
  completed: { label: 'Completed', variant: 'success' },
  failed: { label: 'Failed', variant: 'error' },
  pending: { label: 'Pending', variant: 'neutral' },
  cancelled: { label: 'Cancelled', variant: 'neutral' },
}

function ExecutionStatusBadge({ status }: ExecutionStatusBadgeProps) {
  const config = STATUS_MAP[status]
  return <StatusBadge label={config.label} variant={config.variant} />
}

export { ExecutionStatusBadge }
export type { ExecutionStatusBadgeProps }
