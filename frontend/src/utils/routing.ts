type LocationStateWithFrom = {
  from: { pathname: string }
}

const hasRedirectState = (state: unknown): state is LocationStateWithFrom =>
  state !== null &&
  typeof state === 'object' &&
  'from' in state &&
  (state as Record<string, unknown>).from !== null &&
  typeof (state as Record<string, unknown>).from === 'object' &&
  'pathname' in ((state as Record<string, unknown>).from as Record<string, unknown>) &&
  typeof ((state as Record<string, unknown>).from as Record<string, unknown>).pathname === 'string'

export { hasRedirectState }
export type { LocationStateWithFrom }
