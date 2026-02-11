const extractError = (prefix: string, e: unknown): string =>
  e instanceof Error ? e.message : `${prefix}: unknown error`

export { extractError }
