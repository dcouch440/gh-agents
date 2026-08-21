import { workflowExecutionStore } from '../workflowExecutionStore'
import { store } from './_store'
import { hydrateLiveState } from './hydrate'

/** Poll fast while something is in flight, slowly when idle. */
const ACTIVE_POLL_MS = 2_000
const IDLE_POLL_MS = 15_000
const MAX_BACKOFF_MS = 30_000

let timer: ReturnType<typeof setTimeout> | null = null
let activeWorkflowId: string | null = null
let visibilityHandler: (() => void) | null = null

const isBusy = (): boolean => {
  if (store.getState().isGenerating) return true
  return workflowExecutionStore.selectIsRunning(workflowExecutionStore.store.getState())
}

const nextDelay = (): number => {
  // `hydrateLiveState` owns the failure count so the backoff and the
  // optimistic-flag expiry agree on what "failing" means.
  const failures = store.getState().consecutiveFailures
  if (failures > 0) {
    return Math.min(ACTIVE_POLL_MS * Math.pow(2, failures), MAX_BACKOFF_MS)
  }
  return isBusy() ? ACTIVE_POLL_MS : IDLE_POLL_MS
}

const clearTimer = (): void => {
  if (timer !== null) {
    clearTimeout(timer)
    timer = null
  }
}

const schedule = (): void => {
  clearTimer()
  if (activeWorkflowId === null) return
  timer = setTimeout(() => { void tick() }, nextDelay())
}

const tick = async (): Promise<void> => {
  const workflowId = activeWorkflowId
  if (workflowId === null) return

  await hydrateLiveState(workflowId)

  // Guard against a workflow switch that happened during the await.
  if (activeWorkflowId !== workflowId) return

  schedule()
}

/**
 * Keep the editor's view in step with the server for `workflowId`.
 *
 * Idempotent: calling it again for the same workflow does not stack timers.
 * This is also the safety net for dropped WebSocket events — a run that
 * finished while the socket was down is reconciled on the next idle tick.
 */
const startLiveSync = (workflowId: string): void => {
  if (activeWorkflowId === workflowId && timer !== null) return

  stopLiveSync()
  activeWorkflowId = workflowId
  store.setState({ consecutiveFailures: 0 })

  // A backgrounded tab throttles timers, so catch up the moment it returns.
  visibilityHandler = () => {
    if (document.visibilityState === 'visible' && activeWorkflowId !== null) {
      void tick()
    }
  }
  document.addEventListener('visibilitychange', visibilityHandler)

  // The caller may have hydrated already (the editor page does, on mount) —
  // don't duplicate that fetch, just start the cadence.
  const state = store.getState()
  if (state.workflowId === workflowId && state.hydratedAt !== null) {
    schedule()
    return
  }

  void tick()
}

const stopLiveSync = (): void => {
  clearTimer()
  activeWorkflowId = null
  if (visibilityHandler !== null) {
    document.removeEventListener('visibilitychange', visibilityHandler)
    visibilityHandler = null
  }
}

export { startLiveSync, stopLiveSync, ACTIVE_POLL_MS, IDLE_POLL_MS, MAX_BACKOFF_MS }
