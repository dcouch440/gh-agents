// ============================================================================
// NormalizedMap — Immutable Map with Memoized Array Conversion
// ============================================================================

type NormalizedMap<T> = {
  readonly byId: ReadonlyMap<string, T>
  _array: T[] | null
  _version: number
}

// ── Factory ──────────────────────────────────────────────────────────────────

const createNormalizedMap = <T>(): NormalizedMap<T> => ({
  byId: new Map(),
  _array: [],
  _version: 0,
})

const nmFromArray = <T extends { id: string }>(items: T[]): NormalizedMap<T> => ({
  byId: new Map(items.map((item) => [item.id, item])),
  _array: items,
  _version: 0,
})

// ── Read ─────────────────────────────────────────────────────────────────────

const toArray = <T>(nm: NormalizedMap<T>): T[] => {
  nm._array ??= Array.from(nm.byId.values())
  return nm._array
}

const nmGet = <T>(nm: NormalizedMap<T>, id: string): T | undefined =>
  nm.byId.get(id)

const nmHas = <T>(nm: NormalizedMap<T>, id: string): boolean =>
  nm.byId.has(id)

const nmSize = <T>(nm: NormalizedMap<T>): number =>
  nm.byId.size

// ── Write (all return new NormalizedMap) ─────────────────────────────────────

const nmSet = <T>(nm: NormalizedMap<T>, id: string, item: T): NormalizedMap<T> => ({
  byId: new Map(nm.byId).set(id, item),
  _array: null,
  _version: nm._version + 1,
})

const nmDelete = <T>(nm: NormalizedMap<T>, id: string): NormalizedMap<T> => {
  if (!nm.byId.has(id)) return nm
  const next = new Map(nm.byId)
  next.delete(id)
  return { byId: next, _array: null, _version: nm._version + 1 }
}

const nmMerge = <T extends { id: string }>(nm: NormalizedMap<T>, items: T[]): NormalizedMap<T> => {
  const next = new Map(nm.byId)
  for (const item of items) {
    next.set(item.id, item)
  }
  return { byId: next, _array: null, _version: nm._version + 1 }
}

export { createNormalizedMap, nmFromArray, toArray, nmGet, nmHas, nmSize, nmSet, nmDelete, nmMerge }
export type { NormalizedMap }
