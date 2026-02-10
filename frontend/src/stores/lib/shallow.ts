// ============================================================================
// Shallow Equality Comparator
// ============================================================================

const shallow = <T>(a: T, b: T): boolean => {
  if (Object.is(a, b)) return true
  if (typeof a !== 'object' || typeof b !== 'object') return false
  if (a === null || b === null) return false

  const keysA = Object.keys(a)
  const keysB = Object.keys(b)
  if (keysA.length !== keysB.length) return false

  return keysA.every((key) => Object.hasOwn(b as object, key) && Object.is(a[key as keyof T], b[key as keyof T]))
}

export { shallow }
