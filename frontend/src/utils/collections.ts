// ============================================================================
// Collections — Static Utility Class for Maximum V8 Performance
// ============================================================================

/**
 * Pure, single-pass collection algorithms optimized for V8 internals.
 * Every method accepts `readonly T[]` and never mutates its input.
 *
 * V8 optimization strategy:
 * - Indexed `for (let i = 0; i < n; i++)` with cached length — no iterator
 *   protocol overhead, consistently fastest across all V8 scenarios
 * - `[]` + `.push()` for output arrays — creates PACKED element kinds
 *   (V8 skips hole checks per element vs HOLEY from `new Array(n)`)
 * - `items[i]!` — non-null assertion for `noUncheckedIndexedAccess`,
 *   erased at compile time, zero runtime cost
 */
class Collections {
  private constructor() {
    // Static-only — prevent instantiation
  }

  // ── Map Builders ─────────────────────────────────────────────────────

  /**
   * Build a `Map<K, T>` from an array by key extractor.
   * Last-write-wins for duplicate keys. O(n), 1 Map allocation.
   */
  static keyBy<T, K>(items: readonly T[], keyFn: (item: T) => K): Map<K, T> {
    const map = new Map<K, T>()
    const n = items.length
    for (let i = 0; i < n; i++) {
      const item = items[i]!
      map.set(keyFn(item), item)
    }
    return map
  }

  /**
   * Build a `Map<K, V>` from an array with separate key and value extractors.
   * Eliminates the intermediate tuple array from `new Map(arr.map(x => [k, v]))`.
   * Last-write-wins. O(n), 1 Map allocation.
   */
  static toLookupMap<T, K, V>(
    items: readonly T[],
    keyFn: (item: T) => K,
    valueFn: (item: T) => V,
  ): Map<K, V> {
    const map = new Map<K, V>()
    const n = items.length
    for (let i = 0; i < n; i++) {
      const item = items[i]!
      map.set(keyFn(item), valueFn(item))
    }
    return map
  }

  /**
   * Group items into `Map<K, T[]>` by key extractor.
   * Group arrays are PACKED (created via `[item]` literal + `.push()`).
   * Insertion order preserved within each group. O(n).
   */
  static groupBy<T, K>(items: readonly T[], keyFn: (item: T) => K): Map<K, T[]> {
    const map = new Map<K, T[]>()
    const n = items.length
    for (let i = 0; i < n; i++) {
      const item = items[i]!
      const key = keyFn(item)
      const group = map.get(key)
      if (group) {
        group.push(item)
      } else {
        map.set(key, [item])
      }
    }
    return map
  }

  /**
   * Shorthand for `keyBy(items, item => item.id)`.
   * Constrained to objects with a string `id` property.
   * O(n), 1 Map allocation. No closure allocation (direct `.id` access).
   */
  static indexById<T extends { id: string }>(items: readonly T[]): Map<string, T> {
    const map = new Map<string, T>()
    const n = items.length
    for (let i = 0; i < n; i++) {
      const item = items[i]!
      map.set(item.id, item)
    }
    return map
  }

  // ── Set Builders ─────────────────────────────────────────────────────

  /**
   * Build a `Set<T>` from an array. O(n).
   * Uses the native Set constructor for optimal V8 internal iteration.
   */
  static toSet<T>(items: readonly T[]): Set<T> {
    return new Set(items)
  }

  /**
   * Build a `Set<K>` from an array via key extractor.
   * Eliminates the intermediate array from `new Set(arr.map(fn))`.
   * O(n), 1 Set allocation.
   */
  static toSetBy<T, K>(items: readonly T[], keyFn: (item: T) => K): Set<K> {
    const set = new Set<K>()
    const n = items.length
    for (let i = 0; i < n; i++) {
      set.add(keyFn(items[i]!))
    }
    return set
  }

  // ── Single-Pass Transforms ───────────────────────────────────────────

  /**
   * Combined filter + map in a single pass.
   * Return `null` to skip an item, or a transformed value to include it.
   * Replaces `.filter(pred).map(transform)` chains (which allocate 2 arrays).
   * O(n), 1 output array (PACKED via `.push()`).
   */
  static filterMap<T, U>(
    items: readonly T[],
    fn: (item: T, index: number) => U | null,
  ): U[] {
    const result: U[] = []
    const n = items.length
    for (let i = 0; i < n; i++) {
      const mapped = fn(items[i]!, i)
      if (mapped !== null) {
        result.push(mapped)
      }
    }
    return result
  }

