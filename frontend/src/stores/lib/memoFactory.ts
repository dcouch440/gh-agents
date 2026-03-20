// Memoize factory selectors so they return stable function references for the
// same argument. Without this, useStore's useCallback sees a new selector on
// every render and useSyncExternalStore loops infinitely.
const memoFactory = <A extends string | null, R>(
  factory: (arg: A) => R,
): ((arg: A) => R) => {
  const cache = new Map<A, R>()
  return (arg: A): R => {
    const cached = cache.get(arg)
    if (cached !== undefined) return cached
    const result = factory(arg)
    cache.set(arg, result)
    return result
  }
}

export { memoFactory }
