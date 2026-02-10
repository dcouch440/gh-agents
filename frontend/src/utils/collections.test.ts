import { describe, it, expect } from 'vitest'
import { Collections } from './collections'

// ── keyBy ──────────────────────────────────────────────────────────────────

describe('keyBy', () => {
  it('builds Map from array by key function', () => {
    const items = [{ name: 'a', v: 1 }, { name: 'b', v: 2 }]
    const map = Collections.keyBy(items, (i) => i.name)
    expect(map.size).toBe(2)
    expect(map.get('a')).toBe(items[0])
    expect(map.get('b')).toBe(items[1])
  })

  it('last-write-wins for duplicate keys', () => {
    const items = [{ k: 1, v: 'first' }, { k: 1, v: 'second' }]
    const map = Collections.keyBy(items, (i) => i.k)
    expect(map.get(1)?.v).toBe('second')
  })

  it('returns empty Map for empty array', () => {
    const map = Collections.keyBy([], (x) => x)
    expect(map.size).toBe(0)
  })

  it('works with non-string keys', () => {
    const items = [{ id: 1 }, { id: 2 }]
    const map = Collections.keyBy(items, (i) => i.id)
    expect(map.get(1)).toBe(items[0])
  })
})

// ── toLookupMap ────────────────────────────────────────────────────────────

describe('toLookupMap', () => {
  it('builds Map with transformed values', () => {
    const items = [{ id: 'a', name: 'Alice', age: 30 }, { id: 'b', name: 'Bob', age: 25 }]
    const map = Collections.toLookupMap(items, (i) => i.id, (i) => i.name)
    expect(map.get('a')).toBe('Alice')
    expect(map.get('b')).toBe('Bob')
  })

  it('returns empty Map for empty array', () => {
    const map = Collections.toLookupMap([], (x) => x, (x) => x)
    expect(map.size).toBe(0)
  })

  it('last-write-wins for duplicate keys', () => {
    const items = [{ k: 'x', v: 1 }, { k: 'x', v: 2 }]
    const map = Collections.toLookupMap(items, (i) => i.k, (i) => i.v)
    expect(map.get('x')).toBe(2)
  })
})

// ── groupBy ────────────────────────────────────────────────────────────────

describe('groupBy', () => {
  it('groups items by key function', () => {
    const items = [
      { dept: 'eng', name: 'Alice' },
      { dept: 'sales', name: 'Bob' },
      { dept: 'eng', name: 'Charlie' },
    ]
    const groups = Collections.groupBy(items, (i) => i.dept)
    expect(groups.get('eng')?.length).toBe(2)
    expect(groups.get('sales')?.length).toBe(1)
  })

  it('preserves insertion order within groups', () => {
    const items = [{ g: 1, v: 'a' }, { g: 1, v: 'b' }, { g: 1, v: 'c' }]
    const groups = Collections.groupBy(items, (i) => i.g)
    const group = groups.get(1)
    expect(group?.map((i) => i.v)).toEqual(['a', 'b', 'c'])
  })

  it('returns empty Map for empty array', () => {
    const groups = Collections.groupBy([], () => 'x')
    expect(groups.size).toBe(0)
  })

  it('creates single-element groups for unique keys', () => {
    const items = [{ k: 'a' }, { k: 'b' }, { k: 'c' }]
    const groups = Collections.groupBy(items, (i) => i.k)
    expect(groups.size).toBe(3)
    for (const group of groups.values()) {
      expect(group.length).toBe(1)
    }
  })
})

// ── indexById ──────────────────────────────────────────────────────────────

describe('indexById', () => {
  it('builds Map keyed by id property', () => {
    const items = [{ id: 'x', val: 1 }, { id: 'y', val: 2 }]
    const map = Collections.indexById(items)
    expect(map.get('x')).toBe(items[0])
    expect(map.get('y')).toBe(items[1])
  })

  it('returns empty Map for empty array', () => {
    const map = Collections.indexById([])
    expect(map.size).toBe(0)
  })

  it('last-write-wins for duplicate ids', () => {
    const items = [{ id: 'a', v: 1 }, { id: 'a', v: 2 }]
    const map = Collections.indexById(items)
    expect(map.get('a')?.v).toBe(2)
  })
})

