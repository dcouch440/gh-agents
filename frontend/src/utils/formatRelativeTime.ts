/**
 * Format an ISO timestamp as a human-readable relative time string.
 * Returns empty string for null/undefined input.
 */
const formatRelativeTime = (iso: string | null): string => {
  if (!iso) return ''
  const diff = Date.now() - new Date(iso).getTime()
  const minutes = Math.floor(diff / 60_000)
  if (minutes < 1) return 'just now'
  if (minutes < 60) return `${String(minutes)}m ago`
  const hours = Math.floor(minutes / 60)
  if (hours < 24) return `${String(hours)}h ago`
  const days = Math.floor(hours / 24)
  if (days < 30) return `${String(days)}d ago`
  return new Date(iso).toLocaleDateString()
}

export { formatRelativeTime }
