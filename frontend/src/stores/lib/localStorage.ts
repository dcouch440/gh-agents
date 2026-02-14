// ============================================================================
// localStorage — Safe wrappers for localStorage access (try/catch for SSR & privacy)
// ============================================================================

const lsGet = (key: string): string | null => {
  try {
    return localStorage.getItem(key)
  } catch {
    return null
  }
}

const lsSet = (key: string, value: string): void => {
  try {
    localStorage.setItem(key, value)
  } catch {
    /* noop — localStorage may be unavailable in SSR or private browsing */
  }
}

export { lsGet, lsSet }