// ── toSet ──────────────────────────────────────────────────────────────────

describe('toSet', () => {
  it('builds Set from array', () => {
    const set = Collections.toSet([1, 2, 3])
    expect(set.size).toBe(3)
    expect(set.has(2)).toBe(true)
  })

  it('deduplicates automatically', () => {
    const set = Collections.toSet([1, 1, 2, 2, 3])
    expect(set.size).toBe(3)
  })

  it('returns empty Set for empty array', () => {
    const set = Collections.toSet([])
    expect(set.size).toBe(0)
  })
})

// ── toSetBy ────────────────────────────────────────────────────────────────

describe('toSetBy', () => {
  it('builds Set from derived keys', () => {
    const items = [{ id: 'a' }, { id: 'b' }, { id: 'c' }]
    const set = Collections.toSetBy(items, (i) => i.id)
    expect(set.size).toBe(3)
    expect(set.has('b')).toBe(true)
  })

  it('deduplicates by key', () => {
    const items = [{ id: 'a', v: 1 }, { id: 'a', v: 2 }]
    const set = Collections.toSetBy(items, (i) => i.id)
    expect(set.size).toBe(1)
  })

  it('returns empty Set for empty array', () => {
    const set = Collections.toSetBy([], (x) => x)
    expect(set.size).toBe(0)
  })
})

// ── mapBy ─────────────────────────────────────────────────────────────────

describe('mapBy', () => {
  it('transforms each element via fn', () => {
    const result = Collections.mapBy([1, 2, 3], (n) => n * 10)
    expect(result).toEqual([10, 20, 30])
  })

  it('returns empty array for empty input', () => {
    const result = Collections.mapBy([], (x) => x)
    expect(result).toEqual([])
  })

  it('preserves order', () => {
    const result = Collections.mapBy(['c', 'a', 'b'], (s) => s.toUpperCase())
    expect(result).toEqual(['C', 'A', 'B'])
  })

  it('extracts property from objects', () => {
    const items = [{ id: 'x', v: 1 }, { id: 'y', v: 2 }]
    const result = Collections.mapBy(items, (i) => i.id)
    expect(result).toEqual(['x', 'y'])
  })

  it('handles single-element arrays', () => {
    expect(Collections.mapBy([42], (n) => n + 1)).toEqual([43])
  })
})

// ── filterMap ──────────────────────────────────────────────────────────────

describe('filterMap', () => {
  it('filters and maps in single pass', () => {
    const items = [1, 2, 3, 4, 5]
    const result = Collections.filterMap(items, (n) => (n % 2 === 0 ? n * 10 : null))
    expect(result).toEqual([20, 40])
  })

  it('skips items where fn returns null', () => {
    const result = Collections.filterMap(['a', '', 'b', ''], (s) => (s.length > 0 ? s.toUpperCase() : null))
    expect(result).toEqual(['A', 'B'])
  })

  it('passes index to fn', () => {
    const indices: number[] = []
    Collections.filterMap([10, 20, 30], (_item, index) => {
      indices.push(index)
      return null
    })
    expect(indices).toEqual([0, 1, 2])
  })

  it('returns empty array for empty input', () => {
    const result = Collections.filterMap([], () => 'x')
    expect(result).toEqual([])
  })

  it('returns empty array when all items filtered', () => {
    const result = Collections.filterMap([1, 2, 3], () => null)
    expect(result).toEqual([])
  })
})

// ── partition ──────────────────────────────────────────────────────────────

