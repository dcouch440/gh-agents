import { isRateLimitError } from '@/api'
import { workflowExecutionStore } from '../workflowExecutionStore'
import { agentTraceStore } from '../agentTraceStore'
import { store } from './_store'
import { hydrateActive, DEFAULT_THROTTLE_MS } from './hydrate'

/**
 * Switch the editor to a past run's view and load its agent traces.
 *
 * The live poller (`hydrateLiveState`) stops touching `agentTraceStore` while
 * `workflowExecutionStore`'s view mode is `'history'`, so this is the only
 * place that populates traces for the run being viewed.
 */
const viewHistoricalRun = async (runId: string): Promise<void> => {
  workflowExecutionStore.viewHistoricalRun(runId)
  agentTraceStore.setHydratedRun(runId)
  try {
    await agentTraceStore.hydrateFromTimeline(runId)
  } catch (e) {
    // Best-effort, mirroring `hydrateLiveState` — a missing timeline must not
    // leave the panel stuck, but a 429 is worth recording so the next action
    // backs off instead of hammering.
    if (isRateLimitError(e)) {
      store.setState({
        throttledUntilMs: Date.now() + (e.retryAfterMs ?? DEFAULT_THROTTLE_MS),
      })
    }
  }
}

/** Return to the live run and let the poller resume driving `agentTraceStore`. */
const returnToLive = async (): Promise<void> => {
  workflowExecutionStore.returnToLive()
  await hydrateActive()
}

export { viewHistoricalRun, returnToLive }
