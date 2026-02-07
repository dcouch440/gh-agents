// ============================================================================
// Batch Update Utility
// ============================================================================

let batchDepth = 0
const pendingNotify = new Set<() => void>()

const batch = (fn: () => void): void => {
  batchDepth++
  try {
    fn()
  } finally {
    batchDepth--
    if (batchDepth === 0) {
      const fns = [...pendingNotify]
      pendingNotify.clear()
      fns.forEach((f) => f())
    }
  }
}

const isBatching = (): boolean => batchDepth > 0

const scheduleBatchNotify = (fn: () => void): void => {
  if (batchDepth > 0) {
    pendingNotify.add(fn)
  } else {
    fn()
  }
}

export { batch, isBatching, scheduleBatchNotify }
