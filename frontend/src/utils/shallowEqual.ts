import { Collections } from '@/utils/collections'

/**
 * Shallow equality check for two records.
 *
 * Compares all keys — values are checked with `Object.is` for primitives
 * and `Collections.arraysEqual` for arrays. Returns `false` if key counts
 * differ or any value pair is not equal.
 */
const shallowEqual = (a: Record<string, unknown>, b: Record<string, unknown>): boolean => {
  const keysA = Object.keys(a)
  if (keysA.length !== Object.keys(b).length) return false
  for (let i = 0; i < keysA.length; i++) {
    const key = keysA[i]!
    const valA = a[key]
    const valB = b[key]
    if (Array.isArray(valA)) {
      if (!Array.isArray(valB)) return false
      if (!Collections.arraysEqual(valA as readonly unknown[], valB as readonly unknown[])) return false
    } else {
      if (!Object.is(valA, valB)) return false
    }
  }
  return true
}

export { shallowEqual }
