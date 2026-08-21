import { api } from '@/api'
import { Collections } from '@/utils/collections'
import type { RunStepResult, WorkshopStepSummary, RosterAgent } from '@/types'
import type { StepExecutionState, StepExecutionStatus, StepTimelineEvent } from './types'
import type { SourceStreamState } from '../stepStreamStore/types'
import { store } from './_store'
import { sidebarStore } from '../sidebarStore'
import { stepStreamStore } from '../stepStreamStore'

/**
 * A run as the server currently describes it.
 *
 * `runId === null` means the workflow has never run — the overlay is cleared and
 * the caller may fall back to workshop state.
 */
type ServerRunSnapshot = {
  runId: string | null
  workflowId: string
  status: string | null
  startedAt: string | null
  completedAt: string | null
  durationMs: number | null
  error: string | null
  steps: readonly RunStepResult[]
}

const mapApiStatusToStoreStatus = (apiStatus: string): StepExecutionStatus => {
  switch (apiStatus) {
    case 'completed': return 'success'
    case 'failed': return 'error'
    case 'skipped': return 'skipped'
    case 'running': return 'running'
    case 'pending': return 'pending'
    case 'paused': return 'paused'
    default: return 'idle'
  }
}

const mapRunStepToStepState = (step: RunStepResult): StepExecutionState => ({
  status: mapApiStatusToStoreStatus(step.status),
  stepName: step.step_name,
  agentId: null,
  executionId: step.execution_id,
  output: step.output,
  error: step.error,
  inputTokens: step.input_tokens,
  outputTokens: step.output_tokens,
  durationMs: step.duration_ms,
  forEachProgress: null,
  startedAt: step.started_at,
  completedAt: step.completed_at,
})

const buildEventLog = (steps: readonly RunStepResult[]): StepTimelineEvent[] => {
  const events: StepTimelineEvent[] = []
  for (const step of steps) {
    if (step.started_at) {
      events.push({ stepId: step.step_id, stepName: step.step_name, eventType: 'started', ts: step.started_at })
    }
    if (step.status === 'completed' && step.completed_at) {
      events.push({ stepId: step.step_id, stepName: step.step_name, eventType: 'completed', ts: step.completed_at })
    } else if (step.status === 'failed' && step.completed_at) {
      events.push({ stepId: step.step_id, stepName: step.step_name, eventType: 'failed', ts: step.completed_at })
    }
  }
  events.sort((a, b) => a.ts.localeCompare(b.ts))
  return events
}

// ── Run overlay ─────────────────────────────────────────────────────────────

/**
 * Point the live-run view overlay at `runId`.
 *
 * Clears only client-side per-run buffers (step states, event log, counters) so
 * one run's results can never be painted as if they belonged to another. It
 * never touches `runs` (the history list), never calls a mutating endpoint, and
 * has no relationship to `pinned` / `run_results_summary` — those live in the
 * baseline layer and are re-fetched from the server, not preserved by copying.
 */
const setActiveRun = (runId: string | null, workflowId: string | null): void => {
  store.setState({
    runId,
    workflowId,
    isRunning: false,
    stepStates: {},
    eventLog: [],
    totalSteps: 0,
    completedStepCount: 0,
    durationMs: null,
    error: null,
    startedAt: null,
    completedAt: null,
    viewMode: 'live',
    selectedHistoricalRunId: null,
    historicalRun: null,
  })
}

/** Optimistically open an overlay for a run the client just started. */
const beginRun = (runId: string, workflowId: string): void => {
  setActiveRun(runId, workflowId)
  store.setState({ isRunning: true, startedAt: new Date().toISOString() })
}

/**
 * Within one run, WebSocket may be ahead of a REST snapshot that was already
 * in flight. Keep whichever side describes later progress.
 */
const preferLocal = (
  local: StepExecutionState | undefined,
  server: StepExecutionState,
): boolean => {
  if (local === undefined) return false
  if (local.status === 'running' && server.status === 'pending') return true
  const l = local.completedAt ?? local.startedAt
  const s = server.completedAt ?? server.startedAt
  if (l === null) return false
  if (s === null) return true
  // ISO-8601 is lexicographically ordered.
  return l > s
}

/**
 * A step with no execution row is reported as `skipped`, which is right for a
 * finished run but wrong for one still in flight — there the run simply has not
 * reached it yet. Without this, every step of a just-started run reads as
 * skipped.
 */
const normalizeForActiveRun = (
  state: StepExecutionState,
  isActive: boolean,
): StepExecutionState =>
  isActive && state.status === 'skipped' && state.executionId === null
    ? { ...state, status: 'pending' }
    : state

/**
 * Apply the server's view of the current run. Idempotent — safe to call on
 * every poll tick.
 */
