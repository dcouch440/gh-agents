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
/**
 * Which workflow has a hydration in flight, so overlapping ticks collapse into
 * one. Holds the id rather than a bare flag: a switch to a *different* workflow
 * must never be blocked by the previous one's request still being in the air.
 */
let inFlightFor: string | null = null

const isBusy = (): boolean => {
  if (store.getState().isGenerating) return true
  return workflowExecutionStore.selectIsRunning(workflowExecutionStore.store.getState())
}

const remainingThrottleMs = (): number => {
  const until = store.getState().throttledUntilMs
  return until === null ? 0 : Math.max(0, until - Date.now())
}

const nextDelay = (): number => {
  // Being throttled outranks everything: the server has told us exactly how long
  // to wait, and polling sooner only deepens the hole. Floor it at the active
  // cadence so a zero or already-elapsed wait cannot turn into a tight loop.
  const throttled = remainingThrottleMs()
  if (throttled > 0) return Math.max(throttled, ACTIVE_POLL_MS)

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

  // One hydration at a time. `tick` is reachable from the timer and from the
  // visibility handler, and each hydration fans out into a live-state call plus
  // a fetch per dispatch plus a timeline call — overlapping them multiplies that
  // burst for no new information. The in-flight tick re-arms the timer when it
  // finishes, so bailing out here never stalls the cadence.
  if (inFlightFor === workflowId) return
  // The pending timer would otherwise fire mid-await and start a second tick.
  clearTimer()

  // A throttle can be recorded outside this loop — a trace fetch that came back
  // 429, or the re-hydrate after dropped WebSocket events. The timer already
  // pending at that moment was scheduled without knowing, so check again here
  // rather than spending the request and earning a longer ban.
  if (remainingThrottleMs() > 0) {
    schedule()
    return
  }

  inFlightFor = workflowId
  try {
    await hydrateLiveState(workflowId)
  } finally {
    // Only release the slot if a later tick has not already claimed it.
    if (inFlightFor === workflowId) inFlightFor = null
  }

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

  // A backgrounded tab throttles timers, so catch up the moment it returns —
  // unless the server has asked us to wait, in which case the scheduled timer
  // already has the right delay and firing now would just earn another 429.
  visibilityHandler = () => {
    if (document.visibilityState !== 'visible' || activeWorkflowId === null) return
    if (remainingThrottleMs() > 0) return
    void tick()
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

/**
 * Re-arm the timer against the current state.
 *
 * Cadence is chosen when a tick is scheduled, so a state change that should
 * speed polling up (a Generate starting) would otherwise not take effect until
 * the already-scheduled long delay elapsed.
 */
const rescheduleLiveSync = (): void => {
  if (activeWorkflowId === null) return
  schedule()
}

const stopLiveSync = (): void => {
  clearTimer()
  activeWorkflowId = null
  // Release the in-flight slot. The request itself cannot be cancelled, but its
  // result is discarded by the workflow-id guard in `tick`, so holding the slot
  // would only block a restart of the same workflow until it happened to settle.
  inFlightFor = null
  if (visibilityHandler !== null) {
    document.removeEventListener('visibilitychange', visibilityHandler)
    visibilityHandler = null
  }
}

export { startLiveSync, stopLiveSync, rescheduleLiveSync, ACTIVE_POLL_MS, IDLE_POLL_MS, MAX_BACKOFF_MS }
