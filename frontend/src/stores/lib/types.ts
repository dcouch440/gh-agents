// ============================================================================
// Store Core Types
// ============================================================================

export type Listener = () => void

export type SetState<T> = {
  (partial: Partial<T>): void
  (updater: (state: T) => Partial<T>): void
}

export type GetState<T> = () => T

export type Subscribe = (listener: Listener) => () => void

export type StoreApi<T> = {
  getState: GetState<T>
  setState: SetState<T>
  subscribe: Subscribe
  destroy: () => void
}

export type StateCreator<T> = (set: SetState<T>, get: GetState<T>) => T
