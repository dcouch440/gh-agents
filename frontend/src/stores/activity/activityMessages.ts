// ============================================================================
// activityMessage — Pure function: ActivityEvent → human-readable string
//
// Uses a typed record keyed by ACTIVITY constants. Exhaustiveness is enforced
// by the mapped type — adding a variant to ActivityEvent without a handler
// here produces a compile error. No switch, no assertNever needed.
// ============================================================================

import { ACTIVITY } from '@/types/activity'
import type { ActivityEvent, ActivityEventOf } from '@/types/activity'

// ── Pluralization helper ─────────────────────────────────────────────────────

const plural = (n: number, word: string): string => `${n} ${word}${n === 1 ? '' : 's'}`

// ── Message formatters keyed by event type ───────────────────────────────────
//
// The mapped type ensures every ActivityEvent['type'] has an entry.
// Missing a key = compile error.

type MessageMap = {
  [K in ActivityEvent['type']]: (event: ActivityEventOf<K>) => string
}

const messages: MessageMap = {
  [ACTIVITY.WORKFLOW_STARTED]: (e) => `Workflow started (${plural(e.totalSteps, 'step')})`,

  [ACTIVITY.WORKFLOW_STEP_STARTED]: (e) => `Step "${e.stepName}" started`,

  [ACTIVITY.WORKFLOW_STEP_COMPLETED]: (e) =>
    e.durationMs !== null ? `Step "${e.stepName}" completed (${e.durationMs}ms)` : `Step "${e.stepName}" completed`,

  [ACTIVITY.WORKFLOW_STEP_FAILED]: (e) => `Step "${e.stepName}" FAILED: ${e.error}`,

  [ACTIVITY.WORKFLOW_STEP_PAUSED]: (e) => `Step "${e.stepName}" paused`,

  [ACTIVITY.WORKFLOW_FOR_EACH_PROGRESS]: (e) => `Step "${e.stepName}" progress: ${e.completed}/${e.total}`,

  [ACTIVITY.WORKFLOW_COMPLETED]: (e) => (e.durationMs !== null ? `Workflow completed (${e.durationMs}ms)` : 'Workflow completed'),

  [ACTIVITY.WORKFLOW_FAILED]: (e) => `Workflow FAILED: ${e.error}`,

  [ACTIVITY.WORKFLOW_RESUMED]: (e) => `Workflow resumed at step ${e.stepId}`,

  [ACTIVITY.ROOM_SPEAKER_START]: (e) => `${e.agentName} started speaking (turn ${e.turnNumber})`,

  [ACTIVITY.ROOM_SPEAKER_TOKEN]: (e) => `${e.agentName}: ${e.content}`,

  [ACTIVITY.ROOM_SPEAKER_END]: (e) => `${e.agentName} finished speaking (turn ${e.turnNumber})`,

  [ACTIVITY.ROOM_TURN_COMPLETE]: (e) => `Turn ${e.turnNumber} complete`,

  [ACTIVITY.ROOM_SESSION_COMPLETE]: (e) => `Room session complete (${plural(e.turnNumber, 'turn')})`,

  [ACTIVITY.SESSION_CREATED]: (e) => `Session "${e.title}" created`,

  [ACTIVITY.SESSION_UPDATED]: (e) => (e.title !== null ? `Session updated: "${e.title}"` : 'Session updated'),

  [ACTIVITY.SESSION_DELETED]: (e) => `Session ${e.sessionId} deleted`,
}

// ── Public API ───────────────────────────────────────────────────────────────

const activityMessage = (event: ActivityEvent): string => {
  const handler = messages[event.type] as (event: ActivityEvent) => string
  return handler(event)
}

export { activityMessage }