  /**
   * Split an array into `[pass, fail]` by predicate in a single pass.
   * Replaces two separate `.filter()` calls with inverted predicates.
   * O(n), 2 output arrays (both PACKED via `.push()`).
   */
  static partition<T>(
    items: readonly T[],
    predicate: (item: T) => boolean,
  ): [T[], T[]] {
    const pass: T[] = []
    const fail: T[] = []
    const n = items.length
    for (let i = 0; i < n; i++) {
      const item = items[i]!
      if (predicate(item)) {
        pass.push(item)
      } else {
        fail.push(item)
      }
    }
    return [pass, fail]
  }

  /**
   * Remove duplicates preserving insertion order (first-wins).
   * Optional `keyFn` deduplicates by derived key instead of identity.
   * O(n), 1 Set + 1 output array.
   */
  static dedup<T>(items: readonly T[], keyFn?: (item: T) => unknown): T[] {
    const seen = new Set<unknown>()
    const result: T[] = []
    const n = items.length
    for (let i = 0; i < n; i++) {
      const item = items[i]!
      const key = keyFn ? keyFn(item) : item
      if (!seen.has(key)) {
        seen.add(key)
        result.push(item)
      }
    }
    return result
  }

  // ── Aggregation ──────────────────────────────────────────────────────

  /**
   * Sum a numeric value extracted from each item.
   * O(n), 0 allocations (accumulator only).
   */
  static sumBy<T>(items: readonly T[], valueFn: (item: T) => number): number {
    let sum = 0
    const n = items.length
    for (let i = 0; i < n; i++) {
      sum += valueFn(items[i]!)
    }
    return sum
  }

  /**
   * Compute multiple numeric aggregates in a single pass.
   * Uses a PACKED_SMI numeric accumulator array for fastest V8 numeric indexing
   * in the hot inner loop, then builds the result object in a cold post-pass.
   *
   * O(n * k) where k = number of fields. 1 result object allocation.
   */
  static aggregate<T, K extends string>(
    items: readonly T[],
    fns: Record<K, (item: T) => number>,
  ): Record<K, number> {
    const entries = Object.entries(fns) as [K, (item: T) => number][]
    const k = entries.length

    // Build PACKED_SMI numeric accumulator (fastest for numeric indexing)
    const sums: number[] = []
    for (let j = 0; j < k; j++) sums.push(0)

    // Hot loop — accumulate into numeric array (faster than string-keyed object)
    const n = items.length
    for (let i = 0; i < n; i++) {
      const item = items[i]!
      for (let j = 0; j < k; j++) {
        sums[j] = sums[j]! + entries[j]![1](item)
      }
    }

    // Cold path — build result object once
    const result = {} as Record<K, number>
    for (let j = 0; j < k; j++) {
      result[entries[j]![0]] = sums[j]!
    }
    return result
  }

  // ── Lookup Resolution ────────────────────────────────────────────────

  /**
   * Resolve an ordered list of keys against a Map.
   * Returns only items that exist, in the order of `keys`.
   * Replaces `keys.map(id => map.find(x => x.id === id)).filter(Boolean)`.
   * O(n), 1 output array (PACKED via `.push()`).
   */
  static resolveKeys<K, V>(keys: readonly K[], map: ReadonlyMap<K, V>): V[] {
    const result: V[] = []
    const n = keys.length
    for (let i = 0; i < n; i++) {
      const value = map.get(keys[i]!)
      if (value !== undefined) {
        result.push(value)
      }
    }
    return result
  }

  // ── Comparison ───────────────────────────────────────────────────────

  /**
   * Check if a Set and an array contain the same elements.
   * Short-circuits on size mismatch or first missing element.
   * O(n), 0 allocations.
   */
  static setMatchesArray<T>(set: ReadonlySet<T>, array: readonly T[]): boolean {
    if (set.size !== array.length) return false
    const n = array.length
    for (let i = 0; i < n; i++) {
      if (!set.has(array[i]!)) return false
    }
    return true
  }

  /**
   * Element-wise array comparison using `Object.is`.
   * Short-circuits on length mismatch or first differing element.
   * O(n), 0 allocations.
   */
  static arraysEqual<T>(a: readonly T[], b: readonly T[]): boolean {
    if (a === b) return true
    if (a.length !== b.length) return false
    for (let i = 0; i < a.length; i++) {
      if (!Object.is(a[i], b[i])) return false
    }
    return true
  }

  // ── Sort ─────────────────────────────────────────────────────────────

  /**
   * Return a sorted shallow copy without mutating the input.
   * `.slice()` creates a PACKED copy. `.sort()` is V8's Timsort (already optimal).
   * O(n log n), 1 array allocation.
   */
  static sortedCopy<T>(
    items: readonly T[],
    compareFn: (a: T, b: T) => number,
  ): T[] {
    const copy = items.slice()
    copy.sort(compareFn)
    return copy
  }
}

export { Collections }
