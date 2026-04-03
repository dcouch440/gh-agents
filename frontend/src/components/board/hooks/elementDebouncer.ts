// ============================================================================
// ElementDebouncerMap — Per-element debounce timer management
// ============================================================================
//
// Plain TypeScript class (no React). Manages a Map of independent debounce
// timers keyed by element ID. When an element is scheduled, any previous
// timer for that element is reset. The latest payload is always stored so
// flushAll() can fire callbacks with the most recent data.

type PendingEntry<T> = {
  timer: ReturnType<typeof setTimeout>
  payload: T
}

class ElementDebouncerMap<T> {
  private readonly pending = new Map<string, PendingEntry<T>>()
  private readonly onFire: (elementId: string, payload: T) => void
  private readonly delayMs: number

  constructor(delayMs: number, onFire: (elementId: string, payload: T) => void) {
    this.delayMs = delayMs
    this.onFire = onFire
  }

  /** Schedule a debounced callback for the given element. Resets any existing timer. */
  schedule(elementId: string, payload: T): void {
    const existing = this.pending.get(elementId)
    if (existing) {
      clearTimeout(existing.timer)
    }

    const timer = setTimeout(() => {
      this.pending.delete(elementId)
      this.onFire(elementId, payload)
    }, this.delayMs)

    this.pending.set(elementId, { timer, payload })
  }

  /** Cancel the pending timer for a single element. */
  cancel(elementId: string): void {
    const entry = this.pending.get(elementId)
    if (entry) {
      clearTimeout(entry.timer)
      this.pending.delete(elementId)
    }
  }

  /** Cancel all pending timers without firing callbacks. */
  cancelAll(): void {
    for (const entry of this.pending.values()) {
      clearTimeout(entry.timer)
    }
    this.pending.clear()
  }

  /** Fire all pending callbacks immediately with their latest payloads, then clear. */
  flushAll(): void {
    const entries = Array.from(this.pending.entries())
    for (const [, entry] of entries) {
      clearTimeout(entry.timer)
    }
    this.pending.clear()
    for (const [elementId, entry] of entries) {
      this.onFire(elementId, entry.payload)
    }
  }

  /** Whether any element has a pending timer. */
  hasPending(elementId: string): boolean {
    return this.pending.has(elementId)
  }

  /** Total number of pending timers. */
  get size(): number {
    return this.pending.size
  }

  /** Cancel all timers and release references. */
  dispose(): void {
    this.cancelAll()
  }
}

export { ElementDebouncerMap }
