// ── Dispatch panel utilities ─────────────────────────────────────────────────

import { ACTIVITY } from '@/types/activity'
import type { ActivityEvent } from '@/types/activity'

/**
 * Map a dispatch status string to a MUI-compatible color token.
 */
const statusColor = (status: string): 'success' | 'error' | 'warning' | 'info' => {
  switch (status) {
    case 'completed':
      return 'success'
    case 'failed':
      return 'error'
    case 'cancelled':
      return 'warning'
    default:
      return 'info'
  }
}

/**
 * Format an ISO timestamp as a compact relative time string.
 */
const relativeTime = (isoTs: string): string => {
  const diffMs = Date.now() - new Date(isoTs).getTime()
  if (diffMs < 5_000) return 'just now'
  const seconds = Math.floor(diffMs / 1_000)
  if (seconds < 60) return `${seconds}s ago`
  const minutes = Math.floor(seconds / 60)
  if (minutes < 60) return `${minutes}m ago`
  const hours = Math.floor(minutes / 60)
  return `${hours}h ago`
}

const ERROR_TYPES: ReadonlySet<ActivityEvent['type']> = new Set([
  ACTIVITY.WORKFLOW_STEP_FAILED,
  ACTIVITY.WORKFLOW_FAILED,
  ACTIVITY.DISPATCH_FAILED,
  ACTIVITY.DISPATCH_STREAM_ERROR,
])

/**
 * Check if an activity event type represents an error.
 */
const isErrorEvent = (eventType: ActivityEvent['type']): boolean =>
  ERROR_TYPES.has(eventType)

export { statusColor, relativeTime, isErrorEvent }