describe('partition', () => {
  it('splits array by predicate', () => {
    const [evens, odds] = Collections.partition([1, 2, 3, 4, 5], (n) => n % 2 === 0)
    expect(evens).toEqual([2, 4])
    expect(odds).toEqual([1, 3, 5])
  })

  it('all pass — fail array is empty', () => {
    const [pass, fail] = Collections.partition([2, 4, 6], (n) => n % 2 === 0)
    expect(pass).toEqual([2, 4, 6])
    expect(fail).toEqual([])
  })

  it('none pass — pass array is empty', () => {
    const [pass, fail] = Collections.partition([1, 3, 5], (n) => n % 2 === 0)
    expect(pass).toEqual([])
    expect(fail).toEqual([1, 3, 5])
  })

  it('returns two empty arrays for empty input', () => {
    const [pass, fail] = Collections.partition([], () => true)
    expect(pass).toEqual([])
    expect(fail).toEqual([])
  })
})

// ── dedup ──────────────────────────────────────────────────────────────────

describe('dedup', () => {
  it('removes duplicate primitives', () => {
    expect(Collections.dedup([1, 2, 2, 3, 1])).toEqual([1, 2, 3])
  })

  it('preserves first occurrence order', () => {
    expect(Collections.dedup(['b', 'a', 'b', 'c', 'a'])).toEqual(['b', 'a', 'c'])
  })

  it('returns empty array for empty input', () => {
    expect(Collections.dedup([])).toEqual([])
  })

  it('returns same-length array when no duplicates', () => {
    const result = Collections.dedup([1, 2, 3])
    expect(result).toEqual([1, 2, 3])
  })

  it('deduplicates by keyFn when provided', () => {
    const items = [{ id: 'a', v: 1 }, { id: 'b', v: 2 }, { id: 'a', v: 3 }]
    const result = Collections.dedup(items, (i) => i.id)
    expect(result).toEqual([{ id: 'a', v: 1 }, { id: 'b', v: 2 }])
  })

  it('keyFn dedup preserves first occurrence', () => {
    const items = [{ g: 1, v: 'first' }, { g: 1, v: 'second' }]
    const result = Collections.dedup(items, (i) => i.g)
    expect(result.length).toBe(1)
    expect(result[0]?.v).toBe('first')
  })
})

// ── sumBy ──────────────────────────────────────────────────────────────────

describe('sumBy', () => {
  it('sums numeric values from items', () => {
    const items = [{ cost: 10 }, { cost: 20 }, { cost: 30 }]
    expect(Collections.sumBy(items, (i) => i.cost)).toBe(60)
  })

  it('returns 0 for empty array', () => {
    expect(Collections.sumBy([], () => 1)).toBe(0)
  })

  it('handles single-element arrays', () => {
    expect(Collections.sumBy([{ v: 42 }], (i) => i.v)).toBe(42)
  })

  it('handles negative values', () => {
    expect(Collections.sumBy([{ v: 10 }, { v: -3 }, { v: -7 }], (i) => i.v)).toBe(0)
  })
})

// ── aggregate ──────────────────────────────────────────────────────────────

describe('aggregate', () => {
  it('computes multiple sums in single pass', () => {
    const rows = [
      { input: 10, output: 100, calls: 1 },
      { input: 20, output: 200, calls: 2 },
    ]
    const result = Collections.aggregate(rows, {
      totalInput: (r) => r.input,
      totalOutput: (r) => r.output,
      totalCalls: (r) => r.calls,
    })
    expect(result.totalInput).toBe(30)
    expect(result.totalOutput).toBe(300)
    expect(result.totalCalls).toBe(3)
  })

  it('returns zeros for empty array', () => {
    const result = Collections.aggregate([], {
      a: () => 1,
      b: () => 2,
    })
    expect(result.a).toBe(0)
    expect(result.b).toBe(0)
  })

  it('handles single field', () => {
    const result = Collections.aggregate([{ v: 5 }, { v: 3 }], {
      total: (r) => r.v,
    })
    expect(result.total).toBe(8)
  })
})

// ── resolveKeys ────────────────────────────────────────────────────────────

