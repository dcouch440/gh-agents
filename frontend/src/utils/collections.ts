// ============================================================================
// Collections — Static Utility Class for Memory-Optimal Array Operations
// ============================================================================

/**
 * Pure, single-pass collection algorithms with zero intermediate allocations.
 * Every method accepts `readonly T[]` and never mutates its input.
 *
 * Design constraints:
 * - `noUncheckedIndexedAccess: true` → use `for...of` (or `!` with bounds guard)
 * - `erasableSyntaxOnly: true` → `private constructor` is erasable, allowed
 * - `no-explicit-any` → every generic fully typed
 * - `null` over `undefined` for intentional absence
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
    for (const item of items) {
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
    for (const item of items) {
      map.set(keyFn(item), valueFn(item))
    }
    return map
  }

  /**
   * Group items into `Map<K, T[]>` by key extractor.
   * Group arrays are reused via `map.get()` reference — no spreading.
   * Insertion order preserved within each group. O(n).
   */
  static groupBy<T, K>(items: readonly T[], keyFn: (item: T) => K): Map<K, T[]> {
    const map = new Map<K, T[]>()
    for (const item of items) {
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
   * O(n), 1 Map allocation.
   */
  static indexById<T extends { id: string }>(items: readonly T[]): Map<string, T> {
    const map = new Map<string, T>()
    for (const item of items) {
      map.set(item.id, item)
    }
    return map
  }

  // ── Set Builders ─────────────────────────────────────────────────────

  /**
   * Build a `Set<T>` from an array. O(n).
   * Uses the native Set constructor for optimal iteration.
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
    for (const item of items) {
      set.add(keyFn(item))
    }
    return set
  }

  // ── Single-Pass Transforms ───────────────────────────────────────────

  /**
   * Combined filter + map in a single pass.
   * Return `null` to skip an item, or a transformed value to include it.
   * Replaces `.filter(pred).map(transform)` chains (which allocate 2 arrays).
   * O(n), 1 output array.
   */
  static filterMap<T, U>(
    items: readonly T[],
    fn: (item: T, index: number) => U | null,
  ): U[] {
    const result: U[] = []
    for (let i = 0; i < items.length; i++) {
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
   * O(n), 2 output arrays.
   */
  static partition<T>(
    items: readonly T[],
    predicate: (item: T) => boolean,
  ): [T[], T[]] {
    const pass: T[] = []
    const fail: T[] = []
    for (const item of items) {
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
    for (const item of items) {
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
    for (const item of items) {
      sum += valueFn(item)
    }
    return sum
  }

  /**
   * Compute multiple numeric aggregates in a single pass.
   * `fns` maps names to extractor functions. Returns an object with matching keys.
   *
   * Example: `Collections.aggregate(rows, { input: r => r.in, output: r => r.out })`
   * → `{ input: 150, output: 300 }`
   *
   * O(n * k) where k = number of fields. 1 result object allocation.
   */
  static aggregate<T, K extends string>(
    items: readonly T[],
    fns: Record<K, (item: T) => number>,
  ): Record<K, number> {
    const keys = Object.keys(fns) as K[]
    const extractors = keys.map((k) => fns[k])
    const result = {} as Record<K, number>
    for (const key of keys) {
      result[key] = 0
    }
    for (const item of items) {
      for (let ki = 0; ki < keys.length; ki++) {
        result[keys[ki]!] += extractors[ki]!(item)
      }
    }
    return result
  }

  // ── Lookup Resolution ────────────────────────────────────────────────

  /**
   * Resolve an ordered list of keys against a Map.
   * Returns only items that exist, in the order of `keys`.
   * Replaces `keys.map(id => map.find(x => x.id === id)).filter(Boolean)`.
   * O(n), 1 output array.
   */
  static resolveKeys<K, V>(keys: readonly K[], map: ReadonlyMap<K, V>): V[] {
    const result: V[] = []
    for (const key of keys) {
      const value = map.get(key)
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
    for (const item of array) {
      if (!set.has(item)) return false
    }
    return true
  }

  // ── Sort ─────────────────────────────────────────────────────────────

  /**
   * Return a sorted shallow copy without mutating the input.
   * O(n log n), 1 array allocation via `.slice()`.
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
