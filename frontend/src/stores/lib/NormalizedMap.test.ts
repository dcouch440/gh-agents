import { createNormalizedMap, nmFromArray, toArray, nmGet, nmHas, nmSize, nmSet, nmDelete, nmMerge } from './NormalizedMap'

type Item = { id: string; name: string }

const alice: Item = { id: '1', name: 'Alice' }
const bob: Item = { id: '2', name: 'Bob' }
const charlie: Item = { id: '3', name: 'Charlie' }

describe('NormalizedMap', () => {
  describe('createNormalizedMap', () => {
    it('returns empty map', () => {
      const nm = createNormalizedMap<Item>()
      expect(nmSize(nm)).toBe(0)
      expect(toArray(nm)).toEqual([])
    })
  })

  describe('nmFromArray', () => {
    it('creates map from array with correct byId entries', () => {
      const nm = nmFromArray([alice, bob])
      expect(nmSize(nm)).toBe(2)
      expect(nmGet(nm, '1')).toBe(alice)
      expect(nmGet(nm, '2')).toBe(bob)
    })

    it('preserves original array reference', () => {
      const items = [alice, bob]
      const nm = nmFromArray(items)
      expect(toArray(nm)).toBe(items)
    })
  })

  describe('toArray', () => {
    it('returns memoized array (same reference on repeat calls)', () => {
      const nm = nmFromArray([alice, bob])
      const arr1 = toArray(nm)
      const arr2 = toArray(nm)
      expect(arr1).toBe(arr2)
    })

    it('invalidates after nmSet', () => {
      const nm = nmFromArray([alice])
      const arr1 = toArray(nm)
      const nm2 = nmSet(nm, '2', bob)
      const arr2 = toArray(nm2)
      expect(arr1).not.toBe(arr2)
      expect(arr2).toHaveLength(2)
    })
  })

  describe('nmSet', () => {
    it('adds new item', () => {
      const nm = createNormalizedMap<Item>()
      const nm2 = nmSet(nm, '1', alice)
      expect(nmGet(nm2, '1')).toBe(alice)
      expect(nmSize(nm2)).toBe(1)
    })

    it('updates existing item', () => {
      const nm = nmFromArray([alice])
      const updated = { id: '1', name: 'Alice Updated' }
      const nm2 = nmSet(nm, '1', updated)
      expect(nmGet(nm2, '1')).toBe(updated)
      expect(nmSize(nm2)).toBe(1)
    })

    it('does not mutate original', () => {
      const nm = nmFromArray([alice])
      nmSet(nm, '2', bob)
      expect(nmSize(nm)).toBe(1)
    })
  })

  describe('nmDelete', () => {
    it('removes item', () => {
      const nm = nmFromArray([alice, bob])
      const nm2 = nmDelete(nm, '1')
      expect(nmHas(nm2, '1')).toBe(false)
      expect(nmSize(nm2)).toBe(1)
    })

    it('returns same instance for missing key', () => {
      const nm = nmFromArray([alice])
      const nm2 = nmDelete(nm, 'nonexistent')
      expect(nm2).toBe(nm)
      expect(nm2._version).toBe(nm._version)
    })

    it('does not mutate original', () => {
      const nm = nmFromArray([alice, bob])
      nmDelete(nm, '1')
      expect(nmSize(nm)).toBe(2)
    })
  })

  describe('nmGet / nmHas / nmSize', () => {
    it('nmGet returns item by id', () => {
      const nm = nmFromArray([alice, bob])
      expect(nmGet(nm, '1')).toBe(alice)
      expect(nmGet(nm, '99')).toBeUndefined()
    })

    it('nmHas returns true/false correctly', () => {
      const nm = nmFromArray([alice])
      expect(nmHas(nm, '1')).toBe(true)
      expect(nmHas(nm, '99')).toBe(false)
    })

    it('nmSize returns count', () => {
      expect(nmSize(createNormalizedMap())).toBe(0)
      expect(nmSize(nmFromArray([alice, bob, charlie]))).toBe(3)
    })
  })

  describe('nmMerge', () => {
    it('adds multiple items', () => {
      const nm = nmFromArray([alice])
      const nm2 = nmMerge(nm, [bob, charlie])
      expect(nmSize(nm2)).toBe(3)
      expect(nmGet(nm2, '2')).toBe(bob)
      expect(nmGet(nm2, '3')).toBe(charlie)
    })

    it('overwrites existing items', () => {
      const nm = nmFromArray([alice])
      const updated = { id: '1', name: 'Alice Updated' }
      const nm2 = nmMerge(nm, [updated, bob])
      expect(nmGet(nm2, '1')).toBe(updated)
      expect(nmSize(nm2)).toBe(2)
    })
  })

  describe('version', () => {
    it('increments on mutations', () => {
      const nm = createNormalizedMap<Item>()
      expect(nm._version).toBe(0)

      const nm2 = nmSet(nm, '1', alice)
      expect(nm2._version).toBe(1)

      const nm3 = nmDelete(nm2, '1')
      expect(nm3._version).toBe(2)

      const nm4 = nmMerge(nm3, [bob, charlie])
      expect(nm4._version).toBe(3)
    })

    it('nmFromArray starts at version 0', () => {
      const nm = nmFromArray([alice, bob])
      expect(nm._version).toBe(0)
    })
  })
})
