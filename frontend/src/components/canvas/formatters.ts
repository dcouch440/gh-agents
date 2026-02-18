/** Format a duration in milliseconds to a human-readable string. */
const formatDuration = (ms: number): string => {
  if (ms < 1000) return `${ms}ms`
  if (ms < 60_000) return `${(ms / 1000).toFixed(1)}s`
  return `${(ms / 60_000).toFixed(1)}m`
}

/** Format a token count with k suffix for thousands. */
const formatTokens = (count: number): string => {
  if (count < 1000) return String(count)
  return `${(count / 1000).toFixed(1)}k`
}

/** Format a USD cost with appropriate decimal places. */
const formatCost = (usd: number): string => {
  if (usd < 0.01) return `$${usd.toFixed(4)}`
  return `$${usd.toFixed(2)}`
}

export { formatDuration, formatTokens, formatCost }