const applyServerRun = (snapshot: ServerRunSnapshot): void => {
  if (store.getState().runId !== snapshot.runId) {
    setActiveRun(snapshot.runId, snapshot.workflowId)
  }

  if (snapshot.runId === null) {
    store.setState({ workflowId: snapshot.workflowId })
    return
  }

  const isActive = snapshot.status === 'running' || snapshot.status === 'pending'
  const local = store.getState().stepStates
  const stepStates: Record<string, StepExecutionState> = {}
  for (const step of snapshot.steps) {
    const server = mapRunStepToStepState(step)
    const existing = local[step.step_id]
    stepStates[step.step_id] = existing !== undefined && preferLocal(existing, server)
      ? existing
      : normalizeForActiveRun(server, isActive)
  }

  const completedStepCount = Object.values(stepStates).filter(
    (s) => s.status === 'success' || s.status === 'error' || s.status === 'skipped',
  ).length

  store.setState({
    runId: snapshot.runId,
    workflowId: snapshot.workflowId,
    isRunning: snapshot.status === 'running' || snapshot.status === 'pending',
    stepStates,
    eventLog: buildEventLog(snapshot.steps),
    totalSteps: snapshot.steps.length,
    completedStepCount,
    durationMs: snapshot.durationMs,
    error: snapshot.error,
    startedAt: snapshot.startedAt,
    completedAt: snapshot.completedAt,
    viewMode: 'live',
  })

  // Auto-expand finished steps so their output is visible without a click.
  for (const [stepId, state] of Object.entries(stepStates)) {
    if (state.status === 'success' || state.status === 'error') {
      sidebarStore.expandStep(stepId)
    }
  }
}

// ── Agent source hydration ─────────────────────────────────────────────────

/**
 * Populate `stepStreamStore.sources` from a run's workforce outputs.
 *
 * Only completed workforce steps carry `structured_output`; an agent still
 * streaming has no REST equivalent for its partial buffer.
 */
const hydrateAgentSources = (
  rosterByStep: Record<string, RosterAgent[]>,
  steps: readonly RunStepResult[],
): void => {
  const sources: Record<string, SourceStreamState> = {}

  for (const step of steps) {
    if (step.execution_mode !== 'workforce' || step.structured_output === null) continue
    const agents = step.structured_output.agents
    if (typeof agents !== 'object' || agents === null) continue

    const roster = rosterByStep[step.step_id]
    if (!roster) continue

    const agentRecord = agents as Record<string, unknown>
    const normalizedKeys = Collections.toLookupMap(
      Object.keys(agentRecord),
      (k) => k.toLowerCase().replace(/[\s_-]/g, ''),
      (k) => k,
    )

    for (const rosterAgent of roster) {
      const normalizedName = rosterAgent.name.toLowerCase().replace(/[\s_-]/g, '')
      const matchingKey = normalizedKeys.get(normalizedName)
      if (matchingKey === undefined) continue

      const output = agentRecord[matchingKey]
      const outputStr = typeof output === 'string' ? output : JSON.stringify(output, null, 2)

      sources[rosterAgent.id] = {
        sourceId: rosterAgent.id,
        sourceName: rosterAgent.name,
        stepId: step.step_id,
        status: 'completed',
        streamBuffer: outputStr,
        toolUses: [],
        error: null,
        startedAt: step.started_at ?? null,
        completedAt: step.completed_at ?? null,
      }

      sidebarStore.expandAgent(`${step.step_id}:${rosterAgent.id}`)
    }
  }

  if (Object.keys(sources).length > 0) {
    stepStreamStore.store.setState((s) => ({
      sources: { ...s.sources, ...sources },
    }))
  }
}

// ── Workshop fallback ───────────────────────────────────────────────────────

const mapWorkshopStepToStepState = (step: WorkshopStepSummary, runId: string): StepExecutionState => ({
  status: mapApiStatusToStoreStatus(step.status),
  stepName: null,
  agentId: null,
  executionId: runId,
  output: step.output !== null ? JSON.stringify(step.output) : null,
  error: step.error,
  inputTokens: null,
  outputTokens: null,
  durationMs: null,
  forEachProgress: null,
  startedAt: null,
  completedAt: null,
})

/**
 * Show workshop progress when the workflow has no full run to display.
 *
 * Only applies when `applyServerRun` left the overlay empty — a real run always
 * wins, which is what stops the workshop from clobbering live execution state.
 */
const applyWorkshopFallback = async (workflowId: string): Promise<void> => {
  if (store.getState().runId !== null) return

  try {
    const workshop = await api.workflows.getWorkshopStatus(workflowId)

    if (workshop.completed_steps.length === 0) return
    if (store.getState().runId !== null) return

    const stepStates: Record<string, StepExecutionState> = {}
    for (const step of workshop.completed_steps) {
      stepStates[step.step_id] = mapWorkshopStepToStepState(step, workshop.run_id)
    }

    const completedCount = Collections.filterMap(workshop.completed_steps, (s) =>
      s.status === 'completed' ? s.step_id : null,
    ).length

    store.setState({
      runId: workshop.run_id,
      workflowId: workshop.workflow_id,
      isRunning: false,
      stepStates,
      totalSteps: completedCount + workshop.next_executable_steps.length,
      completedStepCount: completedCount,
      viewMode: 'live',
    })
  } catch {
    // Workshop may not exist yet — no-op.
  }
}

export {
  setActiveRun,
  beginRun,
  applyServerRun,
  applyWorkshopFallback,
  hydrateAgentSources,
  preferLocal,
  normalizeForActiveRun,
  mapApiStatusToStoreStatus,
  mapRunStepToStepState,
  buildEventLog,
}
export type { ServerRunSnapshot }