describe('resolveKeys', () => {
  const map = new Map([['a', 1], ['b', 2], ['c', 3]])

  it('returns items in key order', () => {
    expect(Collections.resolveKeys(['c', 'a'], map)).toEqual([3, 1])
  })

  it('skips keys not present in map', () => {
    expect(Collections.resolveKeys(['a', 'z', 'b'], map)).toEqual([1, 2])
  })

  it('returns empty array when no keys match', () => {
    expect(Collections.resolveKeys(['x', 'y'], map)).toEqual([])
  })

  it('returns empty array for empty keys array', () => {
    expect(Collections.resolveKeys([], map)).toEqual([])
  })

  it('preserves duplicate keys if present in input', () => {
    expect(Collections.resolveKeys(['a', 'a', 'b'], map)).toEqual([1, 1, 2])
  })
})

// ── setMatchesArray ────────────────────────────────────────────────────────

describe('setMatchesArray', () => {
  it('returns true when Set and array have same elements', () => {
    const set = new Set(['a', 'b', 'c'])
    expect(Collections.setMatchesArray(set, ['a', 'b', 'c'])).toBe(true)
  })

  it('returns true regardless of order', () => {
    const set = new Set(['a', 'b', 'c'])
    expect(Collections.setMatchesArray(set, ['c', 'a', 'b'])).toBe(true)
  })

  it('returns false when sizes differ', () => {
    const set = new Set(['a', 'b'])
    expect(Collections.setMatchesArray(set, ['a', 'b', 'c'])).toBe(false)
  })

  it('returns false when elements differ', () => {
    const set = new Set(['a', 'b', 'c'])
    expect(Collections.setMatchesArray(set, ['a', 'b', 'x'])).toBe(false)
  })

  it('returns true for both empty', () => {
    expect(Collections.setMatchesArray(new Set(), [])).toBe(true)
  })
})

// ── arraysEqual ───────────────────────────────────────────────────────

describe('arraysEqual', () => {
  it('returns true for same reference', () => {
    const arr = [1, 2, 3]
    expect(Collections.arraysEqual(arr, arr)).toBe(true)
  })

  it('returns true for equal primitive arrays', () => {
    expect(Collections.arraysEqual([1, 2, 3], [1, 2, 3])).toBe(true)
  })

  it('returns false for different lengths', () => {
    expect(Collections.arraysEqual([1, 2], [1, 2, 3])).toBe(false)
  })

  it('returns false for different elements', () => {
    expect(Collections.arraysEqual([1, 2, 3], [1, 9, 3])).toBe(false)
  })

  it('returns true for both empty', () => {
    expect(Collections.arraysEqual([], [])).toBe(true)
  })

  it('uses Object.is semantics (NaN === NaN)', () => {
    expect(Collections.arraysEqual([NaN], [NaN])).toBe(true)
  })

  it('distinguishes +0 and -0', () => {
    expect(Collections.arraysEqual([0], [-0])).toBe(false)
  })

  it('works with string arrays', () => {
    expect(Collections.arraysEqual(['a', 'b'], ['a', 'b'])).toBe(true)
    expect(Collections.arraysEqual(['a', 'b'], ['a', 'c'])).toBe(false)
  })

  it('works with null/undefined elements', () => {
    expect(Collections.arraysEqual([null, undefined], [null, undefined])).toBe(true)
    expect(Collections.arraysEqual([null], [undefined])).toBe(false)
  })
})

// ── sortedCopy ─────────────────────────────────────────────────────────────

describe('sortedCopy', () => {
  it('returns sorted copy without mutating input', () => {
    const input = [3, 1, 2]
    const sorted = Collections.sortedCopy(input, (a, b) => a - b)
    expect(sorted).toEqual([1, 2, 3])
    expect(input).toEqual([3, 1, 2])
  })

  it('returns empty array for empty input', () => {
    expect(Collections.sortedCopy([], (a, b) => a - b)).toEqual([])
  })

  it('handles single-element arrays', () => {
    expect(Collections.sortedCopy([42], (a, b) => a - b)).toEqual([42])
  })

  it('works with custom comparator', () => {
    const items = [{ name: 'Charlie' }, { name: 'Alice' }, { name: 'Bob' }]
    const sorted = Collections.sortedCopy(items, (a, b) => a.name.localeCompare(b.name))
    expect(sorted.map((i) => i.name)).toEqual(['Alice', 'Bob', 'Charlie'])
  })
})
