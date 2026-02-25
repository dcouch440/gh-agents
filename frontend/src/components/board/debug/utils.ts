// ── Debug panel utilities ────────────────────────────────────────────────────

/**
 * Format a timestamp relative to a reference time.
 * Returns "+0.0s", "+1.2s", "+65.3s", etc.
 */
const relativeTime = (timestampMs: number, referenceMs: number): string => {
  const delta = (timestampMs - referenceMs) / 1000
  const sign = delta >= 0 ? '+' : '-'
  return `${sign}${Math.abs(delta).toFixed(1)}s`
}

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
 * True if this activity event type represents an error.
 */
const isErrorEvent = (eventType: string): boolean =>
  eventType.includes('failed') || eventType.includes('error')

export { relativeTime, statusColor, isErrorEvent }
